//! Normal maps, an experiment (`docs/reference/lighting.md`, Normal maps): sprites catch the
//! light on the side facing it and fall into shade on the side away, without any new art.
//!
//! - **Made from the atlas at start.** Each frame's shape is read from its own pixels: its
//!   edges round off over a few pixels where it meets transparency, and darker pixels sit a
//!   little lower than lighter ones, so panel lines and outlines read as grooves. A frame
//!   that fills its whole cell (floors, walls) gets no rounded edge, so tiles still join.
//! - **Drawn into their own image.** Every map sprite drawn from the atlas gets a copy drawn
//!   from the normal atlas, on its own render layer, which a second camera draws into an
//!   image the size of the world image, before the world camera.
//! - **Used by the darkness.** The light pass (`lighting.rs`) reads each pixel's normal and
//!   the way the light falls on its tile, and shades the light it multiplies over the map.
//!
//! F8 turns it off and on.

use bevy::asset::RenderAssetUsages;
use bevy::camera::RenderTarget;
use bevy::camera::visibility::RenderLayers;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::view::{DebandDither, Msaa, Tonemapping};

use crate::scaling::WorldImage;
use crate::sprites::SpriteAtlas;
use crate::{ChatInput, WorldCamera};

/// The render layer the normal copies are drawn on, seen only by the normal camera.
pub const NORMAL_LAYER: usize = 30;
/// How many pixels in from a frame's edge its rounding reaches.
const BEVEL_PX: f32 = 3.0;
/// How much a pixel's brightness raises it, against the full height of the rounded edge.
const LUMA_HEIGHT: f32 = 0.35;
/// How steep the slopes are made: higher tilts the normals further.
const BUMP: f32 = 3.0;
/// Alpha below which a pixel counts as empty.
const EMPTY_ALPHA: u8 = 128;
/// The normal of a flat surface facing the viewer, as drawn.
const FLAT: LinearRgba = LinearRgba::new(0.5, 0.5, 1.0, 1.0);

/// Whether sprites are shaded by their normals (F8).
#[derive(Resource)]
pub struct NormalsOn(pub bool);

/// The normal atlas, and a copy for sprites drawn mirrored, whose normals point the other
/// way across.
#[derive(Resource)]
struct NormalAtlas {
    image: Handle<Image>,
    mirrored: Handle<Image>,
}

/// What the normal camera draws into: the normal of every pixel of the world image.
#[derive(Resource)]
pub struct NormalImage(pub Handle<Image>);

#[derive(Component)]
struct NormalCamera;

/// On a normal copy.
#[derive(Component)]
struct NormalCopy;

/// On a sprite with a normal copy.
#[derive(Component)]
struct HasNormal;

pub struct NormalsPlugin;

impl Plugin for NormalsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(NormalsOn(true))
            .add_systems(PostStartup, (make_normal_atlas, spawn_normal_camera))
            .add_systems(
                Update,
                (toggle, add_copies, follow, fit_size)
                    .chain()
                    .after(crate::sprites::animate),
            );
    }
}

