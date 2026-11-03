//! Depth for what stands on the floor (`docs/reference/lighting.md`, Depth):
//!
//! - **Contact shadows:** people and what stands on the floor have a soft dark oval on the
//!   floor under their lowest visible pixels, so they stand on it instead of floating over
//!   it. It is measured from the sprite's own pixels, so it sits under the feet whatever
//!   the sprite's shape, and it points nowhere, so no light direction can be wrong. Small
//!   things lying on the floor and fixtures on walls have none. All shadows lie in one
//!   layer, just over the floor and what lies flat on it and under everything that stands,
//!   so a shadow never falls over another object.
//! - **Silhouettes:** someone standing in the dark is seen as a black shape: you can tell
//!   someone is there, not who. The server sends only the shape.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use rs13_world::{Appearance, PixelOffset, Silhouette};

use crate::TILE_PX;
use crate::sprites::{Footprint, Layer, Look, SpriteAtlas};

/// How dark a shadow is at its middle.
const SHADOW_ALPHA: f32 = 0.6;
/// The layer every shadow lies on: over pipes and cables (up to 0.75), under tables and
/// closets (0.9) and everything that stands.
const SHADOW_Z: f32 = 0.8;
/// A frame shorter than this, px, is something lying on the floor: it casts no shadow.
const STANDING_PX: u32 = 14;
/// How far a shadow reaches past its object's sides, px.
const SPREAD_PX: f32 = 3.0;
/// A shadow's height for each pixel of its width, within the bounds below, px.
const FLATNESS: f32 = 0.34;
const MIN_HEIGHT_PX: f32 = 4.0;
const MAX_HEIGHT_PX: f32 = 10.0;
/// How far the shadow's middle sits above the lowest visible pixel, px: the feet stand in
/// it, not on its edge.
const TUCK_PX: f32 = 1.0;
/// The shadow image's size, px; it is stretched to each object.
const BLOB_W: u32 = 64;
const BLOB_H: u32 = 16;

/// What may cast a contact shadow: not what lies flat on the floor, hangs on a wall, or is
/// light itself. Frames too short to be standing are left out as well (`STANDING_PX`).
pub fn casts_shadow(sprite: &str) -> bool {
    !matches!(
        sprite,
        "pipe"
            | "cable"
            | "manifold"
            | "vent"
            | "scrubber"
            | "pump"
            | "valve"
            | "connector"
            | "gas-filter"
            | "gas-mixer"
            | "crematorium-igniter"
            | "shield"
            | "shield-plate"
            | "turret-control"
            | "shot"
            | "door-timer"
            | "airlock-console"
            | "access-console"
            | "access-button"
            | "flasher"
            | "navbeacon"
            | "shields-button"
            | "injector"
            | "meter"
            | "pipe-vent"
            | "airlock"
            | "firedoor"
            | "blast-door"
            | "window-pane"
            | "stairs"
            | "status-display"
            | "status-glyph"
            | "disposal-pipe"
            | "door-button"
            | "light"
            | "light-small"
            | "light-switch"
            | "ignition-switch"
            | "sparker"
            | "sparks"
            | "igniter"
            | "apc"
            | "air-alarm"
            | "fire-alarm"
            | "intercom"
            | "fire"
            | "laser-beam"
            | "floor-tile"
            | "terminal"
            | "thermal-plate"
            | "generator-input"
            | "gas"
    )
}

/// Where a contact shadow goes and how big it is, for a frame with `footprint`, relative
/// to the middle of its cell, px. `None` for frames too short to be standing.
pub fn contact_shadow(footprint: Footprint, flip_x: bool) -> Option<(Vec2, Vec2)> {
    if footprint.bottom - footprint.top + 1 < STANDING_PX {
        return None;
    }
    let half = TILE_PX / 2.0;
    let width = (footprint.right - footprint.left + 1) as f32 + 2.0 * SPREAD_PX;
    let height = (width * FLATNESS).clamp(MIN_HEIGHT_PX, MAX_HEIGHT_PX);
    let mut x = (footprint.left + footprint.right + 1) as f32 / 2.0 - half;
    if flip_x {
        x = -x;
    }
    // The bottom edge of the lowest visible row, in cell coordinates with y up.
    let foot = half - (footprint.bottom + 1) as f32;
    Some((Vec2::new(x, foot + TUCK_PX), Vec2::new(width, height)))
}

/// A soft dark oval, darkest in the middle, fading to nothing at its edge.
fn blob_image() -> Image {
    let mut data = Vec::with_capacity((BLOB_W * BLOB_H * 4) as usize);
    for y in 0..BLOB_H {
        for x in 0..BLOB_W {
            let dx = (x as f32 + 0.5) / BLOB_W as f32 * 2.0 - 1.0;
            let dy = (y as f32 + 0.5) / BLOB_H as f32 * 2.0 - 1.0;
            let a = (1.0 - (dx * dx + dy * dy)).clamp(0.0, 1.0);
            let a = a * a * (3.0 - 2.0 * a);
            data.extend_from_slice(&[0, 0, 0, (a * 255.0).round() as u8]);
        }
    }
    Image::new(
        Extent3d {
            width: BLOB_W,
            height: BLOB_H,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

#[derive(Resource)]
struct Blob(Handle<Image>);

/// On a contact shadow, with the frame it was measured for.
#[derive(Component)]
struct ContactShadow {
    index: usize,
    flip_x: bool,
}

/// On a sprite that has its contact shadow.
#[derive(Component)]
struct Shadowed;

pub struct SpriteShadowsPlugin;

impl Plugin for SpriteShadowsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, make_blob).add_systems(
            Update,
            (add_shadows, follow, silhouettes)
                .chain()
                .after(crate::sprites::animate),
        );
    }
}

