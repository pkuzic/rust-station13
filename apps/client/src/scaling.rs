//! Scaling the pixel-art screen to the window without making it ugly.
//!
//! The map square is 16 cells of 32 px, 512 px across, but the window can be any size.
//! Stretching 512 px by a fractional factor with nearest sampling gives pixels of uneven
//! width that shimmer as the view scrolls; smooth sampling blurs everything.
//!
//! Instead, the world is drawn into an image of `k × 512` px with nearest sampling, where
//! `k` is the smallest whole number that makes it at least as large as the square on screen.
//! Every sprite pixel is then exactly `k × k`. That image is shown in the square with smooth
//! sampling, shrinking it by a factor between ½ and 1, which keeps pixels even and only
//! softens their edges by a screen pixel. The HUD draws its icons from a copy of the atlas
//! enlarged `k` times in the same way. The camera and gliding mobs snap to the `1/k` px
//! grid so the pixels stay put while the view moves.

use bevy::asset::RenderAssetUsages;
use bevy::camera::RenderTarget;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureFormat};
use bevy::window::PrimaryWindow;

use crate::WorldCamera;
use crate::hud::SCREEN_CELLS;
use crate::sprites::SpriteAtlas;

/// Sprite pixels across the square screen.
pub const SCREEN_PX: f32 = SCREEN_CELLS * crate::TILE_PX;
/// The largest enlargement drawn. Past it (squares over 2 048 px) the image is stretched a
/// little instead, which smooth sampling handles well.
pub const MAX_SCALE: u32 = 4;

/// How many screen pixels the world image has per sprite pixel.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelScale(pub u32);

impl Default for PixelScale {
    fn default() -> Self {
        Self(1)
    }
}

impl PixelScale {
    /// The scale for a square of `square_px` physical pixels.
    pub fn for_square(square_px: u32) -> Self {
        let k = (square_px as f32 / SCREEN_PX).ceil() as u32;
        Self(k.clamp(1, MAX_SCALE))
    }

    /// Round a world position to the nearest pixel of the world image.
    pub fn snap(self, v: f32) -> f32 {
        let k = self.0 as f32;
        (v * k).round() / k
    }
}

/// The image the world camera draws into.
#[derive(Resource)]
pub struct WorldImage(pub Handle<Image>);

/// The node showing the world image.
#[derive(Component)]
struct WorldView;

pub struct ScalingPlugin;

impl Plugin for ScalingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PixelScale>()
            .add_systems(PreUpdate, fit_scale);
    }
}

fn target_image(k: u32) -> Image {
    let side = SCREEN_PX as u32 * k;
    let mut image = Image::new_target_texture(side, side, TextureFormat::Rgba8UnormSrgb, None);
    image.sampler = ImageSampler::linear();
    image
}

/// The world image and the node showing it: a square as tall as the window on the left,
/// under the HUD. The world camera draws into the image.
pub fn spawn_world_view(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    camera: Single<Entity, With<WorldCamera>>,
) {
    let image = images.add(target_image(1));
    commands
        .entity(*camera)
        .insert(RenderTarget::Image(image.clone().into()));
    commands.spawn((
        WorldView,
        ImageNode::new(image.clone()),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::VMin(100.0),
            height: Val::VMin(100.0),
            ..default()
        },
        GlobalZIndex(-1),
        Pickable::IGNORE,
    ));
    commands.insert_resource(WorldImage(image));
}

/// When the window's square changes scale, redraw the world at the new size and enlarge
/// the HUD's atlas to match.
fn fit_scale(
    window: Single<&Window, With<PrimaryWindow>>,
    mut scale: ResMut<PixelScale>,
    world: Option<Res<WorldImage>>,
    mut images: ResMut<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    atlas: Option<Res<SpriteAtlas>>,
    mut started: Local<bool>,
) {
    let square = window.physical_width().min(window.physical_height()).max(1);
    let new = PixelScale::for_square(square);
    if *scale == new && *started {
        return;
    }
    let (Some(world), Some(atlas)) = (world, atlas) else {
        return;
    };
    *started = true;
    *scale = new;
    if let Some(mut image) = images.get_mut(&world.0) {
        let side = SCREEN_PX as u32 * new.0;
        image.resize(Extent3d {
            width: side,
            height: side,
            depth_or_array_layers: 1,
        });
    }
    atlas.enlarge_ui(new.0, &mut images, &mut layouts);
}

/// `src`, an RGBA image `width` px wide, with every pixel made `k × k`.
pub fn enlarge(src: &[u8], width: u32, k: u32) -> Vec<u8> {
    let (w, k) = (width as usize, k as usize);
    let rows = src.len() / (w * 4);
    let mut out = Vec::with_capacity(src.len() * k * k);
    for y in 0..rows {
        let row = &src[y * w * 4..(y + 1) * w * 4];
        let mut wide = Vec::with_capacity(row.len() * k);
        for px in row.as_chunks::<4>().0 {
            for _ in 0..k {
                wide.extend_from_slice(px);
            }
        }
        for _ in 0..k {
            out.extend_from_slice(&wide);
        }
    }
    out
}

/// A copy of the atlas enlarged `k` times, for smooth sampling in the UI.
pub fn enlarged_image(src: &Image, k: u32) -> Option<Image> {
    let data = src.data.as_ref()?;
    let (w, h) = (src.width(), src.height());
    let mut image = Image::new(
        Extent3d {
            width: w * k,
            height: h * k,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        enlarge(data, w, k),
        src.texture_descriptor.format,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    image.sampler = ImageSampler::linear();
    Some(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scale_is_the_smallest_whole_number_covering_the_square() {
        assert_eq!(PixelScale::for_square(300).0, 1);
        assert_eq!(PixelScale::for_square(512).0, 1);
        assert_eq!(PixelScale::for_square(513).0, 2);
        assert_eq!(PixelScale::for_square(1080).0, 3);
        assert_eq!(PixelScale::for_square(2160).0, MAX_SCALE);
    }

    #[test]
    fn snapping_lands_on_image_pixels() {
        assert_eq!(PixelScale(2).snap(10.3), 10.5);
        assert_eq!(PixelScale(1).snap(10.3), 10.0);
        assert_eq!(PixelScale(4).snap(-3.1), -3.0);
    }

    #[test]
    fn enlarging_repeats_each_pixel() {
        let red = [255, 0, 0, 255];
        let blue = [0, 0, 255, 255];
        let src = [red, blue].concat();
        let out = enlarge(&src, 2, 2);
        let px = |i: usize| &out[i * 4..i * 4 + 4];
        // Two rows of red, red, blue, blue.
        for row in 0..2 {
            assert_eq!(px(row * 4), red);
            assert_eq!(px(row * 4 + 1), red);
            assert_eq!(px(row * 4 + 2), blue);
            assert_eq!(px(row * 4 + 3), blue);
        }
        assert_eq!(out.len(), 8 * 4, "4 x 2 pixels");
    }
}
