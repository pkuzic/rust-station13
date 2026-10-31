//! Line of sight as real shadows (`docs/reference/lighting.md`, Line of sight).
//!
//! Every wall and closed door near the player casts a shadow away from the player's eye:
//! each of its edges that faces away from the eye sweeps a quad out past the edge of the
//! screen. The opaque square itself stays lit, so walls bordering what you see are seen.
//! The shadows follow the player's drawn position, so they glide as the player walks.
//! With meson scanners on they are see-through, showing the floor plan beyond (Luna
//! `SEE_TURFS`).

use std::collections::HashSet;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use rs13_world::{GridPos, MapLayer, Opaque, TilePos};

use crate::lighting::Sight;
use crate::{OwnPlayer, TILE_PX};

/// Over the light, under nothing else on the map.
const SHADOW_Z: f32 = 20.5;
/// How far around the eye walls cast shadows, in tiles: past the 15 × 15 view.
const REACH: i32 = 9;
/// How far a shadow reaches, px: well past the screen.
const SHADOW_LENGTH: f32 = 40.0 * TILE_PX;
/// How dark the shadows are; with meson scanners, how much of the floor plan shows.
const SHADOW_ALPHA: f32 = 1.0;
const MESON_ALPHA: f32 = 0.78;

#[derive(Component)]
pub struct Shadows;

/// What the shadow mesh was last built from: the eye, rounded to a pixel, and the walls.
#[derive(Resource, Default)]
pub struct ShadowInputs(Option<(IVec2, u64)>);

pub struct ShadowPlugin;

impl Plugin for ShadowPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShadowInputs>()
            .add_systems(Startup, spawn_shadows)
            .add_systems(PostUpdate, update_shadows);
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

/// The four corners of a tile's square, counter-clockwise from the bottom left, px.
fn corners(tile: TilePos) -> [Vec2; 4] {
    let c = Vec2::new(tile.x as f32, tile.y as f32) * TILE_PX;
    let h = TILE_PX / 2.0;
    [
        c + Vec2::new(-h, -h),
        c + Vec2::new(h, -h),
        c + Vec2::new(h, h),
        c + Vec2::new(-h, h),
    ]
}

/// Shadow quads cast from `eye` by each opaque square: one per edge facing away from it.
pub fn shadow_quads(eye: Vec2, opaque: &HashSet<TilePos>) -> Vec<[Vec2; 4]> {
    let mut quads = Vec::new();
    for tile in opaque {
        let c = corners(*tile);
        for i in 0..4 {
            let (a, b) = (c[i], c[(i + 1) % 4]);
            // The edge's outward normal, for a counter-clockwise square.
            let edge = b - a;
            let normal = Vec2::new(edge.y, -edge.x);
            // Edges between two opaque squares cast nothing of their own.
            let mid = (a + b) / 2.0;
            let beyond = mid + normal.normalize() * (TILE_PX / 2.0);
            let next = TilePos::new(
                (beyond.x / TILE_PX).round() as i32,
                (beyond.y / TILE_PX).round() as i32,
                tile.z,
            );
            if opaque.contains(&next) {
                continue;
            }
            if normal.dot(mid - eye) <= 0.0 {
                continue; // Faces the eye: lit.
            }
            let far = |p: Vec2| p + (p - eye).normalize_or_zero() * SHADOW_LENGTH;
            quads.push([a, b, far(b), far(a)]);
        }
    }
    quads
}

/// Whether the centre of `tile` can be seen from `eye`: the straight line between them
/// passes through no opaque square but the tile's own.
pub fn sees(eye: Vec2, tile: TilePos, opaque: &HashSet<TilePos>) -> bool {
    let target = Vec2::new(tile.x as f32, tile.y as f32) * TILE_PX;
    let steps = ((target - eye).length() / 4.0).ceil().max(1.0) as i32;
    (1..steps).all(|i| {
        let p = eye + (target - eye) * (i as f32 / steps as f32);
        let at = TilePos::new(
            (p.x / TILE_PX).round() as i32,
            (p.y / TILE_PX).round() as i32,
            tile.z,
        );
        at == tile || !opaque.contains(&at)
    })
}