fn make_blob(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.insert_resource(Blob(images.add(blob_image())));
}

/// Give each sprite that may stand on the floor its contact shadow. A person's shadow is
/// their body's: what they wear barely changes where their feet are.
#[allow(clippy::type_complexity)]
fn add_shadows(
    mut commands: Commands,
    blob: Res<Blob>,
    things: Query<
        (Entity, &Sprite, &Appearance),
        (
            With<Look>,
            Without<Shadowed>,
            Without<PixelOffset>,
            Without<ContactShadow>,
            Without<Layer>,
            Without<rs13_world::Ghost>,
        ),
    >,
) {
    for (entity, sprite, appearance) in &things {
        if !casts_shadow(&appearance.sprite) || sprite.texture_atlas.is_none() {
            continue;
        }
        let mut shadow = Sprite::from_image(blob.0.clone());
        shadow.color = Color::srgba(1.0, 1.0, 1.0, SHADOW_ALPHA);
        // Measured and placed by `follow`; hidden until then.
        commands.entity(entity).insert(Shadowed).with_child((
            shadow,
            Transform::default(),
            Visibility::Hidden,
            ContactShadow {
                index: usize::MAX,
                flip_x: false,
            },
        ));
    }
}

/// Measure each shadow again when its object's frame changes (it walks, turns, opens), and
/// keep it on the shadow layer when its object changes layer (a chair turned north).
#[allow(clippy::type_complexity)]
fn follow(
    atlas: Option<Res<SpriteAtlas>>,
    sources: Query<(&Sprite, &Transform), Without<ContactShadow>>,
    mut shadows: Query<(
        &ChildOf,
        &mut ContactShadow,
        &mut Sprite,
        &mut Transform,
        &mut Visibility,
    )>,
) {
    let Some(atlas) = atlas else { return };
    for (parent, mut shadow, mut sprite, mut place, mut visibility) in &mut shadows {
        let Ok((source, object)) = sources.get(parent.parent()) else {
            continue;
        };
        let z = SHADOW_Z - object.translation.z;
        if place.translation.z != z {
            place.translation.z = z;
        }
        let Some(index) = source.texture_atlas.as_ref().map(|a| a.index) else {
            continue;
        };
        if shadow.index == index && shadow.flip_x == source.flip_x {
            continue;
        }
        shadow.index = index;
        shadow.flip_x = source.flip_x;
        match atlas
            .footprint(index)
            .and_then(|f| contact_shadow(f, source.flip_x))
        {
            Some((at, size)) => {
                place.translation.x = at.x;
                place.translation.y = at.y;
                sprite.custom_size = Some(size);
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}

/// Other people standing in the dark come from the server as silhouettes, with nothing
/// about who they are (`apps/server/src/silhouettes.rs`): drawn black.
fn silhouettes(mut shapes: Query<&mut Sprite, With<Silhouette>>) {
    for mut sprite in &mut shapes {
        if sprite.color != Color::BLACK {
            sprite.color = Color::BLACK;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_stands_casts_a_shadow_and_what_lies_flat_does_not() {
        assert!(casts_shadow("human"));
        assert!(casts_shadow("closet"));
        assert!(casts_shadow("oxygen-canister"));
        assert!(!casts_shadow("pipe"));
        assert!(!casts_shadow("air-alarm"), "hangs on the wall");
        assert!(!casts_shadow("ignition-switch"), "hangs on the wall");
        assert!(!casts_shadow("fire"), "is light");
    }

    #[test]
    fn the_shadow_sits_under_the_lowest_pixels() {
        // A person: 12 px wide, feet on the cell's bottom row but one.
        let person = Footprint {
            left: 10,
            right: 21,
            top: 2,
            bottom: 30,
        };
        let (at, size) = contact_shadow(person, false).unwrap();
        assert_eq!(at.x, 0.0, "centred under them");
        assert_eq!(at.y, -TILE_PX / 2.0 + 1.0 + TUCK_PX, "at their feet");
        assert!(
            size.x > 12.0 && size.y < size.x / 2.0,
            "flat and a bit wider than them"
        );

        // Something standing off to one side: the shadow follows it, and its mirror image.
        let left = Footprint {
            left: 0,
            right: 9,
            top: 8,
            bottom: 28,
        };
        let (a, _) = contact_shadow(left, false).unwrap();
        let (b, _) = contact_shadow(left, true).unwrap();
        assert_eq!(a.x, -11.0);
        assert_eq!(b.x, 11.0);
    }

    #[test]
    fn small_things_lying_on_the_floor_cast_none() {
        let welder = Footprint {
            left: 6,
            right: 26,
            top: 12,
            bottom: 20,
        };
        assert!(contact_shadow(welder, false).is_none());
    }
}
