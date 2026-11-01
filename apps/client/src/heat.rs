//! Heat shimmer over fires (`docs/reference/lighting.md`, Fire): a quad over each burning
//! tile, reaching a tile above it, where `heat.wgsl` draws faint warm bands that waver and
//! rise. Added to what is under it, so it reads as hot air over the flames.

use bevy::asset::{AssetPath, embedded_asset, embedded_path};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState, RenderPipelineDescriptor,
    SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dKey, Material2dPipeline, Material2dPlugin};
use rs13_world::{Appearance, GridPos};

use crate::TILE_PX;

/// Over the fire (3.2), under wall lights (3.5).
const HEAT_Z: f32 = 3.3;
/// How far above its tile the shimmer reaches, in tiles.
const RISE: f32 = 1.0;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct HeatMaterial {
    /// Seconds, the fire's own offset, unused, unused.
    #[uniform(0)]
    params: Vec4,
}

impl Material2d for HeatMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(AssetPath::from_path_buf(embedded_path!("heat.wgsl")).with_source("embedded"))
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }

    fn specialize(
        _pipeline: &Material2dPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let add = BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent::OVER,
        };
        if let Some(fragment) = descriptor.fragment.as_mut() {
            for target in fragment.targets.iter_mut().flatten() {
                target.blend = Some(add);
            }
        }
        Ok(())
    }
}

/// On a fire that has its shimmer.
#[derive(Component)]
struct Shimmering;

/// A fire's shimmer.
#[derive(Component)]
struct Shimmer(Handle<HeatMaterial>);

#[derive(Resource)]
struct ShimmerMesh(Handle<Mesh>);

pub struct HeatPlugin;

impl Plugin for HeatPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "heat.wgsl");
        app.add_plugins(Material2dPlugin::<HeatMaterial>::default())
            .add_systems(Startup, make_mesh)
            .add_systems(Update, (add_shimmer, tick));
    }
}

/// Whether this look is burning air.
pub fn is_fire(appearance: &Appearance) -> bool {
    appearance.sprite == "fire"
}

fn make_mesh(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(ShimmerMesh(
        meshes.add(Rectangle::new(TILE_PX, TILE_PX * (1.0 + RISE))),
    ));
}

/// Give each fire a shimmer. It goes with the fire when the fire goes out.
fn add_shimmer(
    mut commands: Commands,
    mesh: Res<ShimmerMesh>,
    mut materials: ResMut<Assets<HeatMaterial>>,
    fires: Query<(Entity, &Appearance, &GridPos), Without<Shimmering>>,
) {
    for (entity, appearance, pos) in &fires {
        if !is_fire(appearance) {
            continue;
        }
        let seed = (pos.0.x * 7 + pos.0.y * 13) as f32 * 0.37;
        let material = materials.add(HeatMaterial {
            params: Vec4::new(0.0, seed, 0.0, 0.0),
        });
        // Placed relative to the fire: up by half the rise, over it.
        commands.entity(entity).insert(Shimmering).with_child((
            Shimmer(material.clone()),
            Mesh2d(mesh.0.clone()),
            MeshMaterial2d(material),
            Transform::from_xyz(0.0, TILE_PX * RISE / 2.0, HEAT_Z - 3.2),
        ));
    }
}

fn tick(time: Res<Time>, shimmers: Query<&Shimmer>, mut materials: ResMut<Assets<HeatMaterial>>) {
    for shimmer in &shimmers {
        if let Some(mut material) = materials.get_mut(&shimmer.0) {
            material.params.x = time.elapsed_secs();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_fire_shimmers() {
        let look = |sprite: &str| Appearance::new(sprite, "1");
        assert!(is_fire(&look("fire")));
        assert!(!is_fire(&look("light")));
    }
}
