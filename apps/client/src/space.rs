//! Space behind the station (`docs/reference/lighting.md`, Space): instead of Luna's still
//! star tiles, three layers of stars drawn by `space.wgsl` slide past at different speeds
//! as the camera moves, and drift slowly as the ship travels.
//!
//! One quad, as big as any window, follows the camera under the map; space tiles draw
//! nothing over it.

use bevy::asset::{AssetPath, embedded_asset, embedded_path};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};

use crate::WorldCamera;

/// Under the floor tiles (0).
const SPACE_Z: f32 = -1.0;
/// World px the quad covers each way: more than any window shows.
const SIZE: f32 = 8192.0;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct SpaceMaterial {
    /// Camera x and y (world px), seconds, unused.
    #[uniform(0)]
    params: Vec4,
}

impl Material2d for SpaceMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(AssetPath::from_path_buf(embedded_path!("space.wgsl")).with_source("embedded"))
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Opaque
    }
}

#[derive(Component)]
struct Starfield(Handle<SpaceMaterial>);

pub struct SpacePlugin;

impl Plugin for SpacePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "space.wgsl");
        app.add_plugins(Material2dPlugin::<SpaceMaterial>::default())
            .add_systems(Startup, spawn_starfield)
            .add_systems(PostUpdate, follow_camera.before(TransformSystems::Propagate));
    }
}

fn spawn_starfield(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<SpaceMaterial>>,
) {
    let material = materials.add(SpaceMaterial { params: Vec4::ZERO });
    commands.spawn((
        Starfield(material.clone()),
        Mesh2d(meshes.add(Rectangle::new(SIZE, SIZE))),
        MeshMaterial2d(material),
        Transform::from_xyz(0.0, 0.0, SPACE_Z),
    ));
}

/// Keep the stars under the camera and tell them where it is and what time it is.
fn follow_camera(
    time: Res<Time>,
    camera: Query<&Transform, (With<WorldCamera>, Without<Starfield>)>,
    mut fields: Query<(&Starfield, &mut Transform)>,
    mut materials: ResMut<Assets<SpaceMaterial>>,
) {
    let Ok(camera) = camera.single() else { return };
    let at = camera.translation.truncate();
    for (field, mut transform) in &mut fields {
        transform.translation = at.extend(SPACE_Z);
        if let Some(mut material) = materials.get_mut(&field.0) {
            material.params = Vec4::new(at.x, at.y, time.elapsed_secs(), 0.0);
        }
    }
}
