//! Laying and cutting power cable, and cable coils. Spec: `docs/reference/cables.md`.

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, Dir, GridPos, ObjectSpec, TilePos};

use crate::actions::{ClickOutcome, in_reach};
use crate::floors::FloorWork;
use crate::hands::{Hands, HeldBy, Stack};
use crate::power::{Cable, PowerNode};

/// The cable coil prototype.
pub const COIL: &str = "cable-coil";

/// The direction from `from` to the neighbouring tile `to`, or `None` on the same tile.
fn toward(from: TilePos, to: TilePos) -> Option<Dir> {
    Dir::from_offset(to.x - from.x, to.y - from.y)
}

/// Everything clicking with a cable coil or wirecutters needs.
#[derive(SystemParam)]
pub struct CableWork<'w, 's> {
    /// Cable is only laid or cut on bare plating: under floor tiles it is out of reach.
    pub floors: FloorWork<'w, 's>,
    cables: Query<'w, 's, (Entity, &'static GridPos, &'static mut Cable)>,
    machines: Query<'w, 's, &'static GridPos, (With<PowerNode>, Without<Cable>)>,
    held: Query<'w, 's, &'static HeldBy>,
    places: Query<'w, 's, &'static GridPos, Without<Cable>>,
}

impl CableWork<'_, '_> {
    fn sprite(&self, entity: Entity) -> Option<&str> {
        self.floors.sprite(entity)
    }

    fn is_coil(&self, entity: Entity) -> bool {
        self.sprite(entity) == Some(COIL) && self.floors.stacks.contains(entity)
    }

    fn bare(&self, pos: TilePos) -> bool {
        self.floors.bare(pos)
    }

    /// Whether a cable on `pos` already has this end.
    fn cable_with_end(&self, pos: TilePos, end: Option<Dir>) -> bool {
        self.cables.iter().any(|(_, p, c)| p.0 == pos && c.has_end(end))
    }

    /// Clicking `target` with the active hand, when that is a cable coil or wirecutters
    /// and the target is a cable, a coil or a power machine. `None` when these rules don't
    /// apply and the ordinary click rules do.
    pub fn use_on(
        &mut self,
        mob: Entity,
        mob_pos: TilePos,
        hands: &mut Hands,
        target: Entity,
        commands: &mut Commands,
    ) -> Option<ClickOutcome> {
        let active = hands.active_item()?;
        if active == target {
            return None;
        }
        // Where the target is, if the mob can reach it: on the floor nearby or in its hands.
        let at = match (self.cables.get(target), self.places.get(target)) {
            (Ok((_, p, _)), _) | (_, Ok(p)) => Some(p.0).filter(|p| in_reach(mob_pos, *p)),
            _ if self.held.get(target).is_ok_and(|h| h.0 == mob) => Some(mob_pos),
            _ => None,
        };
        match self.sprite(active) {
            Some(COIL) if self.is_coil(target) => {
                at?;
                Some(self.join_coils(active, target, hands, commands))
            }
            Some(COIL) if self.cables.contains(target) => {
                Some(self.extend_cable(mob_pos, active, target, hands, commands))
            }
            Some(COIL) if self.machines.contains(target) => {
                // Luna `afterattack`: a knot under the machine, pointing at the mob.
                let at = at?;
                let end = toward(at, mob_pos)?;
                Some(self.lay(at, end, active, hands, commands))
            }
            Some("wirecutters") if self.is_coil(target) => {
                at?;
                Some(self.cut_piece(mob_pos, target, commands))
            }
            Some("wirecutters") if self.cables.contains(target) => {
                let at = at?;
                Some(self.cut_cable(at, target, commands))
            }
            _ => None,
        }
    }

