//! Heat haze (`docs/reference/lighting.md`, Air you can see): the air over fires and the
//! supermatter bends what is behind it. A texture with one texel per tile says how hot each
//! tile is; `world_view.wgsl` reads it when it puts the world on the screen, and moves
//! each pixel of the picture a little, side to side and up and down, where the air is hot
//! and just above it, as hot air rises. Nothing is drawn over the picture: the walls,
//! floor and people behind the heat waver.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use rs13_world::{Appearance, GridPos, MapLayer, TilePos};

use crate::OwnPlayer;

/// How hot a burning tile looks, from 0 to 1.
pub const FIRE_HEAT: f32 = 1.0;
/// How hot the supermatter looks.
pub const SUPERMATTER_HEAT: f32 = 0.7;

/// The heat texture `world_view.wgsl` reads: one texel per tile of our deck, top row first.
#[derive(Resource)]
pub struct HeatMap {
    pub image: Handle<Image>,
    /// Width, height and deck of the map it covers.
    pub size: (u32, u32, i32),
    /// Hot tiles last written, to write again only when they change.
    hot: Vec<(TilePos, u8)>,
}

pub struct HeatPlugin;

impl Plugin for HeatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, make_heat_map)
            .add_systems(Update, update_heat_map);
    }
}

/// How hot this look is, if it heats the air.
pub fn heat_of(appearance: &Appearance) -> Option<f32> {
    match appearance.sprite.as_str() {
        "fire" => Some(FIRE_HEAT),
        "supermatter" => Some(SUPERMATTER_HEAT),
        _ => None,
    }
}

fn heat_image(width: u32, height: u32, data: Vec<u8>) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::R8Unorm,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    image.sampler = bevy::image::ImageSampler::linear();
    image
}

fn make_heat_map(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.insert_resource(HeatMap {
        image: images.add(heat_image(1, 1, vec![0])),
        size: (1, 1, 0),
        hot: Vec::new(),
    });
}

/// The texels for a `width` × `height` deck with these hot tiles, top row first.
pub fn heat_texels(width: u32, height: u32, hot: &[(TilePos, u8)]) -> Vec<u8> {
    let mut data = vec![0; (width * height) as usize];
    for (pos, heat) in hot {
        let (x, y) = (pos.x, pos.y);
        if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
            continue;
        }
        let i = ((height as i32 - 1 - y) * width as i32 + x) as usize;
        data[i] = data[i].max(*heat);
    }
    data
}

/// Write the heat texture again when the deck or its hot tiles change.
fn update_heat_map(
    mut heat: ResMut<HeatMap>,
    mut images: ResMut<Assets<Image>>,
    maps: Query<&MapLayer>,
    own: Query<(&GridPos, Option<&rs13_world::CameraView>), With<OwnPlayer>>,
    things: Query<(&GridPos, &Appearance)>,
) {
    let deck = own.iter().next().map(|(p, view)| view.map_or(p.0, |v| v.0).z);
    let Some(map) = maps
        .iter()
        .find(|m| Some(m.0.z) == deck)
        .or_else(|| maps.iter().min_by_key(|m| m.0.z))
    else {
        return;
    };
    let size = (map.0.width, map.0.height, map.0.z);
    let mut hot: Vec<(TilePos, u8)> = things
        .iter()
        .filter(|(pos, _)| pos.0.z == size.2)
        .filter_map(|(pos, look)| Some((pos.0, (heat_of(look)? * 255.0).round() as u8)))
        .collect();
    hot.sort();
    if heat.size == size && heat.hot == hot {
        return;
    }
    if let Some(mut image) = images.get_mut(&heat.image) {
        *image = heat_image(size.0, size.1, heat_texels(size.0, size.1, &hot));
    }
    heat.size = size;
    heat.hot = hot;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fire_and_the_supermatter_heat_the_air() {
        let look = |sprite: &str| Appearance::new(sprite, "1");
        assert_eq!(heat_of(&look("fire")), Some(FIRE_HEAT));
        assert_eq!(heat_of(&look("supermatter")), Some(SUPERMATTER_HEAT));
        assert_eq!(heat_of(&look("light")), None);
    }

    #[test]
    fn hot_tiles_land_on_their_texel_top_row_first() {
        let texels = heat_texels(3, 2, &[(TilePos::new(2, 0, 0), 255), (TilePos::new(9, 9, 0), 9)]);
        // y = 0 is the bottom row, the texture's second.
        assert_eq!(texels, vec![0, 0, 0, 0, 0, 255]);
    }
}
