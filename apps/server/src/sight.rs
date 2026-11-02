//! Send each player only what their mob can see (`docs/reference/lighting.md`, Line of
//! sight and Field of view), so a modified client can't look through walls, floors, bags
//! or behind its back. The client draws with the same rules (`rs13_world::sight`).

use std::collections::{HashMap, HashSet};

use bevy::ecs::query::QueryFilter;
use bevy::prelude::*;
use rs13_classic::closets::InCloset;
use rs13_classic::equipment::WornBy;
use rs13_classic::hands::HeldBy;
use rs13_classic::movement::Mob;
use rs13_classic::storage::{InStorage, OpenStorage};
use rs13_network::lightyear::prelude::*;
use rs13_world::{Facing, GridPos, Hidden, LightSource, MapLayer, NetId, Opaque, TilePos, sight};

use crate::Controls;
use crate::silhouettes::{Shade, Shaded};

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
    facing: Option<Facing>,
    at: TilePos,
    person: bool,
    seen: &mut impl FnMut(TilePos) -> bool,
) -> bool {
    let far = at.z != eye.z || (at.x - eye.x).abs() > SEND_RANGE || (at.y - eye.y).abs() > SEND_RANGE;
    let eye_f = [eye.x as f32, eye.y as f32];
    far || !seen(at)
        || (person && facing.is_some_and(|f| sight::behind_back(eye_f, sight::facing_vector(f.dir()), at)))
}

/// Where something off the map is.
#[derive(Debug, Clone, Copy)]
pub enum Holder {
    /// In a mob's hand, or worn by it.
    Mob(Entity),
    /// In a container (a backpack, a box).
    Storage(Entity),
    /// Shut inside a closet or locker.
    Closet,
}

/// Whether something off the map is out of sight: held or worn by someone out of sight,
/// inside a container the viewer hasn't got open, or shut in a closet. `placed_hidden`
/// says which things on the map are out of sight; `me` is the viewer's own mob.
pub fn held_out_of_sight(
    entity: Entity,
    me: Entity,
    open: Option<Entity>,
    holders: &HashMap<Entity, Holder>,
    placed_hidden: &HashSet<Entity>,
) -> bool {
    let mut at = entity;
    // Bounded, in case containers ever nest in a loop.
    for _ in 0..8 {
        if at == me {
            return false;
        }
        match holders.get(&at) {
            Some(Holder::Mob(mob)) => at = *mob,
            Some(Holder::Storage(container)) if open == Some(*container) => at = *container,
            Some(Holder::Storage(_) | Holder::Closet) => return true,
            None => return placed_hidden.contains(&at),
        }
    }
    true
}

