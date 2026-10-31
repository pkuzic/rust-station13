//! Field of view (`docs/reference/lighting.md`, Field of view).
//!
//! A person sees what is in front of them and to the sides, not behind. The arc behind our
//! player, turned the way they face, is grayed over, and other people standing in it are
//! not drawn. Whoever is right next to us is still noticed. Luna has no such thing; this
//! follows later SS13 servers' view cones.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use rs13_world::TilePos;

use crate::sprites::Look;
use crate::{OwnPlayer, TILE_PX};

/// How wide the arc behind is, either side of straight back.
const HALF_BLIND: f32 = 60.0_f32.to_radians();
/// Over the light, under the line-of-sight shadows.
const FOV_Z: f32 = 20.4;
/// The cone starts this far from the eye, so our own tile stays clear, px.
const INNER: f32 = 0.75 * TILE_PX;
const OUTER: f32 = 40.0 * TILE_PX;
/// How many slices the cone's curved edges are made of.
const SEGMENTS: usize = 16;
const GRAY: Color = Color::srgba(0.12, 0.12, 0.14, 0.55);

/// Where our player's eye is and which way they face, for this frame.
#[derive(Resource, Default)]
pub struct Fov(Option<(Vec2, Vec2)>);

impl Fov {
    /// Whether something on `tile` is behind our player, out of sight. Our own tile and the
    /// ones around it never are.
    pub fn hides(&self, tile: TilePos) -> bool {
        let Some((eye, facing)) = self.0 else {
            return false;
        };
        let centre = Vec2::new(tile.x as f32, tile.y as f32) * TILE_PX;
        let near = (centre - eye).abs().max_element() < 1.5 * TILE_PX;
        !near && behind(eye, facing, centre)
    }
}

/// Whether `p` lies in the blind arc of someone at `eye` facing `facing` (a unit vector).
pub fn behind(eye: Vec2, facing: Vec2, p: Vec2) -> bool {
    let to = (p - eye).normalize_or_zero();
    to != Vec2::ZERO && to.dot(-facing) > HALF_BLIND.cos()
}

/// The way a DMI facing index points (0 south, 1 north, 2 east, 3 west).
fn facing_vec(facing: u32) -> Vec2 {
    match facing {
        1 => Vec2::Y,
        2 => Vec2::X,
        3 => Vec2::NEG_X,
        _ => Vec2::NEG_Y,
    }
}

#[derive(Component)]
struct Cone;

pub struct FovPlugin;

impl Plugin for FovPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Fov>()
            .add_systems(Startup, spawn_cone)
            .add_systems(PostUpdate, update_fov);
    }
}

fn spawn_cone(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((
        Cone,
        Mesh2d(meshes.add(cone_mesh())),
        MeshMaterial2d(materials.add(ColorMaterial::from(GRAY))),
        Transform::from_xyz(0.0, 0.0, FOV_Z),
        Visibility::Hidden,
    ));
}

/// The blind arc of someone at the origin facing north: centred on straight down, a band
/// from `INNER` to `OUTER`.
fn cone_mesh() -> Mesh {
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    let start = -std::f32::consts::FRAC_PI_2 - HALF_BLIND;
    for i in 0..=SEGMENTS {
        let angle = start + 2.0 * HALF_BLIND * i as f32 / SEGMENTS as f32;
        let dir = Vec2::from_angle(angle);
        positions.push([dir.x * INNER, dir.y * INNER, 0.0]);
        positions.push([dir.x * OUTER, dir.y * OUTER, 0.0]);
        if i > 0 {
            let k = 2 * i as u32;
            indices.extend([k - 2, k - 1, k + 1, k - 2, k + 1, k]);
        }
    }
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_indices(Indices::U32(indices))
}

/// Follow our player's eye and facing with the cone; hide it with the lighting off.
fn update_fov(
    mut fov: ResMut<Fov>,
    lighting_on: Res<crate::lighting::LightingOn>,
    me: Query<(&Transform, &Look), (With<OwnPlayer>, Without<Cone>)>,
    cone: Single<(&mut Transform, &mut Visibility), With<Cone>>,
) {
    let (mut transform, mut visibility) = cone.into_inner();
    let (Ok((at, look)), true) = (me.single(), lighting_on.0) else {
        fov.0 = None;
        visibility.set_if_neq(Visibility::Hidden);
        return;
    };
    let eye = at.translation.truncate();
    let facing = facing_vec(look.facing);
    fov.0 = Some((eye, facing));
    visibility.set_if_neq(Visibility::Inherited);
    transform.translation = eye.extend(FOV_Z);
    // The mesh points behind someone facing north; turn it to our facing.
    transform.rotation = Quat::from_rotation_z(Vec2::Y.angle_to(facing));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn behind_is_the_back_arc() {
        let eye = Vec2::ZERO;
        let north = Vec2::Y;
        assert!(behind(eye, north, Vec2::new(0.0, -100.0)));
        assert!(behind(eye, north, Vec2::new(40.0, -100.0)));
        assert!(!behind(eye, north, Vec2::new(100.0, 0.0)), "beside is seen");
        assert!(!behind(eye, north, Vec2::new(0.0, 100.0)), "ahead is seen");
    }

    #[test]
    fn neighbours_are_never_hidden() {
        let fov = Fov(Some((Vec2::ZERO, Vec2::Y)));
        assert!(!fov.hides(TilePos::new(0, -1, 0)), "right behind us");
        assert!(fov.hides(TilePos::new(0, -3, 0)));
        assert!(!fov.hides(TilePos::new(0, 3, 0)));
        assert!(
            !Fov::default().hides(TilePos::new(0, -3, 0)),
            "no player, no cone"
        );
    }
}