/// The normal map of an RGBA atlas `width` × `height` px in cells of `tile` px: per pixel,
/// the normal as RGB (x east, y north, z towards the viewer, each from 1 to 255, 128 for
/// none) and the
/// pixel's own alpha. Each cell is worked out on its own, so frames don't bleed together.
pub fn normal_map(rgba: &[u8], width: u32, height: u32, tile: u32) -> Vec<u8> {
    let (w, t) = (width as usize, tile as usize);
    let mut out = vec![0u8; rgba.len()];
    let alpha = |x: usize, y: usize| rgba.get((y * w + x) * 4 + 3).copied().unwrap_or(0);
    for cy in (0..height as usize).step_by(t) {
        for cx in (0..w).step_by(t) {
            let (cw, ch) = (t.min(w - cx), t.min(height as usize - cy));
            let solid = |x: usize, y: usize| alpha(cx + x, cy + y) >= EMPTY_ALPHA;
            if !(0..ch).any(|y| (0..cw).any(|x| solid(x, y))) {
                continue;
            }
            let distance = edge_distance(cw, ch, solid);
            let height_at: Vec<f32> = (0..ch)
                .flat_map(|y| (0..cw).map(move |x| (x, y)))
                .map(|(x, y)| {
                    if !solid(x, y) {
                        return 0.0;
                    }
                    let i = ((cy + y) * w + cx + x) * 4;
                    let luma =
                        (0.299 * rgba[i] as f32 + 0.587 * rgba[i + 1] as f32 + 0.114 * rgba[i + 2] as f32)
                            / 255.0;
                    (distance[y * cw + x] / BEVEL_PX).min(1.0) + LUMA_HEIGHT * luma
                })
                .collect();
            // Outside the cell, the nearest pixel inside it.
            let h = |x: isize, y: isize| {
                let x = x.clamp(0, cw as isize - 1) as usize;
                let y = y.clamp(0, ch as isize - 1) as usize;
                height_at[y * cw + x]
            };
            for y in 0..ch {
                for x in 0..cw {
                    let (xi, yi) = (x as isize, y as isize);
                    // Sobel slopes, east and down the image.
                    let gx = (h(xi + 1, yi - 1) + 2.0 * h(xi + 1, yi) + h(xi + 1, yi + 1)
                        - h(xi - 1, yi - 1)
                        - 2.0 * h(xi - 1, yi)
                        - h(xi - 1, yi + 1))
                        / 8.0;
                    let gy = (h(xi - 1, yi + 1) + 2.0 * h(xi, yi + 1) + h(xi + 1, yi + 1)
                        - h(xi - 1, yi - 1)
                        - 2.0 * h(xi, yi - 1)
                        - h(xi + 1, yi - 1))
                        / 8.0;
                    // Rising eastwards faces west; rising down the image faces north.
                    let n = Vec3::new(-gx * BUMP, gy * BUMP, 1.0).normalize();
                    let i = ((cy + y) * w + cx + x) * 4;
                    let encode = |v: f32| (v * 127.0 + 128.0).round().clamp(1.0, 255.0) as u8;
                    out[i] = encode(n.x);
                    out[i + 1] = encode(n.y);
                    out[i + 2] = encode(n.z);
                    out[i + 3] = rgba[i + 3];
                }
            }
        }
    }
    out
}

/// How far each pixel of a `w` × `h` cell is from the nearest empty pixel in it, in pixels
/// (chamfer distance). Past the cell's border counts as solid, so a frame that fills its
/// cell has no edge.
fn edge_distance(w: usize, h: usize, solid: impl Fn(usize, usize) -> bool) -> Vec<f32> {
    const FAR: f32 = 1e6;
    const DIAGONAL: f32 = std::f32::consts::SQRT_2;
    let mut d: Vec<f32> = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| if solid(x, y) { FAR } else { 0.0 })
        .collect();
    let at = |d: &[f32], x: isize, y: isize| {
        if x < 0 || y < 0 || x >= w as isize || y >= h as isize {
            FAR
        } else {
            d[y as usize * w + x as usize]
        }
    };
    for y in 0..h as isize {
        for x in 0..w as isize {
            let i = y as usize * w + x as usize;
            let best = [
                at(&d, x - 1, y) + 1.0,
                at(&d, x, y - 1) + 1.0,
                at(&d, x - 1, y - 1) + DIAGONAL,
                at(&d, x + 1, y - 1) + DIAGONAL,
            ]
            .into_iter()
            .fold(d[i], f32::min);
            d[i] = best;
        }
    }
    for y in (0..h as isize).rev() {
        for x in (0..w as isize).rev() {
            let i = y as usize * w + x as usize;
            let best = [
                at(&d, x + 1, y) + 1.0,
                at(&d, x, y + 1) + 1.0,
                at(&d, x + 1, y + 1) + DIAGONAL,
                at(&d, x - 1, y + 1) + DIAGONAL,
            ]
            .into_iter()
            .fold(d[i], f32::min);
            d[i] = best;
        }
    }
    d
}

/// A normal map mirrored left to right turns its normals the other way across.
fn mirror_normals(normals: &[u8]) -> Vec<u8> {
    let mut out = normals.to_vec();
    for px in out.as_chunks_mut::<4>().0 {
        if px[3] > 0 {
            px[0] = (256 - u16::from(px[0])).min(255) as u8;
        }
    }
    out
}

