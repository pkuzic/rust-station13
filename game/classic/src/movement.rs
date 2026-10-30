//! Mob movement on the tile grid.
//!
//! Spec: `docs/reference/movement.md` (luna:code/modules/mob/mob.dm `/client/Move`,
//! luna:code/WorkInProgress/NewMobs and etc/Damagesystem.dm `human/movement_delay`).

use std::collections::HashSet;

use bevy_ecs::prelude::*;
use rs13_world::{Dense, Dir, GridPos, MapLayer, MobStat, TileMap, TilePos};

use crate::access::IdCard;
use crate::atmos::{has_handhold, is_space};
use crate::doors::{Airlock, BUMP_COOLDOWN_DS, DoorState};
use crate::hands::Hands;
use crate::health::Vitals;
use crate::objects::PlayerControlled;
use crate::time::WorldTime;

/// Extra delay for the run intent, in deciseconds.
pub const RUN_DELAY_DS: f32 = 1.0;
/// Extra delay for the walk intent, in deciseconds.
pub const WALK_DELAY_DS: f32 = 4.0;
/// A healthy, unencumbered human's `movement_delay()`, in deciseconds.
pub const HUMAN_BASE_DELAY_DS: f32 = 1.3;

/// A mob: anything that moves by itself or by a player's control.
#[derive(Component, Debug, Default)]
pub struct Mob {
    /// Earliest world time (deciseconds) at which the next step may start.
    pub next_move_ds: f32,
    /// When this mob last bumped into something, for the bump cooldown.
    pub last_bump_ds: Option<f32>,
}

/// What the controller of a mob is asking for this tick. Set from player input or AI.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveIntent {
    pub dir: Option<Dir>,
    pub run: bool,
}

impl Default for MoveIntent {
    fn default() -> Self {
        Self { dir: None, run: true }
    }
}

/// Delay after a successful step, in deciseconds.
pub fn step_delay_ds(run: bool) -> f32 {
    let intent = if run { RUN_DELAY_DS } else { WALK_DELAY_DS };
    intent + HUMAN_BASE_DELAY_DS
}

/// Where a step from `from` in `dir` ends up, or `None` if it is blocked.
///
/// A blocked diagonal slides along its horizontal component, then its vertical one.
pub fn resolve_step(map: &TileMap, occupied: &HashSet<TilePos>, from: TilePos, dir: Dir) -> Option<TilePos> {
    let free = |pos: TilePos| !map.is_solid(pos) && !occupied.contains(&pos);
    let target = from.step(dir);
    if free(target) {
        return Some(target);
    }
    let (horizontal, vertical) = dir.cardinals()?;
    [horizontal, vertical]
        .into_iter()
        .map(|d| from.step(d))
        .find(|p| free(*p))
}

/// Moves every mob whose controller asks to move and whose delay has expired.
/// A player-controlled mob that walks into a closed airlock opens it.
/// Mobs are processed in entity order so results are deterministic.
pub fn move_mobs(
    time: Res<WorldTime>,
    map: Single<&MapLayer>,
    mut mobs: Query<(
        Entity,
        &mut GridPos,
        &mut Mob,
        &MoveIntent,
        Has<PlayerControlled>,
        Option<&Hands>,
        Option<&Vitals>,
        Option<&crate::equipment::Equipment>,
    )>,
    mut objects: Query<(Entity, &GridPos, Has<Dense>, Option<&mut Airlock>), Without<Mob>>,
    cards: Query<&IdCard>,
    wearables: Query<&crate::equipment::Wearable>,
    electrified: Query<&crate::wires::Wiring>,
    mut shocks: ResMut<crate::wires::PendingShocks>,
) {
    let now = time.deciseconds();
    let mut occupied: HashSet<TilePos> = mobs.iter().map(|(_, pos, ..)| pos.0).collect();
    occupied.extend(
        objects
            .iter()
            .filter(|(_, _, dense, _)| *dense)
            .map(|(_, pos, ..)| pos.0),
    );

    let mut order: Vec<Entity> = mobs.iter().map(|(e, ..)| e).collect();
    order.sort();
    for entity in order {
        let Ok((_, mut pos, mut mob, intent, is_player, hands, vitals, equipment)) = mobs.get_mut(entity)
        else {
            continue;
        };
        let Some(dir) = intent.dir else { continue };
        if vitals.is_some_and(|v| v.stat != MobStat::Conscious) {
            continue;
        }
        // In open space there is nothing to walk on: only push off from something nearby.
        if is_space(&map.0, pos.0) && !has_handhold(&map.0, pos.0) {
            continue;
        }
        if now < mob.next_move_ds {
            continue;
        }
        if let Some(target) = resolve_step(&map.0, &occupied, pos.0, dir) {
            occupied.remove(&pos.0);
            occupied.insert(target);
            pos.0 = target;
            let worn = equipment.map_or(0.0, |e| crate::equipment::slowdown(e, &wearables));
            mob.next_move_ds = now + step_delay_ds(intent.run) + worn;
            continue;
        }

        // Blocked: bump whatever is in the way.
        let cooled_down = mob.last_bump_ds.is_none_or(|t| now - t > BUMP_COOLDOWN_DS);
        if !is_player || !cooled_down {
            continue;
        }
        let ahead = pos.0.step(dir);
        for (obj_entity, obj_pos, _, airlock) in &mut objects {
            if obj_pos.0 != ahead {
                continue;
            }
            // Luna `Bumped`: ignored while the panel is open.
            if let Some(mut airlock) = airlock
                && airlock.state == DoorState::Closed
                && !airlock.panel_open
            {
                mob.last_bump_ds = Some(now);
                if electrified.get(obj_entity).is_ok_and(|w| w.is_electrified()) && airlock.powered {
                    shocks.0.push((entity, crate::wires::HAND_SHOCK));
                    continue;
                }
                let card = hands
                    .and_then(|h| h.active_item())
                    .and_then(|e| cards.get(e).ok());
                airlock.use_by(now, card);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room() -> TileMap {
        TileMap::from_ron(
            r#"(
                legend: { 'W': (tile: (id: "wall", solid: true)), '.': (tile: (id: "floor")) },
                rows: ["WWWWW", "W...W", "W...W", "WWWWW"],
            )"#,
            0,
        )
        .unwrap()
    }

    #[test]
    fn step_delays_match_luna() {
        assert!((step_delay_ds(true) - 2.3).abs() < 1e-6);
        assert!((step_delay_ds(false) - 5.3).abs() < 1e-6);
    }

    #[test]
    fn walls_and_mobs_block() {
        let map = room();
        let from = TilePos::new(1, 1, 0);
        assert_eq!(resolve_step(&map, &HashSet::new(), from, Dir::West), None);
        let occupied = HashSet::from([TilePos::new(2, 1, 0)]);
        assert_eq!(resolve_step(&map, &occupied, from, Dir::East), None);
    }

    #[test]
    fn blocked_diagonal_slides_along_a_wall() {
        let map = room();
        // From the top-left floor tile, north-east hits the top wall: slide east.
        let from = TilePos::new(1, 2, 0);
        assert_eq!(
            resolve_step(&map, &HashSet::new(), from, Dir::NorthEast),
            Some(TilePos::new(2, 2, 0))
        );
    }
}
