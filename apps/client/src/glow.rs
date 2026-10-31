//! Things that give off their own light stay bright in the dark and glow
//! (`docs/reference/lighting.md`, Glow): screens and status lights, lit lamps, fire, laser
//! beams, the supermatter.
//!
//! The darkness is multiplied over everything on the map, so a glowing part is drawn a
//! second time above it, from a copy of the atlas that keeps only that part's pixels,
//! brighter than full white so the world camera's bloom picks it up. The sight shadows
//! still cover it.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension};
use rs13_world::{Appearance, LightSource};

use crate::sprites::{Look, OverlayPart, SpriteAtlas};

/// Over the darkness (`lighting.rs`, 20), under the view cone and sight shadows.
const GLOW_Z: f32 = 20.2;
/// How far past full white glowing pixels are drawn: what the bloom spreads.
const GLOW_BOOST: f32 = 1.6;

/// Which pixels of a sprite give off light.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlowKind {
    /// All of it: fire, a laser beam, the supermatter.
    Whole,
    /// Its bright, saturated pixels: a screen, status lights.
    Screen,
    /// Its bright pixels, while it is lit (has a `LightSource`): lamps.
    Lamp,
}

/// What glows, by sprite id.
pub fn glow_kind(sprite: &str) -> Option<GlowKind> {
    Some(match sprite {
        "fire" | "laser-beam" | "supermatter" => GlowKind::Whole,
        "light" | "light-small" | "desk-lamp" | "desk-lamp-green" | "flashlight" | "penlight" | "lantern" => {
            GlowKind::Lamp
        }
        "power-monitor" | "laser-computer" | "intercom" | "smes" | "apc" | "cell-charger" | "fire-alarm"
        | "air-alarm" | "teg" | "freezer" | "engine-laser" | "meter" | "pacman" | "light-switch" => {
            GlowKind::Screen
        }
        _ => return None,
    })
}

/// Whether a pixel is part of what glows: bright and strongly coloured for a screen
/// (indicator greens, reds and blues, not grey metal), or just bright for a lamp.
pub fn glows(kind: GlowKind, rgba: [u8; 4]) -> bool {
    if rgba[3] == 0 {
        return false;
    }
    let max = rgba[..3].iter().copied().max().unwrap_or(0) as f32 / 255.0;
    let min = rgba[..3].iter().copied().min().unwrap_or(0) as f32 / 255.0;
    let saturation = if max > 0.0 { (max - min) / max } else { 0.0 };
    match kind {
        GlowKind::Whole => true,
        GlowKind::Screen => max >= 0.55 && saturation >= 0.45,
        GlowKind::Lamp => max >= 0.8 || (max >= 0.55 && saturation >= 0.45),
    }
}

/// Copies of the atlas holding only the glowing pixels, per kind, and the faint light each
/// glowing screen casts.
#[derive(Resource)]
pub struct GlowAtlas {
    screen: Handle<Image>,
    lamp: Handle<Image>,
    /// `(sprite, state)` → the colour of its glowing pixels, averaged over its frames.
    colours: HashMap<(String, String), [f32; 3]>,
}

/// How far a glowing screen's own light reaches, in tiles: a faint pool round it.
pub const SCREEN_LIGHT_REACH: f32 = 2.0;
/// Fewer glowing pixels than this cast no light: a speck of a status light.
const MIN_GLOWING_PIXELS: usize = 6;

impl GlowAtlas {
    /// The light a glowing screen casts, as strengths in tiles of reach per colour, for its
    /// sprite and state; `None` if it doesn't glow, or barely.
    pub fn screen_light(&self, sprite: &str, state: &str) -> Option<[f32; 3]> {
        let colour = self
            .colours
            .get(&(sprite.to_owned(), state.to_owned()))
            .or_else(|| self.colours.get(&(sprite.to_owned(), String::new())))?;
        let top = colour.iter().copied().fold(0.0_f32, f32::max);
        (top > 0.0).then(|| colour.map(|c| c / top * SCREEN_LIGHT_REACH))
    }
}

/// The average colour of the glowing pixels of every frame of every screen's states.
fn screen_colours(atlas: &SpriteAtlas) -> HashMap<(String, String), [f32; 3]> {
    let src = atlas.source();
    let Some(data) = src.data.as_ref() else {
        return HashMap::new();
    };
    let (width, tile, columns) = (
        src.width() as usize,
        atlas.atlas.tile as usize,
        atlas.atlas.columns as usize,
    );
    let mut out = HashMap::new();
    for (sprite, states) in &atlas.atlas.sprites {
        if glow_kind(sprite) != Some(GlowKind::Screen) {
            continue;
        }
        for (state, anim) in states {
            let (mut sum, mut count) = ([0.0_f32; 3], 0usize);
            for cell in &anim.cells {
                let (cx, cy) = (
                    (*cell as usize % columns) * tile,
                    (*cell as usize / columns) * tile,
                );
                for y in cy..cy + tile {
                    for x in cx..cx + tile {
                        let i = (y * width + x) * 4;
                        let Some(px) = data.get(i..i + 4) else { continue };
                        let px = [px[0], px[1], px[2], px[3]];
                        if glows(GlowKind::Screen, px) {
                            for c in 0..3 {
                                sum[c] += px[c] as f32 / 255.0;
                            }
                            count += 1;
                        }
                    }
                }
            }
            let frames = anim.cells.len().max(1);
            if count / frames >= MIN_GLOWING_PIXELS {
                out.insert((sprite.clone(), state.clone()), sum.map(|s| s / count as f32));
            }
        }
    }
    out
}

/// On a glowing copy: the sprite it shines over.
#[derive(Component)]
struct GlowOf;

/// On a sprite with a glowing copy.
#[derive(Component)]
struct Glowing;