fn normal_image(data: Vec<u8>, width: u32, height: u32) -> Image {
    let mut image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        // Linear: the numbers are directions, not colours.
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    image
}

fn make_normal_atlas(mut commands: Commands, atlas: Res<SpriteAtlas>, mut images: ResMut<Assets<Image>>) {
    let src = atlas.source();
    let Some(data) = src.data.as_ref() else {
        return;
    };
    let (width, height) = (src.width(), src.height());
    let normals = normal_map(data, width, height, atlas.atlas.tile);
    let mirrored = mirror_normals(&normals);
    commands.insert_resource(NormalAtlas {
        image: images.add(normal_image(normals, width, height)),
        mirrored: images.add(normal_image(mirrored, width, height)),
    });
}

fn target_image(side: u32) -> Image {
    let mut image = Image::new_target_texture(side, side, TextureFormat::Rgba8Unorm, None);
    image.sampler = ImageSampler::nearest();
    image
}

/// The normal camera: a child of the world camera, so it sees exactly what that sees, drawing
/// only the normal copies, into an image the world image's size, before the world camera.
fn spawn_normal_camera(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    world: Option<Res<WorldImage>>,
    camera: Single<(Entity, &Projection), With<WorldCamera>>,
) {
    let side = world
        .and_then(|w| images.get(&w.0).map(|i| i.width()))
        .unwrap_or(crate::scaling::SCREEN_PX as u32);
    let image = images.add(target_image(side));
    let (world_camera, projection) = *camera;
    commands.entity(world_camera).with_child((
        Camera2d,
        Camera {
            order: -2,
            clear_color: ClearColorConfig::Custom(Color::LinearRgba(FLAT)),
            ..default()
        },
        RenderTarget::Image(image.clone().into()),
        projection.clone(),
        Tonemapping::None,
        DebandDither::Disabled,
        Msaa::Off,
        RenderLayers::layer(NORMAL_LAYER),
        Transform::default(),
        NormalCamera,
    ));
    commands.insert_resource(NormalImage(image));
}

fn toggle(
    keys: Res<ButtonInput<KeyCode>>,
    chat: Res<ChatInput>,
    mut on: ResMut<NormalsOn>,
    mut cameras: Query<&mut Camera, With<NormalCamera>>,
) {
    if !chat.open && keys.just_pressed(KeyCode::F8) {
        on.0 = !on.0;
    }
    for mut camera in &mut cameras {
        if camera.is_active != on.0 {
            camera.is_active = on.0;
        }
    }
}

/// Give every map sprite drawn from the atlas a copy drawn from the normal atlas.
#[allow(clippy::type_complexity)]
fn add_copies(
    mut commands: Commands,
    atlas: Option<Res<SpriteAtlas>>,
    normals: Option<Res<NormalAtlas>>,
    sprites: Query<
        (Entity, &Sprite),
        (
            Without<HasNormal>,
            Without<NormalCopy>,
            Without<crate::glow::GlowOf>,
        ),
    >,
) {
    let (Some(atlas), Some(normals)) = (atlas, normals) else {
        return;
    };
    let source = atlas.image();
    for (entity, sprite) in &sprites {
        if sprite.image != source || sprite.texture_atlas.is_none() {
            continue;
        }
        let mut copy = sprite.clone();
        copy.image = if sprite.flip_x {
            normals.mirrored.clone()
        } else {
            normals.image.clone()
        };
        copy.color = Color::WHITE.with_alpha(sprite.color.alpha());
        commands.entity(entity).insert(HasNormal).with_child((
            copy,
            Transform::default(),
            RenderLayers::layer(NORMAL_LAYER),
            NormalCopy,
        ));
    }
}