/// Work out what each client's mob can't see, and stop or resume sending it.
///
/// Never sent: anything under intact floor tiles (`Hidden`), and anything off the map that
/// is out of sight (`held_out_of_sight`).
///
/// Always sent: the client's own mob and what it carries (but not what is in its bags
/// until one is opened), closed doors and light sources (the client needs them to draw
/// shadows and light around what it does see).
///
/// Only links Lightyear replicates to (they carry a `PriorityMap`) are handled, so every
/// change lands. Run once a tick for everyone, and every frame, before replication sends,
/// for links not yet handled (`F = Without<OutOfSight>`), so a client that has just joined
/// is never sent what it can't see.
#[allow(clippy::type_complexity)]
pub fn update_sight<F: QueryFilter>(
    mut links: Query<(Entity, &Controls, Option<&mut OutOfSight>), (With<PriorityMap>, F)>,
    viewers: Query<(Option<&Facing>, Option<&OpenStorage>)>,
    cameras: Query<&rs13_world::CameraView>,
    maps: Query<&MapLayer>,
    things: Query<
        (
            Entity,
            Option<&GridPos>,
            Has<Mob>,
            Has<Opaque>,
            Has<LightSource>,
            Has<Hidden>,
            (Option<&Shade>, Has<Shaded>),
        ),
        With<NetId>,
    >,
    contained: Query<(
        Entity,
        Option<&HeldBy>,
        Option<&WornBy>,
        Option<&InStorage>,
        Option<&InCloset>,
    )>,
    mut commands: Commands,
) {
    let closed: HashSet<TilePos> = things
        .iter()
        .filter(|(_, _, _, opaque, ..)| *opaque)
        .filter_map(|(_, pos, ..)| pos.map(|p| p.0))
        .collect();
    let holders: HashMap<Entity, Holder> = contained
        .iter()
        .filter_map(|(entity, held, worn, stored, closet)| {
            let holder = if let Some(h) = held {
                Holder::Mob(h.0)
            } else if let Some(w) = worn {
                Holder::Mob(w.0)
            } else if let Some(s) = stored {
                Holder::Storage(s.0)
            } else if closet.is_some() {
                Holder::Closet
            } else {
                return None;
            };
            Some((entity, holder))
        })
        .collect();
    let place = |entity: Entity| things.get(entity).ok().and_then(|(_, pos, ..)| pos.map(|p| p.0));
    for (link, controls, out) in &mut links {
        let me = controls.0;
        let Ok((facing, open)) = viewers.get(me) else {
            continue;
        };
        // Watching a camera, you see from it, all the way round (`cameras.rs`); shut in a
        // closet, from where the closet stands.
        let camera = cameras.get(me).ok().map(|v| v.0);
        let eye = camera.or_else(|| place(me)).or_else(|| {
            contained
                .get(me)
                .ok()
                .and_then(|(.., closet)| closet.and_then(|c| place(c.0)))
        });
        let Some(eye) = eye else { continue };
        let facing = if camera.is_some() {
            None
        } else {
            Some(facing.copied().unwrap_or(Facing(0)))
        };
        let open = open.map(|o| o.0);
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
        let placed_hidden: HashSet<Entity> = things
            .iter()
            .filter(
                |(entity, pos, person, opaque, light, under_floor, (shade, shaded))| {
                    let Some(at) = pos else { return false };
                    // Our own mob is always sent, even shut in a closet (which hides it from
                    // everyone else). Under intact floor tiles (pipes, cables, vents): no one
                    // sees it. Someone else in the dark is sent as their stand-in only, and
                    // our own stand-in never to us (`silhouettes.rs`).
                    if *entity == me {
                        return false;
                    }
                    if *shaded || shade.is_some_and(|s| s.0 == me) {
                        return true;
                    }
                    let person = *person || shade.is_some();
                    *under_floor
                        || (!*opaque && !*light && out_of_sight(eye, facing, at.0, person, &mut seen))
                },
            )
            .map(|(entity, ..)| entity)
            .collect();
        let hidden: HashSet<Entity> = things
            .iter()
            .filter(|(entity, pos, ..)| {
                if pos.is_some() {
                    placed_hidden.contains(entity)
                } else {
                    held_out_of_sight(*entity, me, open, &holders, &placed_hidden)
                }
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
        let north = Some(Facing::of(rs13_world::Dir::North));
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
        // Through a camera there is no behind (`cameras.md`).
        assert!(
            !out_of_sight(eye, None, TilePos::new(0, -3, 0), true, &mut seen),
            "a camera sees all round"
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

    #[test]
    fn what_people_carry_follows_them_and_bags_stay_shut() {
        let mut world = World::new();
        let [me, other, my_bag, their_bag, coin, wrench, locker_item] =
            std::array::from_fn(|_| world.spawn_empty().id());
        let holders = HashMap::from([
            (my_bag, Holder::Mob(me)),
            (coin, Holder::Storage(my_bag)),
            (their_bag, Holder::Mob(other)),
            (wrench, Holder::Storage(their_bag)),
            (locker_item, Holder::Closet),
        ]);
        let none = HashSet::new();
        let gone = HashSet::from([other]);
        let check = |e, open, hidden: &HashSet<Entity>| held_out_of_sight(e, me, open, &holders, hidden);
        assert!(!check(my_bag, None, &none), "what I carry");
        assert!(check(coin, None, &none), "inside my bag, shut");
        assert!(!check(coin, Some(my_bag), &none), "inside my bag, open");
        assert!(!check(their_bag, None, &none), "what someone in sight carries");
        assert!(check(their_bag, None, &gone), "carried by someone out of sight");
        assert!(check(wrench, None, &none), "inside their bag");
        assert!(check(locker_item, None, &none), "in a closet");
    }
}
