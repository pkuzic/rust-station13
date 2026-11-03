//! Line of sight in whole tiles, as BYOND draws it (`docs/reference/lighting.md`, Line of
//! sight): each tile is either seen or black.
//!
//! A tile is seen when the straight line from the centre of the player's tile to its
//! centre crosses no wall or closed door (`rs13_world::sight::sees`, the test the server
//! uses to decide what to send). A wall or closed door is also seen when a floor tile
//! beside it is, and a room's corner when the floor touching its corner is. Everything
//! else in view is covered by black squares; with meson scanners on they are see-through,
//! showing the floor plan beyond (Luna `SEE_TURFS`).

use std::collections::HashSet;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use rs13_world::sight::{self, seen_tiles};
use rs13_world::{Appearance, GridPos, MapLayer, NetId, Opaque, Slot, TileMap, TilePos, WornView};

use crate::{OwnPlayer, TILE_PX};

/// Over the light and the view cone, under nothing else on the map.
const SHADOW_Z: f32 = 20.5;
/// How far around the player tiles are worked out, in tiles: past the 15 × 15 view.
const REACH: i32 = 9;
/// How dark unseen tiles are; with meson scanners, how much of the floor plan shows.
const SHADOW_ALPHA: f32 = 1.0;
const MESON_ALPHA: f32 = 0.78;

/// What the player can see right now (Luna's `view`).
#[derive(Resource, Default)]
pub struct Sight {
    /// The tiles of our deck the player sees; `None` shows everything (lighting off, or
    /// before our player arrives).
    pub seen: Option<HashSet<TilePos>>,
    /// Meson scanners on: the floor plan shows through walls (Luna `SEE_TURFS`).
    pub meson: bool,
}

impl Sight {
    /// Whether `tile` is seen.
    pub fn sees(&self, tile: TilePos) -> bool {
        self.seen.as_ref().is_none_or(|seen| seen.contains(&tile))
    }
}

/// Walls and closed doors near `at` (`opaque_near` used to be the shadow casters).
fn blocks_at<'a>(map: &'a TileMap, doors: &'a HashSet<TilePos>) -> impl Fn(TilePos) -> bool + 'a {
    move |p| map.is_opaque(p) || doors.contains(&p)
}

#[derive(Component)]
struct Shadows;

/// What the black squares were last drawn from: our tile, the walls and doors, meson.
#[derive(Resource, Default)]
struct ShadowInputs(Option<u64>);

pub struct ShadowPlugin;

impl Plugin for ShadowPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShadowInputs>()
            .init_resource::<Sight>()
            .add_systems(Startup, spawn_shadows)
            .add_systems(Update, update_sight.before(crate::lighting::DrawLight));
    }
}

fn empty_mesh() -> Mesh {
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new())
        .with_inserted_indices(Indices::U32(Vec::new()))
}

fn spawn_shadows(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((
        Shadows,
        Mesh2d(meshes.add(empty_mesh())),
        MeshMaterial2d(materials.add(ColorMaterial::from(Color::srgba(0.0, 0.0, 0.0, SHADOW_ALPHA)))),
        Transform::from_xyz(0.0, 0.0, SHADOW_Z),
    ));
}

