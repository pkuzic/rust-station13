//! The station's air: the zone solver hooked up to doors, plus space wind.
//! Design: `docs/ATMOSPHERICS.md`; tuning values: `docs/reference/environment.md`.

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use rs13_core::TICK_RATE_HZ;
use rs13_simulation::atmos::AtmosGrid;
use rs13_world::{AtmosView, Dense, Dir, GridPos, MapLayer, TileMap, TilePos};

use crate::doors::Airlock;
use crate::firedoors::{Firedoor, closed_edges, crosses_closed_edge};
use crate::hands::Item;
use crate::movement::Mob;
use crate::time::WorldTime;

/// Seconds per tick.
pub const TICK_SECONDS: f64 = 1.0 / TICK_RATE_HZ as f64;
/// Wind that moves loose items, m/s.
pub const ITEM_WIND: f64 = 15.0;
/// Wind that drags people, m/s. A strong gale is about 25 m/s.
pub const MOB_WIND: f64 = 30.0;
/// Fastest anything is blown along: one tile every this many ticks.
pub const MIN_PUSH_TICKS: u64 = 2;

/// The air of the station's deck.
#[derive(Resource, Debug)]
pub struct StationAtmos(pub AtmosGrid);

/// When each blown entity may be pushed again.
#[derive(Resource, Debug, Default)]
pub struct WindCooldowns(HashMap<Entity, u64>);

/// Build the air for a map with doors at `portals` and firedoors sealing the `edges`
/// between a tile and its neighbour in a direction.
pub fn build(map: &rs13_world::TileMap, portals: &[TilePos], edges: &[(TilePos, Dir)]) -> StationAtmos {
    let mut grid = AtmosGrid::new(map, portals);
    for (at, dir) in edges {
        grid.add_edge(*at, *dir);
    }
    StationAtmos(grid)
}

/// Tell the solver which doors and firedoors are open, then advance it one tick. An
/// airlock lets air through only while fully open, like it lets people through.
pub fn step_atmos(
    atmos: Option<ResMut<StationAtmos>>,
    doors: Query<(&GridPos, &Airlock)>,
    firedoors: Query<(&GridPos, &Firedoor)>,
) {
    let Some(mut atmos) = atmos else { return };
    for (pos, door) in &doors {
        atmos.0.set_portal_open(pos.0, !door.is_dense());
    }
    for (pos, door) in &firedoors {
        if let Some((at, dir)) = door.edge(pos.0) {
            atmos.0.set_edge_open(at, dir, !door.closed);
        }
    }
    atmos.0.step(TICK_SECONDS);
}

/// Ticks between pushes for a wind of `speed` m/s: faster wind, faster flight.
fn push_interval(speed: f64) -> u64 {
    ((200.0 / speed).round() as u64).clamp(MIN_PUSH_TICKS, 20)
}

/// Space wind: drag loose items and people along the airflow, out through doors and
/// breaches if that is where it goes.
pub fn space_wind(
    time: Res<WorldTime>,
    atmos: Option<Res<StationAtmos>>,
    map: Single<&MapLayer>,
    mut cooldowns: ResMut<WindCooldowns>,
    mut movers: Query<
        (Entity, &mut GridPos, Has<Mob>),
        (Or<(With<Mob>, With<Item>)>, Without<crate::closets::InCloset>),
    >,
    blockers: Query<&GridPos, (With<Dense>, Without<Mob>, Without<Item>)>,
    firedoors: Query<(&GridPos, &Firedoor), (Without<Mob>, Without<Item>)>,
    equipment: Query<&crate::equipment::Equipment>,
    wearables: Query<&crate::equipment::Wearable>,
    mut commands: Commands,
) {
    let Some(atmos) = atmos else { return };
    // Magnetic boots hold to the floor (Luna `shoes/magnetic`, `docs/reference/equipment.md`).
    let gripping = |mob: Entity| {
        equipment
            .get(mob)
            .ok()
            .and_then(|e| e.get(rs13_world::Slot::Shoes))
            .and_then(|shoes| wearables.get(shoes).ok())
            .is_some_and(|w| w.magnetic)
    };
    let now = time.tick.0;
    cooldowns.0.retain(|_, until| *until > now);
    let sealed = closed_edges(firedoors.iter());

    let mob_tiles: Vec<TilePos> = movers
        .iter()
        .filter(|(_, _, m)| *m)
        .map(|(_, p, _)| p.0)
        .collect();
    let blocked: Vec<TilePos> = blockers.iter().map(|p| p.0).collect();
    let mut order: Vec<Entity> = movers.iter().map(|(e, ..)| e).collect();
    order.sort();
    let mut moved_mobs: Vec<(TilePos, TilePos)> = Vec::new();

    for entity in order {
        if cooldowns.0.contains_key(&entity) {
            continue;
        }
        let Ok((_, mut pos, is_mob)) = movers.get_mut(entity) else {
            continue;
        };
        let Some(wind) = atmos.0.wind_at(pos.0) else {
            continue;
        };
        if is_mob && gripping(entity) {
            continue;
        }
        let threshold = if is_mob { MOB_WIND } else { ITEM_WIND };
        if wind.speed < threshold {
            continue;
        }
        let target = pos.0.step(wind.dir);
        let occupied = |p: TilePos| {
            (mob_tiles.contains(&p) && !moved_mobs.iter().any(|(from, _)| *from == p))
                || moved_mobs.iter().any(|(_, to)| *to == p)
        };
        if map.0.is_solid(target)
            || blocked.contains(&target)
            || crosses_closed_edge(&sealed, pos.0, target)
            || (is_mob && occupied(target))
        {
            continue;
        }
        if is_mob {
            moved_mobs.push((pos.0, target));
        }
        pos.0 = target;
        cooldowns.0.insert(entity, now + push_interval(wind.speed));
        // Blown out into space: keep flying.
        if is_space(&map.0, target) {
            commands.entity(entity).insert(Drift(wind.dir));
        }
    }
}

