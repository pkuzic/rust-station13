//! Fire extinguishers, the wall cabinets that hold them, and water tanks to refill them
//! from. Spec: `docs/reference/extinguishers.md`.

use std::collections::HashSet;

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, Dense, Dir, GridPos, NetId, ObjectSpec, TilePos};

use crate::actions::ClickOutcome;
use crate::atmos::StationAtmos;
use crate::hands::{Hands, HeldBy};
use crate::rng::GameRng;
use crate::sound::SoundEvents;
use crate::time::WorldTime;

/// The fire extinguisher prototype.
pub const EXTINGUISHER: &str = "fire-extinguisher";
/// The light, compact one in emergency toolboxes (Luna `extinguisher/mini`).
pub const MINI_EXTINGUISHER: &str = "fire-extinguisher-mini";
/// The wall cabinet that holds one.
pub const CABINET: &str = "extinguisher-cabinet";
/// The water tank to refill them from.
pub const WATER_TANK: &str = "water-tank";
/// The sprite of a puff of water in flight.
pub const PUFF: &str = "water-puff";
/// Water a fire extinguisher holds, units (Luna `max_water`), and a mini one.
pub const MAX_WATER: u32 = 50;
pub const MINI_WATER: u32 = 30;
/// Water in a new water tank, units (Luna's reagent dispensers).
pub const TANK_WATER: u32 = 1000;
/// Time between sprays, ds (Luna `last_use + 20`).
pub const COOLDOWN_DS: f32 = 20.0;
/// Puffs in one spray, a unit of water each.
pub const PUFFS: u32 = 5;
/// Steps a puff takes at most, and the time between them, ds (Luna `sleep(2)`).
pub const PUFF_STEPS: u8 = 5;
pub const PUFF_STEP_DS: f32 = 2.0;
/// A puff is gone this long after it was sprayed, ds (Luna `spawn(70)`).
pub const PUFF_LIFE_DS: f32 = 70.0;

/// A fire extinguisher: its water and all it holds, whether the safety is on, and when
/// it last sprayed.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Extinguisher {
    pub water: u32,
    pub max: u32,
    pub safety: bool,
    pub last_use: Option<f32>,
}

/// The extinguisher a new item of prototype `proto` is, full and with the safety on.
pub fn kind(proto: &str) -> Option<Extinguisher> {
    let max = match proto {
        EXTINGUISHER => MAX_WATER,
        MINI_EXTINGUISHER => MINI_WATER,
        _ => return None,
    };
    Some(Extinguisher {
        water: max,
        max,
        safety: true,
        last_use: None,
    })
}

/// A tank of water to refill fire extinguishers from.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaterTank {
    pub water: u32,
}

/// A wall cabinet: the extinguisher in it, and whether its door is open.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cabinet {
    pub held: Option<Entity>,
    pub opened: bool,
}

/// On an extinguisher kept in a cabinet (it is off the map).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InCabinet(pub Entity);

/// A puff of water flying toward `target`.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Puff {
    pub target: TilePos,
    pub steps: u8,
    pub next: f32,
    pub gone_at: f32,
}

/// Sprays asked for this tick: from where, toward where.
#[derive(Resource, Debug, Default)]
pub struct PendingSprays(pub Vec<(TilePos, TilePos)>);

/// Spawn a cabinet with its extinguisher, or a water tank.
pub fn spawn_fixture(
    world: &mut World,
    spec: &ObjectSpec,
    pos: TilePos,
    id: NetId,
) -> Result<Entity, String> {
    if spec.id == WATER_TANK {
        return Ok(world
            .spawn((
                GridPos(pos),
                id,
                Appearance::new(WATER_TANK, ""),
                WaterTank { water: TANK_WATER },
                Dense,
            ))
            .id());
    }
    let cabinet = world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(CABINET, cabinet_look(false, None)),
        ))
        .id();
    let held = crate::objects::spawn_object(world, &ObjectSpec::new(EXTINGUISHER), pos)?;
    world
        .entity_mut(held)
        .remove::<GridPos>()
        .insert(InCabinet(cabinet));
    world.entity_mut(cabinet).insert(Cabinet {
        held: Some(held),
        opened: false,
    });
    Ok(cabinet)
}

/// Luna `extinguisher_cabinet/update_icon`; `held` is whether it holds a mini one, if
/// it holds one.
fn cabinet_look(opened: bool, held: Option<bool>) -> &'static str {
    match (opened, held) {
        (false, _) => "extinguisher_closed",
        (true, Some(false)) => "extinguisher_full",
        (true, Some(true)) => "extinguisher_mini",
        (true, None) => "extinguisher_empty",
    }
}

/// Everything extinguishers, cabinets and water tanks need from a click.
#[derive(SystemParam)]
pub struct ExtinguisherWork<'w, 's> {
    extinguishers: Query<'w, 's, &'static mut Extinguisher>,
    tanks: Query<'w, 's, &'static mut WaterTank>,
    cabinets: Query<'w, 's, &'static mut Cabinet>,
    sprays: ResMut<'w, PendingSprays>,
}