pub struct GlowPlugin;

impl Plugin for GlowPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, make_glow_atlas)
            .add_systems(Update, (add_glows, follow).chain().after(crate::sprites::animate));
    }
}

fn masked(src: &Image, kind: GlowKind) -> Option<Image> {
    let data = src.data.as_ref()?;
    let out: Vec<u8> = data
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|px| if glows(kind, *px) { *px } else { [0; 4] })
        .collect();
    let mut image = Image::new(
        Extent3d {
            width: src.width(),
            height: src.height(),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        out,
        src.texture_descriptor.format,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    Some(image)
}

fn make_glow_atlas(mut commands: Commands, atlas: Res<SpriteAtlas>, mut images: ResMut<Assets<Image>>) {
    let (Some(screen), Some(lamp)) = (
        masked(atlas.source(), GlowKind::Screen),
        masked(atlas.source(), GlowKind::Lamp),
    ) else {
        return;
    };
    commands.insert_resource(GlowAtlas {
        screen: images.add(screen),
        lamp: images.add(lamp),
        colours: screen_colours(&atlas),
    });
}

/// Give each glowing sprite, and each overlay on one (a SMES's charge lights), a glowing
/// copy.
#[allow(clippy::type_complexity)]
fn add_glows(
    mut commands: Commands,
    atlas: Option<Res<SpriteAtlas>>,
    glow: Option<Res<GlowAtlas>>,
    things: Query<(Entity, &Appearance, &Sprite), (With<Look>, Without<Glowing>)>,
    parts: Query<(Entity, &ChildOf, &Sprite), (With<OverlayPart>, Without<Glowing>)>,
    appearances: Query<&Appearance>,
) {
    let (Some(atlas), Some(glow)) = (atlas, glow) else {
        return;
    };
    let image = |kind: GlowKind| match kind {
        GlowKind::Whole => atlas.image(),
        GlowKind::Screen => glow.screen.clone(),
        GlowKind::Lamp => glow.lamp.clone(),
    };
    let mut add = |entity: Entity, kind: GlowKind, sprite: &Sprite| {
        let Some(texture) = sprite.texture_atlas.clone() else {
            return;
        };
        let mut copy = Sprite::from_atlas_image(image(kind), texture);
        copy.color = Color::LinearRgba(LinearRgba::rgb(GLOW_BOOST, GLOW_BOOST, GLOW_BOOST));
        copy.flip_x = sprite.flip_x;
        commands
            .entity(entity)
            .insert(Glowing)
            .with_child((copy, Transform::default(), GlowOf));
    };
    for (entity, appearance, sprite) in &things {
        if let Some(kind) = glow_kind(&appearance.sprite) {
            add(entity, kind, sprite);
        }
    }
    for (entity, parent, sprite) in &parts {
        let kind = appearances
            .get(parent.parent())
            .ok()
            .and_then(|a| glow_kind(&a.sprite));
        if let Some(kind) = kind {
            // An overlay is a lit part of its machine: all of it that is bright counts.
            add(
                entity,
                if kind == GlowKind::Whole {
                    kind
                } else {
                    GlowKind::Lamp
                },
                sprite,
            );
        }
    }
}

/// Keep each glowing copy on its sprite's current frame, over the darkness, and dark when a
/// lamp is off.
#[allow(clippy::type_complexity)]
fn follow(
    sources: Query<
        (
            &Sprite,
            &GlobalTransform,
            Option<&Appearance>,
            Option<&ChildOf>,
            Has<LightSource>,
        ),
        Without<GlowOf>,
    >,
    appearances: Query<(&Appearance, Has<LightSource>)>,
    mut glows: Query<(&ChildOf, &mut Sprite, &mut Transform, &mut Visibility), With<GlowOf>>,
) {
    for (parent, mut sprite, mut transform, mut visibility) in &mut glows {
        let Ok((source, global, appearance, grandparent, lit)) = sources.get(parent.parent()) else {
            continue;
        };
        if let (Some(from), Some(to)) = (source.texture_atlas.as_ref(), sprite.texture_atlas.as_mut())
            && to.index != from.index
        {
            to.index = from.index;
        }
        sprite.flip_x = source.flip_x;
        let z = GLOW_Z - global.translation().z;
        if (transform.translation.z - z).abs() > 1e-4 {
            transform.translation.z = z;
        }
        // A lamp glows only while lit; an overlay follows its machine.
        let (look, lit) = match appearance {
            Some(a) => (Some(a), lit),
            None => grandparent
                .and_then(|g| appearances.get(g.parent()).ok())
                .map_or((None, lit), |(a, l)| (Some(a), l)),
        };
        let on = look.and_then(|a| glow_kind(&a.sprite)) != Some(GlowKind::Lamp) || lit;
        visibility.set_if_neq(if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screens_glow_in_their_coloured_lights_not_their_metal() {
        assert!(glows(GlowKind::Screen, [40, 220, 60, 255]), "a green light");
        assert!(!glows(GlowKind::Screen, [170, 170, 175, 255]), "grey metal");
        assert!(!glows(GlowKind::Screen, [40, 90, 40, 255]), "a dark green");
        assert!(!glows(GlowKind::Screen, [40, 220, 60, 0]), "transparent");
        assert!(glows(GlowKind::Lamp, [250, 250, 230, 255]), "a lit tube");
        assert!(glows(GlowKind::Whole, [90, 40, 10, 255]));
    }

    #[test]
    fn what_glows() {
        assert_eq!(glow_kind("fire"), Some(GlowKind::Whole));
        assert_eq!(glow_kind("light"), Some(GlowKind::Lamp));
        assert_eq!(glow_kind("apc"), Some(GlowKind::Screen));
        assert_eq!(glow_kind("crowbar"), None);
    }
}