/// Moving through space with nothing to stop it.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Drift(pub Dir);

/// Ticks per tile while drifting.
pub const DRIFT_TICKS: u64 = 4;

pub fn is_space(map: &TileMap, pos: TilePos) -> bool {
    map.tile(pos).is_some_and(|t| t.id == "space")
}

/// Whether something next to `pos` (in any of the 8 directions) can be pushed off from.
pub fn has_handhold(map: &TileMap, pos: TilePos) -> bool {
    [
        Dir::North,
        Dir::NorthEast,
        Dir::East,
        Dir::SouthEast,
        Dir::South,
        Dir::SouthWest,
        Dir::West,
        Dir::NorthWest,
    ]
    .into_iter()
    .any(|d| map.tile(pos.step(d)).is_some_and(|t| t.id != "space"))
}

/// Drifting things keep going in a straight line until they reach something to stop on.
/// A switched-on jetpack holds its wearer still (`jetpacks.md`).
pub fn drift(
    time: Res<WorldTime>,
    map: Single<&MapLayer>,
    mut drifters: Query<(Entity, &mut GridPos, &Drift, Option<&crate::equipment::Equipment>)>,
    jetpacks: Query<&crate::jetpacks::Jetpack>,
    mut commands: Commands,
) {
    if !time.tick.0.is_multiple_of(DRIFT_TICKS) {
        return;
    }
    for (entity, mut pos, drift, equipment) in &mut drifters {
        let next = pos.0.step(drift.0);
        let held_still = crate::jetpacks::worn_jetpack(equipment, &jetpacks).is_some();
        if held_still || !is_space(&map.0, pos.0) || map.0.is_solid(next) {
            commands.entity(entity).remove::<Drift>();
            continue;
        }
        pos.0 = next;
    }
}

/// How often the debug overlay is refreshed, in ticks (twice a second).
pub const VIEW_TICKS: u64 = 10;

/// Refresh the per-tile pressure picture on the map entity, for the debug overlay.
pub fn publish_atmos_view(
    time: Res<WorldTime>,
    atmos: Option<Res<StationAtmos>>,
    mut maps: Query<(&MapLayer, &mut AtmosView)>,
) {
    let Some(atmos) = atmos else { return };
    if !time.tick.0.is_multiple_of(VIEW_TICKS) {
        return;
    }
    for (map, mut view) in &mut maps {
        let m = &map.0;
        let mut pressures = Vec::with_capacity((m.width * m.height) as usize);
        for y in 0..m.height as i32 {
            for x in 0..m.width as i32 {
                let p = atmos.0.pressure_at(TilePos::new(x, y, m.z));
                pressures.push((p * 10.0).round().clamp(0.0, f64::from(u16::MAX)) as u16);
            }
        }
        if view.pressure_dkpa != pressures {
            view.pressure_dkpa = pressures;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handholds_are_anything_but_space() {
        let map = TileMap::from_ron(
            r#"(
                legend: { ' ': (tile: (id: "space")), 'W': (tile: (id: "wall", solid: true)) },
                rows: ["     ", "     ", "     ", "     ", "W    "],
            )"#,
            0,
        )
        .unwrap();
        assert!(has_handhold(&map, TilePos::new(1, 1, 0)), "the wall is diagonal");
        assert!(!has_handhold(&map, TilePos::new(3, 3, 0)), "open space");
        assert!(is_space(&map, TilePos::new(3, 3, 0)));
    }
}