/// Keep each copy on its sprite's frame, facing and see-through-ness.
fn follow(
    normals: Option<Res<NormalAtlas>>,
    sources: Query<&Sprite, Without<NormalCopy>>,
    mut copies: Query<(&ChildOf, &mut Sprite), With<NormalCopy>>,
) {
    let Some(normals) = normals else { return };
    for (parent, mut copy) in &mut copies {
        let Ok(source) = sources.get(parent.parent()) else {
            continue;
        };
        if let (Some(from), Some(to)) = (source.texture_atlas.as_ref(), copy.texture_atlas.as_ref())
            && to.index != from.index
            && let Some(to) = copy.texture_atlas.as_mut()
        {
            to.index = from.index;
        }
        if copy.flip_x != source.flip_x {
            copy.flip_x = source.flip_x;
            copy.image = if source.flip_x {
                normals.mirrored.clone()
            } else {
                normals.image.clone()
            };
        }
        let alpha = source.color.alpha();
        if (copy.color.alpha() - alpha).abs() > 1e-3 {
            copy.color = Color::WHITE.with_alpha(alpha);
        }
        if copy.custom_size != source.custom_size {
            copy.custom_size = source.custom_size;
        }
    }
}

/// Keep the normal image the size of the world image, which follows the window
/// (`scaling.rs`).
fn fit_size(
    world: Option<Res<WorldImage>>,
    normal: Option<Res<NormalImage>>,
    mut images: ResMut<Assets<Image>>,
) {
    let (Some(world), Some(normal)) = (world, normal) else {
        return;
    };
    let Some(size) = images.get(&world.0).map(|i| i.texture_descriptor.size) else {
        return;
    };
    let current = images.get(&normal.0).map(|i| i.texture_descriptor.size);
    if current != Some(size)
        && let Some(mut image) = images.get_mut(&normal.0)
    {
        image.resize(size);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 4 × 4 atlas of one cell: `solid` pixels grey, the rest empty.
    fn cell(solid: impl Fn(usize, usize) -> bool) -> Vec<u8> {
        (0..4)
            .flat_map(|y| (0..4).map(move |x| (x, y)))
            .flat_map(|(x, y)| if solid(x, y) { [128, 128, 128, 255] } else { [0; 4] })
            .collect()
    }

    fn normal(map: &[u8], x: usize, y: usize) -> [u8; 4] {
        let i = (y * 4 + x) * 4;
        [map[i], map[i + 1], map[i + 2], map[i + 3]]
    }

    #[test]
    fn a_frame_filling_its_cell_is_flat() {
        let map = normal_map(&cell(|_, _| true), 4, 4, 4);
        for y in 0..4 {
            for x in 0..4 {
                assert_eq!(normal(&map, x, y), [128, 128, 255, 255], "{x},{y}");
            }
        }
    }

    #[test]
    fn a_shapes_edges_face_outwards() {
        // A 2 × 2 block in the middle of the cell.
        let map = normal_map(&cell(|x, y| (1..3).contains(&x) && (1..3).contains(&y)), 4, 4, 4);
        let left = normal(&map, 1, 1);
        let right = normal(&map, 2, 2);
        assert!(left[0] < 128, "the left edge faces west: {left:?}");
        assert!(left[1] > 128, "the top edge faces north: {left:?}");
        assert!(right[0] > 128, "the right edge faces east: {right:?}");
        assert!(right[1] < 128, "the bottom edge faces south: {right:?}");
        assert_eq!(normal(&map, 0, 0)[3], 0, "empty pixels stay empty");
    }

    #[test]
    fn mirroring_turns_normals_the_other_way_across() {
        let map = normal_map(&cell(|x, y| (1..3).contains(&x) && (1..3).contains(&y)), 4, 4, 4);
        let mirrored = mirror_normals(&map);
        assert_eq!(
            u16::from(normal(&mirrored, 1, 1)[0]),
            256 - u16::from(normal(&map, 1, 1)[0])
        );
        assert_eq!(normal(&mirrored, 1, 1)[1], normal(&map, 1, 1)[1]);
    }

    #[test]
    fn cells_do_not_bleed_into_each_other() {
        // Two 4 × 4 cells side by side: the left full, the right empty.
        let atlas: Vec<u8> = (0..4)
            .flat_map(|_| (0..8).map(|x| if x < 4 { [128, 128, 128, 255] } else { [0; 4] }))
            .flatten()
            .collect();
        let map = normal_map(&atlas, 8, 4, 4);
        let i = (4 - 1) * 4;
        assert_eq!(
            &map[i..i + 3],
            &[128, 128, 255],
            "the full cell's right edge is flat"
        );
    }
}
