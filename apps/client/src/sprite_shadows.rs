//! Depth for what stands on the floor (`docs/reference/lighting.md`, Depth):
//!
//! - **Contact shadows:** people and what stands on the floor have a soft dark oval on the
//!   floor under their lowest visible pixels, so they stand on it instead of floating over
//!   it. It is measured from the sprite's own pixels, so it sits under the feet whatever
//!   the sprite's shape, and it points nowhere, so no light direction can be wrong. Small
//!   things lying on the floor and fixtures on walls have none. All shadows lie in one
//!   layer, just over the floor and what lies flat on it and under everything that stands,
//!   so a shadow never falls over another object.
//! - **Cast shadows:** where the light leans one way (`lighting.rs`, `LightLean`), what
//!   stands also casts a long soft shadow away from it, from its feet, longer and darker the
//!   further the light leans, and gone in the dark and in even light.
//! - **Silhouettes:** someone standing in the dark is seen as a black shape: you can tell
//!   someone is there, not who. The server sends only the shape.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use rs13_world::{Appearance, GridPos, PixelOffset, Silhouette};

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
/// A cast shadow: how long it is where the light leans fully one way, px; how wide for the
/// width of its contact shadow; how dark at most.
const CAST_PX: f32 = 26.0;
const CAST_WIDTH: f32 = 0.6;
const CAST_ALPHA: f32 = 0.45;
/// The brightness (as drawn) below which a cast shadow fades out, and above which it is
/// fully there.
const CAST_FADE_FROM: f32 = 0.15;
const CAST_FADE_TO: f32 = 0.45;
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
            | "shuttle-door"
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

/// On a cast shadow: a child of its contact shadow, from the same feet.
#[derive(Component)]
struct CastShadow;

pub struct SpriteShadowsPlugin;

impl Plugin for SpriteShadowsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, make_blob).add_systems(
            Update,
            (add_shadows, follow, cast, silhouettes)
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
        let mut long = Sprite::from_image(blob.0.clone());
        long.color = Color::srgba(1.0, 1.0, 1.0, 0.0);
        // Measured and placed by `follow` and `cast`; hidden until then.
        commands.entity(entity).insert(Shadowed).with_child((
            shadow,
            Transform::default(),
            Visibility::Hidden,
            ContactShadow {
                index: usize::MAX,
                flip_x: false,
            },
            children![(long, Transform::from_xyz(0.0, 0.0, -0.0005), CastShadow)],
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

/// Where a cast shadow goes, for the light's `lean` (towards the light, 0 to 1 long) and
/// the width of its contact shadow, px: its middle from the feet, its length and width, and
/// how far it turns (radians from east). `None` where the light doesn't lean.
pub fn cast_shadow(lean: Vec2, contact_width: f32) -> Option<(Vec2, Vec2, f32)> {
    let size = lean.length().min(1.0);
    if size < 0.05 {
        return None;
    }
    let away = -lean / lean.length();
    let width = contact_width * CAST_WIDTH;
    let length = width / 2.0 + CAST_PX * size;
    // From under the feet.
    Some((
        away * (length / 2.0 - width / 4.0),
        Vec2::new(length, width),
        away.y.atan2(away.x),
    ))
}

/// Lay each cast shadow away from the light where its object stands, fading it in the dark.
#[allow(clippy::type_complexity)]
fn cast(
    on: Res<crate::lighting::LightingOn>,
    lean: Res<crate::lighting::LightLean>,
    objects: Query<(&GlobalTransform, &GridPos)>,
    contacts: Query<(&ChildOf, &Sprite), (With<ContactShadow>, Without<CastShadow>)>,
    mut casts: Query<(&ChildOf, &mut Sprite, &mut Transform), With<CastShadow>>,
) {
    for (contact, mut sprite, mut place) in &mut casts {
        let Ok((object, contact_sprite)) = contacts.get(contact.parent()) else {
            continue;
        };
        let Ok((global, grid)) = objects.get(object.parent()) else {
            continue;
        };
        let (way, bright) = lean.at(global.translation().truncate() / TILE_PX, grid.0.z);
        let width = contact_sprite.custom_size.map_or(TILE_PX / 2.0, |s| s.x);
        let fade = ((bright - CAST_FADE_FROM) / (CAST_FADE_TO - CAST_FADE_FROM)).clamp(0.0, 1.0);
        let shape = if on.0 { cast_shadow(way, width) } else { None };
        let alpha = shape.map_or(0.0, |_| CAST_ALPHA * way.length().min(1.0) * fade);
        if (sprite.color.alpha() - alpha).abs() > 1e-3 {
            sprite.color = Color::srgba(1.0, 1.0, 1.0, alpha);
        }
        let Some((at, size, turn)) = shape else {
            continue;
        };
        if sprite.custom_size.is_none_or(|s| (s - size).length() > 0.05) {
            sprite.custom_size = Some(size);
        }
        let moved = at.extend(place.translation.z);
        if (place.translation - moved).length() > 0.05 {
            place.translation = moved;
        }
        let rotation = Quat::from_rotation_z(turn);
        if place.rotation.angle_between(rotation) > 1e-3 {
            place.rotation = rotation;
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
    fn a_cast_shadow_falls_away_from_the_light() {
        // Light from the east, fully leaning: the shadow lies west of the feet, turned west.
        let (at, size, turn) = cast_shadow(Vec2::new(1.0, 0.0), 20.0).unwrap();
        assert!(at.x < 0.0 && at.y.abs() < 1e-5, "{at:?}");
        assert_eq!(size, Vec2::new(10.0 * CAST_WIDTH + CAST_PX, 20.0 * CAST_WIDTH));
        assert!((turn.abs() - std::f32::consts::PI).abs() < 1e-5, "west: {turn}");
        // Half as much lean, shorter.
        let (_, half, _) = cast_shadow(Vec2::new(0.0, -0.5), 20.0).unwrap();
        assert_eq!(half.x, 10.0 * CAST_WIDTH + CAST_PX / 2.0);
        assert!(cast_shadow(Vec2::ZERO, 20.0).is_none(), "even light");
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
