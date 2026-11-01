//! Putting the world image on the screen (`scaling.rs` draws it): through
//! `world_view.wesl`, which bends the picture where the air is hot (`heat.rs`).

use bevy::asset::{AssetPath, embedded_asset, embedded_path};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

use crate::heat::HeatMap;
use crate::scaling::SCREEN_PX;
use crate::{TILE_PX, WorldCamera};

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct WorldViewMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub world: Handle<Image>,
    #[texture(2)]
    #[sampler(3)]
    pub heat: Handle<Image>,
    /// Camera centre x and y (world px), seconds, world px across the square.
    #[uniform(4)]
    pub view: Vec4,
    /// Map width and height (tiles), sprite px per tile, unused.
    #[uniform(5)]
    pub map: Vec4,
}

impl WorldViewMaterial {
    pub fn new(world: Handle<Image>, heat: Handle<Image>) -> Self {
        Self {
            world,
            heat,
            view: Vec4::new(0.0, 0.0, 0.0, SCREEN_PX),
            map: Vec4::new(1.0, 1.0, TILE_PX, 0.0),
        }
    }
}

impl UiMaterial for WorldViewMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(AssetPath::from_path_buf(embedded_path!("world_view.wesl")).with_source("embedded"))
    }
}

pub struct WorldViewPlugin;

impl Plugin for WorldViewPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "world_view.wesl");
        app.add_plugins(UiMaterialPlugin::<WorldViewMaterial>::default())
            .add_systems(PostUpdate, update_view);
    }
}

/// Tell the shader where the camera is, what time it is and how big the map is.
fn update_view(
    time: Res<Time>,
    heat: Option<Res<HeatMap>>,
    camera: Query<&Transform, With<WorldCamera>>,
    nodes: Query<&MaterialNode<WorldViewMaterial>>,
    mut materials: ResMut<Assets<WorldViewMaterial>>,
) {
    let Ok(camera) = camera.single() else { return };
    let size = heat.map_or((1, 1, 0), |h| h.size);
    for node in &nodes {
        if let Some(mut material) = materials.get_mut(&node.0) {
            material.view = Vec4::new(
                camera.translation.x,
                camera.translation.y,
                time.elapsed_secs(),
                SCREEN_PX,
            );
            material.map = Vec4::new(size.0 as f32, size.1 as f32, TILE_PX, 0.0);
        }
    }
}
