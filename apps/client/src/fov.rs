//! Field of view (`docs/reference/lighting.md`, Field of view).
//!
//! A person sees what is in front of them and to the sides, not behind. The arc behind our
//! player, turned the way they face, is darkened, and other people standing in it are
//! not drawn. Whoever is right next to us is still noticed. Luna has no such thing; this
//! follows later SS13 servers' view cones.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use rs13_world::{Facing, TilePos, sight};

use crate::sprites::Look;
use crate::{OwnPlayer, TILE_PX};

/// How wide the arc behind is, either side of straight back.
const HALF_BLIND: f32 = sight::BLIND_HALF_ANGLE_DEG.to_radians();
/// Over the light, under the line-of-sight shadows.
const FOV_Z: f32 = 20.4;
/// The cone fades in from nothing at the eye to full by this far out, px, so it has no
/// hard edge round the player.
const FADE: f32 = 1.5 * TILE_PX;
const OUTER: f32 = 40.0 * TILE_PX;
/// How many slices the cone's curved edges are made of.
const SEGMENTS: usize = 16;
/// Only ever darkens what is under it.
const SHADE: Color = Color::srgba(0.0, 0.0, 0.0, 0.4);

/// Where our player's eye is and which way they face, for this frame.
#[derive(Resource, Default)]
pub struct Fov(Option<(Vec2, Vec2)>);

impl Fov {
    /// Whether a person on `tile` is behind our player's back, out of sight: the same test
    /// the server uses (`rs13_world::sight::behind_back`).
    pub fn hides(&self, tile: TilePos) -> bool {
        self.0.is_some_and(|(eye, facing)| {
            sight::behind_back([eye.x / TILE_PX, eye.y / TILE_PX], facing.to_array(), tile)
        })
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
        MeshMaterial2d(materials.add(ColorMaterial::from(SHADE))),
        Transform::from_xyz(0.0, 0.0, FOV_Z),
        Visibility::Hidden,
    ));
}

/// The blind arc of someone at the origin facing north: centred on straight down, fading
/// in from the eye to `FADE`, then full out to `OUTER`. Vertex colours carry the fade.
fn cone_mesh() -> Mesh {
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    let start = -std::f32::consts::FRAC_PI_2 - HALF_BLIND;
    // Three corners along each spoke: the eye (clear), the end of the fade, the far end.
    let rings = [(0.0, 0.0), (FADE, 1.0), (OUTER, 1.0)];
    for i in 0..=SEGMENTS {
        let angle = start + 2.0 * HALF_BLIND * i as f32 / SEGMENTS as f32;
        let dir = Vec2::from_angle(angle);
        for (r, alpha) in rings {
            positions.push([dir.x * r, dir.y * r, 0.0]);
            colors.push([1.0, 1.0, 1.0, alpha]);
        }
        if i > 0 {
            let (a, b) = (3 * (i as u32 - 1), 3 * i as u32);
            for ring in 0..2 {
                indices.extend([
                    a + ring,
                    a + ring + 1,
                    b + ring + 1,
                    a + ring,
                    b + ring + 1,
                    b + ring,
                ]);
            }
        }
    }
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
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
    let facing = Vec2::from_array(sight::facing_vector(Facing(look.facing as u8).dir()));
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