impl ExtinguisherWork<'_, '_> {
    pub fn is_extinguisher(&self, item: Entity) -> bool {
        self.extinguishers.contains(item)
    }

    pub fn is_cabinet(&self, target: Entity) -> bool {
        self.cabinets.contains(target)
    }

    /// Whether a click with `item` goes to the extinguisher rather than the usual rules:
    /// it is one, and its safety is off or `near_tank` (a water tank in reach is clicked).
    pub fn takes_click(&self, item: Option<Entity>, target: Option<Entity>, near_tank: bool) -> bool {
        let Some(ext) = item.and_then(|i| self.extinguishers.get(i).ok()) else {
            return false;
        };
        let tank = near_tank && target.is_some_and(|t| self.tanks.contains(t));
        tank || !ext.safety
    }

    /// Clicking the extinguisher in the active hand flips its safety (Luna `attack_self`).
    pub fn use_self(&mut self, item: Entity) -> Option<ClickOutcome> {
        let mut ext = self.extinguishers.get_mut(item).ok()?;
        ext.safety = !ext.safety;
        Some(ClickOutcome::told(if ext.safety {
            "extinguisher-safety-on"
        } else {
            "extinguisher-safety-off"
        }))
    }

    /// Luna `extinguisher/afterattack`: on a water tank in reach it is refilled; with the
    /// safety off it sprays toward `at` from `from`, unless it is empty or sprayed less
    /// than two seconds ago.
    pub fn use_on(
        &mut self,
        item: Entity,
        target: Option<Entity>,
        near: bool,
        from: TilePos,
        at: TilePos,
        now: f32,
    ) -> Option<ClickOutcome> {
        let mut ext = self.extinguishers.get_mut(item).ok()?;
        if near && let Some(mut tank) = target.and_then(|t| self.tanks.get_mut(t).ok()) {
            let moved = (ext.max - ext.water.min(ext.max)).min(tank.water);
            tank.water -= moved;
            ext.water += moved;
            return Some(ClickOutcome::told("extinguisher-refilled"));
        }
        if ext.safety {
            return None;
        }
        if ext.water == 0 {
            return Some(ClickOutcome::told("extinguisher-empty"));
        }
        if ext.last_use.is_some_and(|t| now < t + COOLDOWN_DS) {
            return Some(ClickOutcome::default());
        }
        ext.last_use = Some(now);
        ext.water = ext.water.saturating_sub(PUFFS);
        self.sprays.0.push((from, at));
        Some(ClickOutcome::default())
    }

    /// Luna `extinguisher_cabinet/attack_hand` and `attackby`: an empty hand takes the
    /// extinguisher out (opening the door) or works the door; an extinguisher goes in if
    /// the door is open and the cabinet empty, and otherwise works the door like anything
    /// else in the hand. The caller checks reach.
    pub fn use_cabinet(
        &mut self,
        mob: Entity,
        target: Entity,
        hands: &mut Hands,
        commands: &mut Commands,
    ) -> Option<ClickOutcome> {
        let mut cabinet = self.cabinets.get_mut(target).ok()?;
        let held = hands.active_item();
        let mut outcome = ClickOutcome::default();
        match held {
            None if cabinet.held.is_some() => {
                let ext = cabinet.held.take()?;
                commands.entity(ext).remove::<InCabinet>().insert(HeldBy(mob));
                *hands.slot_mut(hands.active) = Some(ext);
                cabinet.opened = true;
                outcome = took_or_placed("extinguisher-taken");
            }
            Some(item) if self.extinguishers.contains(item) && cabinet.opened && cabinet.held.is_none() => {
                *hands.slot_mut(hands.active) = None;
                commands.entity(item).remove::<HeldBy>().insert(InCabinet(target));
                cabinet.held = Some(item);
                outcome = took_or_placed("extinguisher-placed");
            }
            _ => cabinet.opened = !cabinet.opened,
        }
        Some(outcome)
    }

    /// What examining an extinguisher or a water tank adds.
    pub fn examine(&self, target: Entity) -> Option<LocMsg> {
        if let Ok(ext) = self.extinguishers.get(target) {
            return Some(LocMsg::new("examine-extinguisher").num("count", i64::from(ext.water)));
        }
        let tank = self.tanks.get(target).ok()?;
        Some(LocMsg::new("examine-water-tank").num("count", i64::from(tank.water)))
    }
}

fn took_or_placed(id: &str) -> ClickOutcome {
    ClickOutcome {
        to_mob: Some(LocMsg::new(id).arg("item", EXTINGUISHER).arg("cabinet", CABINET)),
        seen: None,
    }
}

/// The way from `a` toward `b`, diagonals included (BYOND `get_dir`).
fn toward(a: TilePos, b: TilePos) -> Option<Dir> {
    Dir::from_offset((b.x - a.x).signum(), (b.y - a.y).signum())
}