/// Work out what the player sees when they move to another tile or a wall or door
/// changes, and cover the rest in black.
#[allow(clippy::too_many_arguments)]
fn update_sight(
    mut inputs: ResMut<ShadowInputs>,
    mut sight: ResMut<Sight>,
    lighting_on: Res<crate::lighting::LightingOn>,
    me: Query<(&GridPos, Option<&WornView>, Option<&rs13_world::CameraView>), With<OwnPlayer>>,
    items: Query<(&NetId, &Appearance)>,
    maps: Query<&MapLayer>,
    doors: Query<&GridPos, With<Opaque>>,
    shadows: Single<(&Mesh2d, &MeshMaterial2d<ColorMaterial>, &mut Visibility), With<Shadows>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let (mesh, material, mut visibility) = shadows.into_inner();
    let (Ok((pos, worn, view)), true) = (me.single(), lighting_on.0) else {
        visibility.set_if_neq(Visibility::Hidden);
        if sight.seen.is_some() {
            sight.seen = None;
        }
        inputs.0 = None;
        return;
    };
    // Watching a security camera, you see from it (`cameras.md`).
    let eye = GridPos(view.map_or(pos.0, |v| v.0));
    let pos = &eye;
    visibility.set_if_neq(Visibility::Inherited);
    let Some(map) = maps.iter().find(|m| m.0.z == pos.0.z) else {
        return;
    };
    let me = pos.0;
    let meson = worn
        .and_then(|w| w.get(Slot::Glasses))
        .and_then(|id| items.iter().find(|(n, _)| **n == id))
        .is_some_and(|(_, a)| a.sprite == "meson-glasses");
    let mut closed: Vec<TilePos> = doors.iter().map(|p| p.0).filter(|p| p.z == me.z).collect();
    closed.sort();

    let mut digest = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(&(me, &closed, meson), &mut digest);
    std::hash::Hash::hash(&map.0.width, &mut digest);
    let key = std::hash::Hasher::finish(&digest);
    if inputs.0 == Some(key) && sight.seen.is_some() {
        return;
    }
    inputs.0 = Some(key);

    let closed: HashSet<TilePos> = closed.into_iter().collect();
    let seen = seen_tiles(me, sight::VIEW_RANGE, blocks_at(&map.0, &closed));

    // Black squares over every tile within reach that isn't seen.
    let h = TILE_PX / 2.0;
    let mut positions = Vec::new();
    for dy in -REACH..=REACH {
        for dx in -REACH..=REACH {
            let tile = TilePos::new(me.x + dx, me.y + dy, me.z);
            if seen.contains(&tile) {
                continue;
            }
            let c = Vec2::new(tile.x as f32, tile.y as f32) * TILE_PX;
            positions.extend([
                [c.x - h, c.y - h, 0.0],
                [c.x + h, c.y - h, 0.0],
                [c.x + h, c.y + h, 0.0],
                [c.x - h, c.y + h, 0.0],
            ]);
        }
    }
    let indices: Vec<u32> = (0..positions.len() as u32 / 4)
        .flat_map(|q| {
            let i = q * 4;
            [i, i + 1, i + 2, i, i + 2, i + 3]
        })
        .collect();
    if let Some(mut m) = meshes.get_mut(&mesh.0) {
        m.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        m.insert_indices(Indices::U32(indices));
    }
    if let Some(mut m) = materials.get_mut(&material.0) {
        m.color = Color::srgba(0.0, 0.0, 0.0, if meson { MESON_ALPHA } else { SHADOW_ALPHA });
    }
    sight.seen = Some(seen);
    sight.meson = meson;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn walls(tiles: &'static [(i32, i32)]) -> impl Fn(TilePos) -> bool {
        move |p| tiles.contains(&(p.x, p.y))
    }

    #[test]
    fn behind_a_wall_is_unseen_but_the_wall_is_seen() {
        let seen = seen_tiles(TilePos::new(0, 0, 0), 7, walls(&[(2, 0)]));
        assert!(seen.contains(&TilePos::new(2, 0, 0)), "the wall");
        assert!(!seen.contains(&TilePos::new(4, 0, 0)), "behind it");
        assert!(seen.contains(&TilePos::new(0, 3, 0)));
    }

    #[test]
    fn a_rooms_walls_and_corners_are_seen_but_not_the_wall_behind() {
        // A room from (-2, -2) to (2, 2) in a ring of walls, and a second ring outside it.
        let ring = |r: i32| {
            (-r..=r)
                .flat_map(move |x| (-r..=r).map(move |y| (x, y)))
                .filter(move |(x, y)| x.abs() == r || y.abs() == r)
        };
        let set: HashSet<(i32, i32)> = ring(3).chain(ring(4)).collect();
        let seen = seen_tiles(TilePos::new(0, 0, 0), 7, move |p| set.contains(&(p.x, p.y)));
        for t in [(3, 0), (0, -3), (3, 3), (-3, -3)] {
            assert!(seen.contains(&TilePos::new(t.0, t.1, 0)), "{t:?}");
        }
        assert!(!seen.contains(&TilePos::new(4, 0, 0)), "the wall behind the wall");
    }

    #[test]
    fn a_door_diagonally_past_a_wall_corner_is_seen() {
        let seen = seen_tiles(
            TilePos::new(32, 18, 0),
            7,
            walls(&[(33, 19), (33, 18), (33, 17), (33, 16)]),
        );
        assert!(seen.contains(&TilePos::new(33, 17, 0)));
    }
}
