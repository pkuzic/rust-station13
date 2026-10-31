//! Send each player only what their mob can see (`docs/reference/lighting.md`, Line of
//! sight and Field of view), so a modified client can't look through walls or behind its
//! back. The client draws with the same rules (`rs13_world::sight`).

use std::collections::{HashMap, HashSet};

use bevy::ecs::query::QueryFilter;
use bevy::prelude::*;
use rs13_classic::movement::Mob;
use rs13_network::lightyear::prelude::*;
use rs13_world::{Facing, GridPos, Hidden, LightSource, MapLayer, NetId, Opaque, TilePos, sight};

use crate::Controls;

/// How far out things are still sent, in tiles: one past the view, so whoever walks in
/// is already there to slide into sight.
const SEND_RANGE: i32 = sight::VIEW_RANGE + 1;

/// On a client's link: the entities it is not being sent right now.
#[derive(Component, Debug, Default)]
pub struct OutOfSight(HashSet<Entity>);

/// Whether a viewer at `eye` (facing `facing`) can't see something at `at`: too far, on
/// another deck, behind a wall or closed door, or, for a person, behind their back.
pub fn out_of_sight(
    eye: TilePos,
    facing: Facing,
    at: TilePos,
    person: bool,
    seen: &mut impl FnMut(TilePos) -> bool,
) -> bool {
    let far = at.z != eye.z || (at.x - eye.x).abs() > SEND_RANGE || (at.y - eye.y).abs() > SEND_RANGE;
    let eye_f = [eye.x as f32, eye.y as f32];
    far || !seen(at) || (person && sight::behind_back(eye_f, sight::facing_vector(facing.dir()), at))
}

/// Work out what each client's mob can't see, and stop or resume sending it.
///
/// Never sent: anything under intact floor tiles (`Hidden`).
///
/// Always sent otherwise: anything not on the map (held, worn or stored: the client needs its own
/// inventory), closed doors and light sources (the client needs them to draw shadows and
/// light around what it does see), and the client's own mob.
///
/// Only links Lightyear replicates to (they carry a `PriorityMap`) are handled, so every
/// change lands. Run once a tick for everyone, and every frame, before replication sends,
/// for links not yet handled (`F = Without<OutOfSight>`), so a client that has just joined
/// is never sent what it can't see.
#[allow(clippy::type_complexity)]
pub fn update_sight<F: QueryFilter>(
    mut links: Query<(Entity, &Controls, Option<&mut OutOfSight>), (With<PriorityMap>, F)>,
    viewers: Query<(&GridPos, Option<&Facing>)>,
    maps: Query<&MapLayer>,
    things: Query<
        (
            Entity,
            Option<&GridPos>,
            Has<Mob>,
            Has<Opaque>,
            Has<LightSource>,
            Has<Hidden>,
        ),
        With<NetId>,
    >,
    mut commands: Commands,
) {
    let closed: HashSet<TilePos> = things
        .iter()
        .filter(|(_, _, _, opaque, ..)| *opaque)
        .filter_map(|(_, pos, ..)| pos.map(|p| p.0))
        .collect();
    for (link, controls, out) in &mut links {
        let Ok((eye, facing)) = viewers.get(controls.0) else {
            continue;
        };
        let eye = eye.0;
        let facing = facing.copied().unwrap_or(Facing(0));
        let Some(map) = maps.iter().find(|m| m.0.z == eye.z) else {
            continue;
        };
        let mut tiles: HashMap<TilePos, bool> = HashMap::new();
        let mut seen = |at: TilePos| {
            *tiles.entry(at).or_insert_with(|| {
                sight::sees([eye.x as f32, eye.y as f32], at, |p| {
                    map.0.is_opaque(p) || closed.contains(&p)
                })
            })
        };
        let hidden: HashSet<Entity> = things
            .iter()
            .filter(|(entity, pos, person, opaque, light, under_floor)| {
                let Some(at) = pos else { return false };
                // Under intact floor tiles (pipes, cables, vents): no one sees it.
                *under_floor
                    || (*entity != controls.0
                        && !*opaque
                        && !*light
                        && out_of_sight(eye, facing, at.0, *person, &mut seen))
            })
            .map(|(entity, ..)| entity)
            .collect();

        let empty = HashSet::new();
        let before = out.as_ref().map_or(&empty, |o| &o.0);
        for entity in hidden.difference(before) {
            commands.lose_visibility(*entity, link);
        }
        for entity in before.difference(&hidden) {
            if things.contains(*entity) {
                commands.gain_visibility(*entity, link);
            }
        }
        match out {
            Some(mut out) => out.0 = hidden,
            None => {
                commands.entity(link).insert(OutOfSight(hidden));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn people_behind_walls_or_backs_are_not_sent_but_neighbours_are() {
        let eye = TilePos::new(0, 0, 0);
        let north = Facing::of(rs13_world::Dir::North);
        // A wall at (2, 0): what's behind it isn't seen.
        let mut seen = |p: TilePos| !(p.x > 2 && p.y == 0);
        assert!(out_of_sight(eye, north, TilePos::new(4, 0, 0), false, &mut seen));
        assert!(
            !out_of_sight(eye, north, TilePos::new(0, 3, 0), true, &mut seen),
            "ahead"
        );
        assert!(
            out_of_sight(eye, north, TilePos::new(0, -3, 0), true, &mut seen),
            "behind"
        );
        assert!(
            !out_of_sight(eye, north, TilePos::new(0, -3, 0), false, &mut seen),
            "an item behind"
        );
        assert!(
            !out_of_sight(eye, north, TilePos::new(0, -1, 0), true, &mut seen),
            "next to"
        );
        assert!(
            out_of_sight(eye, north, TilePos::new(0, 9, 0), false, &mut seen),
            "too far"
        );
        assert!(
            out_of_sight(eye, north, TilePos::new(0, 1, 1), false, &mut seen),
            "another deck"
        );
    }
}