    /// Clicking a tile itself with a cable coil: lay a cable from the middle of the tile
    /// toward the mob, or, on the mob's own tile, the way it faces (Luna `LayOnTurf`).
    pub fn use_on_tile(
        &mut self,
        (mob_pos, facing): (TilePos, Option<Dir>),
        hands: &mut Hands,
        tile: TilePos,
        commands: &mut Commands,
    ) -> Option<ClickOutcome> {
        let active = hands.active_item()?;
        if !self.is_coil(active) {
            return None;
        }
        if !in_reach(mob_pos, tile) {
            return Some(ClickOutcome::told("cable-too-far"));
        }
        if !self.bare(tile) {
            return Some(if self.floors.intact(tile) {
                ClickOutcome::told("cable-floor-intact")
            } else {
                ClickOutcome::default()
            });
        }
        let end = toward(tile, mob_pos).unwrap_or(facing.unwrap_or(Dir::South));
        Some(self.lay(tile, end, active, hands, commands))
    }

    /// A new cable on `at` from a knot toward `end`, using up one length of the coil.
    fn lay(
        &mut self,
        at: TilePos,
        end: Dir,
        coil: Entity,
        hands: &mut Hands,
        commands: &mut Commands,
    ) -> ClickOutcome {
        if !self.bare(at) {
            return ClickOutcome::default();
        }
        if self.cable_with_end(at, Some(end)) {
            return ClickOutcome::told("cable-exists");
        }
        spawn_cable(commands, at, vec![end]);
        self.floors.use_up(coil, 1, hands, commands);
        ClickOutcome::default()
    }

    /// Luna `JoinCable`: a cable that points at the mob is continued onto the mob's tile;
    /// a knot is turned to point at the mob as well.
    fn extend_cable(
        &mut self,
        mob_pos: TilePos,
        coil: Entity,
        target: Entity,
        hands: &mut Hands,
        commands: &mut Commands,
    ) -> ClickOutcome {
        let Ok((_, at, cable)) = self.cables.get(target) else {
            return ClickOutcome::default();
        };
        let (at, cable) = (at.0, *cable);
        if !self.bare(at) || at == mob_pos {
            return ClickOutcome::default();
        }
        if !in_reach(mob_pos, at) {
            return ClickOutcome::told("cable-too-far");
        }
        let Some(to_mob) = toward(at, mob_pos) else {
            return ClickOutcome::default();
        };
        if cable.has_end(Some(to_mob)) {
            // Luna places this piece on the cable's own tile, pointing away from the mob;
            // it belongs on the mob's tile, pointing back at the cable.
            if !self.bare(mob_pos) {
                return ClickOutcome::told("cable-floor-intact");
            }
            return self.lay(mob_pos, to_mob.opposite(), coil, hands, commands);
        }
        if cable.a.is_some() {
            return ClickOutcome::default();
        }
        let Some(turned) = Cable::new(Some(cable.b), Some(to_mob)) else {
            return ClickOutcome::default();
        };
        if self
            .cables
            .iter()
            .any(|(e, p, c)| e != target && p.0 == at && *c == turned)
        {
            return ClickOutcome::told("cable-exists");
        }
        if let Ok((_, _, mut cable)) = self.cables.get_mut(target) {
            *cable = turned;
        }
        self.floors.use_up(coil, 1, hands, commands);
        ClickOutcome::default()
    }

    /// Wirecutters on a cable: it comes apart into a short coil on its tile.
    fn cut_cable(&mut self, at: TilePos, target: Entity, commands: &mut Commands) -> ClickOutcome {
        if !self.bare(at) {
            return ClickOutcome::default();
        }
        let Ok((_, _, cable)) = self.cables.get(target) else {
            return ClickOutcome::default();
        };
        let lengths = cable.lengths();
        commands.entity(target).despawn();
        commands.queue(move |world: &mut World| spawn_coil(world, at, lengths));
        ClickOutcome {
            to_mob: None,
            seen: Some(LocMsg::new("cable-cut").arg("item", "cable")),
        }
    }

    /// Wirecutters on a coil of more than two lengths cut a piece of one off, onto the
    /// mob's tile.
    fn cut_piece(&mut self, mob_pos: TilePos, coil: Entity, commands: &mut Commands) -> ClickOutcome {
        let Ok(mut stack) = self.floors.stacks.get_mut(coil) else {
            return ClickOutcome::default();
        };
        if stack.amount <= 2 {
            return ClickOutcome::default();
        }
        stack.amount -= 1;
        commands.queue(move |world: &mut World| spawn_coil(world, mob_pos, 1));
        ClickOutcome::told("coil-cut-piece")
    }

