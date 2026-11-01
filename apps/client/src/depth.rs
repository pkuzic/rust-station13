//! A touch of depth (`docs/reference/lighting.md`, Depth): a crisp dark line along the foot
//! of every wall, a few sprite pixels wide, as if the walls stood up out of the floor,
//! without any new art.
//!
//! One mesh per deck: along each edge between a floor tile and a wall, a thin strip on the
//! floor side. Drawn over the floor, under everything on it; rebuilt only when the map
//! changes.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use rs13_world::{MapLayer, TileMap, TilePos};

use crate::TILE_PX;

/// Over the floor tiles (0), under pipes and everything else (0.5 and up).
const DEPTH_Z: f32 = 0.3;
/// How far the shade reaches into the floor, px: a crisp line, a few sprite pixels.
const REACH: f32 = 3.0;
/// How dark the line is.
const SHADE: f32 = 0.35;

#[derive(Component)]
struct WallShade(i32);

pub struct DepthPlugin;

impl Plugin for DepthPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, shade_walls);
    }
}

/// The strips along the foot of every wall: per strip, its four corners (two at the wall,
/// two out on the floor), in px.
pub fn strips(map: &TileMap) -> Vec<[[f32; 2]; 4]> {
    let mut out = Vec::new();
    let h = TILE_PX / 2.0;
    for y in 0..map.height as i32 {
        for x in 0..map.width as i32 {
            let floor = TilePos::new(x, y, map.z);
            let open = map.tile(floor).is_some_and(|t| !t.solid && t.id != "space");
            if !open {
                continue;
            }
            let c = [x as f32 * TILE_PX, y as f32 * TILE_PX];
            for (dx, dy) in [(0, 1), (0, -1), (1, 0), (-1, 0)] {
                let next = TilePos::new(x + dx, y + dy, map.z);
                if !map.tile(next).is_some_and(|t| t.solid && t.is_opaque()) {
                    continue;
                }
                let (dx, dy) = (dx as f32, dy as f32);
                // The edge shared with the wall, and the same edge moved in by `REACH`.
                let edge_mid = [c[0] + dx * h, c[1] + dy * h];
                let along = [dy.abs() * h, dx.abs() * h];
                let a = [edge_mid[0] - along[0], edge_mid[1] - along[1]];
                let b = [edge_mid[0] + along[0], edge_mid[1] + along[1]];
                let inward = [-dx * REACH, -dy * REACH];
                out.push([
                    a,
                    b,
                    [b[0] + inward[0], b[1] + inward[1]],
                    [a[0] + inward[0], a[1] + inward[1]],
                ]);
            }
        }
    }
    out
}

fn mesh_of(map: &TileMap) -> Mesh {
    let strips = strips(map);
    let mut positions = Vec::with_capacity(strips.len() * 4);
    let mut indices = Vec::with_capacity(strips.len() * 6);
    for (i, s) in strips.iter().enumerate() {
        positions.extend(s.iter().map(|p| [p[0], p[1], 0.0]));
        let j = 4 * i as u32;
        indices.extend([j, j + 1, j + 2, j, j + 2, j + 3]);
    }
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_indices(Indices::U32(indices))
}

/// Build each deck's shading when its map arrives or changes.
fn shade_walls(
    mut commands: Commands,
    maps: Query<&MapLayer, Changed<MapLayer>>,
    existing: Query<(Entity, &WallShade)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for map in &maps {
        for (entity, shade) in &existing {
            if shade.0 == map.0.z {
                commands.entity(entity).despawn();
            }
        }
        commands.spawn((
            WallShade(map.0.z),
            crate::OnDeck(map.0.z),
            Mesh2d(meshes.add(mesh_of(&map.0))),
            MeshMaterial2d(materials.add(ColorMaterial::from(Color::srgba(0.0, 0.0, 0.0, SHADE)))),
            Transform::from_xyz(0.0, 0.0, DEPTH_Z),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floor_beside_walls_is_shaded_on_that_side_only() {
        let map = TileMap::from_ron(
            r#"(
                legend: { 'W': (tile: (id: "wall", solid: true)), '.': (tile: (id: "floor")) },
                rows: ["WWW", "W..", "WWW"],
            )"#,
            0,
        )
        .unwrap();
        let s = strips(&map);
        // (1, 1) has walls north, south and west; (2, 1) north and south.
        assert_eq!(s.len(), 5);
        // A strip starts at the wall edge and reaches `REACH` into the floor.
        let west = s
            .iter()
            .find(|q| q[0][0] == 0.5 * TILE_PX && q[1][0] == 0.5 * TILE_PX)
            .unwrap();
        assert!((west[2][0] - (0.5 * TILE_PX + REACH)).abs() < 1e-3);
    }
}