/// Rebuild the shadows when the eye moves a pixel or a wall or door changes.
fn update_shadows(
    mut inputs: ResMut<ShadowInputs>,
    sight: Res<Sight>,
    lighting_on: Res<crate::lighting::LightingOn>,
    me: Query<(&Transform, &GridPos), With<OwnPlayer>>,
    maps: Query<&MapLayer>,
    doors: Query<&GridPos, With<Opaque>>,
    shadows: Single<(&Mesh2d, &MeshMaterial2d<ColorMaterial>, &mut Visibility), With<Shadows>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let (mesh, material, mut visibility) = shadows.into_inner();
    let (Ok((transform, pos)), true) = (me.single(), lighting_on.0) else {
        visibility.set_if_neq(Visibility::Hidden);
        return;
    };
    visibility.set_if_neq(Visibility::Inherited);
    let Some(map) = maps.iter().find(|m| m.0.z == pos.0.z) else {
        return;
    };
    let opaque = opaque_near(&map.0, pos.0, doors.iter().map(|p| p.0));
    let eye = transform.translation.truncate();

    let mut digest = std::collections::hash_map::DefaultHasher::new();
    let mut walls: Vec<&TilePos> = opaque.iter().collect();
    walls.sort();
    std::hash::Hash::hash(&walls, &mut digest);
    std::hash::Hash::hash(&sight.meson, &mut digest);
    let key = (eye.round().as_ivec2(), std::hash::Hasher::finish(&digest));
    if inputs.0 == Some(key) {
        return;
    }
    inputs.0 = Some(key);

    if let Some(mut m) = materials.get_mut(&material.0) {
        m.color = Color::srgba(
            0.0,
            0.0,
            0.0,
            if sight.meson { MESON_ALPHA } else { SHADOW_ALPHA },
        );
    }
    let quads = shadow_quads(eye, &opaque);
    let positions: Vec<[f32; 3]> = quads.iter().flatten().map(|p| [p.x, p.y, 0.0]).collect();
    let indices: Vec<u32> = (0..quads.len() as u32)
        .flat_map(|q| {
            let i = q * 4;
            [i, i + 1, i + 2, i, i + 2, i + 3]
        })
        .collect();
    if let Some(mut m) = meshes.get_mut(&mesh.0) {
        m.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        m.insert_indices(Indices::U32(indices));
    }
}

/// Opaque squares within reach of `at`: opaque tiles and closed doors.
pub fn opaque_near(
    map: &rs13_world::TileMap,
    at: TilePos,
    doors: impl Iterator<Item = TilePos>,
) -> HashSet<TilePos> {
    let near = |p: &TilePos| p.z == at.z && (p.x - at.x).abs() <= REACH && (p.y - at.y).abs() <= REACH;
    let mut opaque: HashSet<TilePos> = (-REACH..=REACH)
        .flat_map(|dy| (-REACH..=REACH).map(move |dx| TilePos::new(at.x + dx, at.y + dy, at.z)))
        .filter(|p| map.is_opaque(*p))
        .collect();
    opaque.extend(doors.filter(near));
    opaque
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether any shadow quad starts inside the square of `tile`'s near half (towards the
    /// eye at the origin, east of it): the wall itself would be shaded.
    fn shades_near_face(quads: &[[Vec2; 4]], tile: TilePos) -> bool {
        let near_x = tile.x as f32 * TILE_PX - TILE_PX / 2.0;
        quads
            .iter()
            .any(|q| q[0].x < near_x - 0.01 || q[1].x < near_x - 0.01)
    }

    #[test]
    fn a_wall_shadows_what_is_behind_it_but_not_itself() {
        let wall = TilePos::new(2, 0, 0);
        let opaque = HashSet::from([wall]);
        let eye = Vec2::ZERO;
        assert!(sees(eye, wall, &opaque), "the wall itself is seen");
        assert!(!sees(eye, TilePos::new(4, 0, 0), &opaque), "behind it is not");
        assert!(sees(eye, TilePos::new(0, 3, 0), &opaque), "beside it is");
        let quads = shadow_quads(eye, &opaque);
        assert!(!quads.is_empty());
        assert!(!shades_near_face(&quads, wall));
    }

    #[test]
    fn edges_between_two_walls_cast_nothing() {
        let opaque = HashSet::from([TilePos::new(2, 0, 0), TilePos::new(3, 0, 0)]);
        let quads = shadow_quads(Vec2::ZERO, &opaque);
        let shared = 2.5 * TILE_PX;
        assert!(
            !quads
                .iter()
                .any(|q| (q[0].x - shared).abs() < 0.01 && (q[1].x - shared).abs() < 0.01),
            "no quad on the shared edge"
        );
    }
}