    /// One coil on another: the one in hand takes as much of the other as it can hold
    /// (Luna `cable_coil/attackby`).
    fn join_coils(
        &mut self,
        held: Entity,
        other: Entity,
        hands: &mut Hands,
        commands: &mut Commands,
    ) -> ClickOutcome {
        let (Ok(mine), Ok(theirs)) = (self.floors.stacks.get(held), self.floors.stacks.get(other)) else {
            return ClickOutcome::default();
        };
        let (mine, theirs) = (*mine, *theirs);
        if mine.amount >= mine.max {
            return ClickOutcome::told("coil-full");
        }
        let room = mine.max - mine.amount;
        let outcome = if theirs.amount <= room {
            self.floors.use_up(other, theirs.amount, hands, commands);
            ClickOutcome::told("coil-joined")
        } else {
            self.floors.use_up(other, room, hands, commands);
            ClickOutcome {
                to_mob: Some(LocMsg::new("coil-transferred").num("count", i64::from(room))),
                seen: None,
            }
        };
        if let Ok(mut stack) = self.floors.stacks.get_mut(held) {
            stack.amount += theirs.amount.min(room);
        }
        outcome
    }

    /// What examining a coil adds (Luna `cable_coil/examine`).
    pub fn examine(&self, target: Entity) -> Option<LocMsg> {
        if !self.is_coil(target) {
            return None;
        }
        let amount = self.floors.stacks.get(target).ok()?.amount;
        Some(match amount {
            1 => LocMsg::new("examine-coil-short"),
            2 => LocMsg::new("examine-coil-piece"),
            n => LocMsg::new("examine-coil").num("count", i64::from(n)),
        })
    }
}

fn spawn_cable(commands: &mut Commands, at: TilePos, ends: Vec<Dir>) {
    commands.queue(move |world: &mut World| {
        let spec = ObjectSpec {
            ends,
            ..ObjectSpec::new("cable")
        };
        // Cable is a built-in prototype: this cannot fail.
        let _ = crate::objects::spawn_object(world, &spec, at);
    });
}

/// A coil of `lengths` on the floor at `at`.
fn spawn_coil(world: &mut World, at: TilePos, lengths: u32) {
    let _ = crate::objects::spawn_stack(world, COIL, at, lengths);
}

/// Cable sprites follow their ends; coils show one, two or three loops by length.
pub fn update_cable_looks(
    mut cables: Query<(&Cable, &mut Appearance), Changed<Cable>>,
    mut coils: Query<(&Stack, &mut Appearance), (Changed<Stack>, Without<Cable>)>,
) {
    for (cable, mut appearance) in &mut cables {
        let state = cable.state();
        if appearance.state != state {
            appearance.state = state;
        }
    }
    for (stack, mut appearance) in &mut coils {
        if appearance.sprite != COIL {
            continue;
        }
        let state = match stack.amount {
            1 => "1",
            2 => "2",
            _ => "",
        };
        if appearance.state != state {
            appearance.state = state.to_owned();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cables_keep_luna_order_and_states() {
        let bend = Cable::new(Some(Dir::East), Some(Dir::North)).unwrap();
        assert_eq!(bend.state(), "1-4");
        assert_eq!(bend.lengths(), 2);
        let knot = Cable::new(Some(Dir::West), None).unwrap();
        assert_eq!(knot.state(), "0-8");
        assert_eq!(knot.lengths(), 1);
        assert!(knot.has_end(None));
        assert_eq!(Cable::new(None, None), None);
        assert_eq!(Cable::new(Some(Dir::East), Some(Dir::East)), None);
        assert_eq!(Cable::from_ends(&[]).unwrap().state(), "0-1");
        assert_eq!(
            Cable::from_ends(&[Dir::SouthWest, Dir::North]).unwrap().state(),
            "1-10"
        );
    }
}
