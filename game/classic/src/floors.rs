//! Floor tiles: prying them up with a crowbar, putting them back, and what they hide.
//! Spec: `docs/reference/floors.md`.

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, GridPos, Hidden, MapLayer, ObjectSpec, TileDef, TilePos};

use crate::actions::{ClickOutcome, in_reach};
use crate::hands::{Hands, Stack};

/// The floor tile prototype, and the tile id of a floor with its tiles in place.
pub const FLOOR_TILE: &str = "floor-tile";
const FLOOR: &str = "floor";
const PLATING: &str = "plating";

/// On a pipe or cable laid under the floor tiles (Luna `level = 1`): hidden while its
/// tile has them.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnderFloor;

/// Whether a placed object lies under the floor tiles.
pub fn lies_under_floor(spec: &ObjectSpec) -> bool {
    matches!(spec.id.as_str(), "cable" | "pipe" | "manifold" | "terminal") && !spec.above_floor
}

/// Bare plating: no floor tiles over it.
pub fn bare(map: &MapLayer, pos: TilePos) -> bool {
    map.0.tile(pos).is_some_and(|t| t.id == PLATING)
}

/// Floor tiles in place (Luna `intact`).
pub fn intact(map: &MapLayer, pos: TilePos) -> bool {
    map.0.tile(pos).is_some_and(|t| t.id == FLOOR)
}

/// Floors, and the piles of things laid on them: floor tiles, lengths of cable.
#[derive(SystemParam)]
pub struct FloorWork<'w, 's> {
    map: Query<'w, 's, &'static MapLayer>,
    pub(crate) stacks: Query<'w, 's, &'static mut Stack>,
    pub(crate) looks: Query<'w, 's, &'static Appearance>,
}

impl FloorWork<'_, '_> {
    pub fn sprite(&self, entity: Entity) -> Option<&str> {
        self.looks.get(entity).ok().map(|a| a.sprite.as_str())
    }

    pub fn bare(&self, pos: TilePos) -> bool {
        self.map.iter().next().is_some_and(|m| bare(m, pos))
    }

    pub fn intact(&self, pos: TilePos) -> bool {
        self.map.iter().next().is_some_and(|m| intact(m, pos))
    }

    /// Clicking a tile itself with a crowbar or floor tiles (Luna `floor/attackby`).
    /// `None` when these rules don't apply.
    pub fn use_on_tile(
        &mut self,
        mob_pos: TilePos,
        hands: &mut Hands,
        tile: TilePos,
        commands: &mut Commands,
    ) -> Option<ClickOutcome> {
        let active = hands.active_item()?;
        if !in_reach(mob_pos, tile) {
            return None;
        }
        match self.sprite(active)? {
            "crowbar" if self.intact(tile) => {
                commands.queue(move |world: &mut World| {
                    if set_floor(world, tile, FLOOR, PLATING) {
                        let _ = crate::objects::spawn_stack(world, FLOOR_TILE, tile, 1);
                    }
                });
                Some(ClickOutcome::default())
            }
            FLOOR_TILE if self.bare(tile) && self.stacks.contains(active) => {
                commands.queue(move |world: &mut World| {
                    set_floor(world, tile, PLATING, FLOOR);
                });
                self.use_up(active, 1, hands, commands);
                Some(ClickOutcome::default())
            }
            _ => None,
        }
    }

    /// Floor tiles on another pile of floor tiles: the pile in hand takes all it can, or
    /// a single tile from the other hand (Luna `stack/attackby`). The caller checks reach.
    pub fn use_on(
        &mut self,
        hands: &mut Hands,
        target: Entity,
        commands: &mut Commands,
    ) -> Option<ClickOutcome> {
        let active = hands.active_item()?;
        if active == target || self.sprite(active)? != FLOOR_TILE || self.sprite(target)? != FLOOR_TILE {
            return None;
        }
        let (Ok(mine), Ok(theirs)) = (self.stacks.get(active), self.stacks.get(target)) else {
            return None;
        };
        let (mine, theirs) = (*mine, *theirs);
        let room = mine.max.saturating_sub(mine.amount);
        let moved = if hands.get(hands.active.other()) == Some(target) {
            1.min(room)
        } else {
            theirs.amount.min(room)
        };
        if moved > 0 {
            self.use_up(target, moved, hands, commands);
            if let Ok(mut stack) = self.stacks.get_mut(active) {
                stack.amount += moved;
            }
        }
        Some(ClickOutcome::default())
    }

    /// What examining a pile of floor tiles adds (Luna `stack/examine`).
    pub fn examine(&self, target: Entity) -> Option<LocMsg> {
        if self.sprite(target)? != FLOOR_TILE {
            return None;
        }
        let amount = self.stacks.get(target).ok()?.amount;
        Some(LocMsg::new("examine-floor-tiles").num("count", i64::from(amount)))
    }

    /// Take some off a stack; an empty stack is gone, out of the hand holding it.
    pub fn use_up(&mut self, item: Entity, amount: u32, hands: &mut Hands, commands: &mut Commands) {
        let Ok(mut stack) = self.stacks.get_mut(item) else {
            return;
        };
        stack.amount = stack.amount.saturating_sub(amount);
        if stack.amount == 0 {
            for hand in [hands.active, hands.active.other()] {
                if hands.get(hand) == Some(item) {
                    *hands.slot_mut(hand) = None;
                }
            }
            commands.entity(item).despawn();
        }
    }
}

/// Turn the tile at `pos` from `from` into `to`. False if it no longer is `from` (someone
/// else got there first this tick).
fn set_floor(world: &mut World, pos: TilePos, from: &str, to: &str) -> bool {
    let mut maps = world.query::<&mut MapLayer>();
    let Some(mut map) = maps.iter_mut(world).next() else {
        return false;
    };
    if map.0.tile(pos).is_none_or(|t| t.id != from) {
        return false;
    }
    map.0.set_tile(
        pos,
        TileDef {
            id: to.to_owned(),
            solid: false,
            opaque: None,
        },
    )
}

/// Hide what lies under floor tiles, and show it once they are gone (Luna `levelupdate`).
pub fn hide_under_floors(
    map: Query<&MapLayer>,
    things: Query<(Entity, &GridPos, Has<Hidden>), With<UnderFloor>>,
    mut commands: Commands,
) {
    let Some(map) = map.iter().next() else { return };
    for (entity, pos, hidden) in &things {
        match (intact(map, pos.0), hidden) {
            (true, false) => {
                commands.entity(entity).insert(Hidden);
            }
            (false, true) => {
                commands.entity(entity).remove::<Hidden>();
            }
            _ => {}
        }
    }
}