/// A quarter turn counter-clockwise (`clockwise` false) or clockwise of `dir` (BYOND
/// `turn(dir, ±90)`), as an offset.
fn quarter(dir: Dir, clockwise: bool) -> (i32, i32) {
    let (x, y) = dir.offset();
    if clockwise { (y, -x) } else { (-y, x) }
}

/// Spray the puffs asked for, move the flying ones and take away the spent ones. Each puff
/// heads for the clicked tile or one of the two beside it across the spray, a step every
/// 0.2 s, five steps at most, stopping at walls, closed doors and sealed edges, and puts
/// out any fire on every tile it reaches.
#[allow(clippy::too_many_arguments)]
pub fn run_puffs(
    time: Res<WorldTime>,
    mut commands: Commands,
    mut sprays: ResMut<PendingSprays>,
    mut rng: ResMut<GameRng>,
    mut ids: ResMut<crate::objects::NetIds>,
    mut sounds: ResMut<SoundEvents>,
    atmos: Option<ResMut<StationAtmos>>,
    decks: crate::decks::Decks,
    (doors, firedoors, panes): (
        Query<
            &GridPos,
            (
                With<Dense>,
                Or<(With<crate::doors::Airlock>, With<crate::blastdoors::BlastDoor>)>,
            ),
        >,
        Query<(&GridPos, &crate::firedoors::Firedoor), Without<Puff>>,
        Res<crate::windows::PaneEdges>,
    ),
    mut puffs: Query<(Entity, &mut Puff, &mut GridPos), Without<Dense>>,
) {
    let now = time.deciseconds();
    for (from, at) in std::mem::take(&mut sprays.0) {
        sounds.play(crate::sound::EXTINGUISH, from, 75.0);
        let mut targets = vec![at];
        if let Some(dir) = toward(from, at) {
            for clockwise in [false, true] {
                let (dx, dy) = quarter(dir, clockwise);
                targets.push(TilePos::new(at.x + dx, at.y + dy, at.z));
            }
        }
        for _ in 0..PUFFS {
            let target = targets[rng.range(0, targets.len() as i32 - 1) as usize];
            commands.spawn((
                GridPos(from),
                ids.alloc(),
                Appearance::new(PUFF, ""),
                Puff {
                    target,
                    steps: PUFF_STEPS,
                    next: now,
                    gone_at: now + PUFF_LIFE_DS,
                },
            ));
        }
    }
    // Puffs sprayed now take their first step next tick (spawns apply after this system).
    let closed: HashSet<TilePos> = doors.iter().map(|p| p.0).collect();
    let mut sealed = crate::firedoors::closed_edges(firedoors.iter());
    sealed.extend(panes.0.iter().copied());
    let mut wet: Vec<TilePos> = Vec::new();
    for (entity, mut puff, mut pos) in &mut puffs {
        if now >= puff.gone_at {
            commands.entity(entity).despawn();
            continue;
        }
        if puff.steps == 0 || now < puff.next {
            continue;
        }
        puff.steps -= 1;
        puff.next = now + PUFF_STEP_DS;
        if let Some(dir) = toward(pos.0, puff.target)
            && let Some(map) = decks.deck(pos.0.z)
            && let Some(to) = crate::movement::resolve_step(map, &closed, &sealed, pos.0, dir)
        {
            pos.0 = to;
        }
        wet.push(pos.0);
        if pos.0 == puff.target {
            puff.steps = 0;
        }
    }
    if let Some(mut atmos) = atmos {
        for at in wet {
            atmos.0.douse(at);
        }
    }
}

/// A cabinet shows its door and what it holds.
pub fn update_cabinet_looks(
    mut cabinets: Query<(&Cabinet, &mut Appearance), Changed<Cabinet>>,
    extinguishers: Query<&Extinguisher>,
) {
    for (cabinet, mut look) in &mut cabinets {
        let mini = cabinet
            .held
            .map(|e| extinguishers.get(e).is_ok_and(|x| x.max == MINI_WATER));
        let state = cabinet_look(cabinet.opened, mini);
        if look.state != state {
            look.state = state.to_owned();
        }
    }
}

/// An extinguisher with its safety off shows it.
pub fn update_extinguisher_looks(mut items: Query<(&Extinguisher, &mut Appearance), Changed<Extinguisher>>) {
    for (ext, mut look) in &mut items {
        let state = if ext.safety { "" } else { "on" };
        if look.state != state {
            look.state = state.to_owned();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_side_targets_lie_across_the_spray() {
        assert_eq!(quarter(Dir::North, false), (-1, 0));
        assert_eq!(quarter(Dir::North, true), (1, 0));
        assert_eq!(quarter(Dir::East, false), (0, 1));
        assert_eq!(quarter(Dir::NorthEast, false), (-1, 1));
    }

    #[test]
    fn cabinet_sprite_follows_door_and_contents() {
        assert_eq!(cabinet_look(false, Some(false)), "extinguisher_closed");
        assert_eq!(cabinet_look(true, Some(false)), "extinguisher_full");
        assert_eq!(cabinet_look(true, Some(true)), "extinguisher_mini");
        assert_eq!(cabinet_look(true, None), "extinguisher_empty");
    }
}
