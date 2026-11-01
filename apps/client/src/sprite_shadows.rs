//! Depth for what stands on the floor (`docs/reference/lighting.md`, Depth):
//!
//! - **Drop shadows:** people and objects cast a flattened dark copy of themselves on the
//!   floor at their foot, so they stand on it instead of floating over it. All shadows lie
//!   in one layer, just over the floor and what lies flat on it and under everything that
//!   stands, so a shadow never falls over another object.
//! - **Silhouettes:** someone standing in the dark is seen as a black shape: you can tell
//!   someone is there, not who.

use bevy::prelude::*;
use rs13_network::protocol::Player;
use rs13_world::{Appearance, GridPos, PixelOffset};

use crate::lighting::LightLevels;
use crate::sprites::{Layer, Look};
use crate::{OwnPlayer, TILE_PX};

/// How much of its height a shadow keeps: flattened onto the floor, as from a light high
/// above.
const SQUASH: f32 = 0.6;
/// Where a shadow's foot sits from its object's, px: to the right and down, so it shows
/// as a dark rim on the floor along the object's foot.
const SHADOW_OFFSET: Vec2 = Vec2::new(5.0, -3.0);
const SHADOW_COLOUR: Color = Color::srgba(0.0, 0.0, 0.0, 0.45);
/// The layer every shadow lies on: over pipes and cables (up to 0.75), under tables and
/// closets (0.9) and everything that stands.
const SHADOW_Z: f32 = 0.8;
/// People on a tile darker than this are silhouettes (the same as the server's "dark").
const SILHOUETTE_BELOW: f32 = rs13_simulation::lighting::DARK_BELOW;

/// What casts a drop shadow: everything drawn standing on the floor, not what lies flat on
/// it, hangs on a wall, or is light itself.
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
            | "injector"
            | "meter"
            | "pipe-vent"
            | "airlock"
            | "firedoor"
            | "light"
            | "light-small"
            | "light-switch"
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
    )
}

/// On a drop shadow.
#[derive(Component)]
struct DropShadow;

/// On a sprite that has its drop shadow.
#[derive(Component)]
struct Shadowed;

pub struct SpriteShadowsPlugin;

impl Plugin for SpriteShadowsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (add_shadows, follow, silhouettes)
                .chain()
                .after(crate::sprites::animate),
        );
    }
}

/// The shadow's place under its object, which stands at `object_z`.
fn shadow_transform(object_z: f32) -> Transform {
    let foot = -(1.0 - SQUASH) * TILE_PX / 2.0;
    Transform::from_xyz(SHADOW_OFFSET.x, foot + SHADOW_OFFSET.y, SHADOW_Z - object_z)
        .with_scale(Vec3::new(1.0, SQUASH, 1.0))
}

/// Give each sprite that stands on the floor a drop shadow. A person's shadow is their
/// body's: what they wear barely changes its flattened shape, and overlapping shadows
/// would darken each other.
#[allow(clippy::type_complexity)]
fn add_shadows(
    mut commands: Commands,
    things: Query<
        (Entity, &Sprite, &Appearance, &Transform),
        (
            With<Look>,
            Without<Shadowed>,
            Without<PixelOffset>,
            Without<DropShadow>,
            Without<Layer>,
        ),
    >,
) {
    for (entity, sprite, appearance, transform) in &things {
        if !casts_shadow(&appearance.sprite) {
            continue;
        }
        let Some(texture) = sprite.texture_atlas.clone() else {
            continue;
        };
        let mut shadow = Sprite::from_atlas_image(sprite.image.clone(), texture);
        shadow.color = SHADOW_COLOUR;
        shadow.flip_x = sprite.flip_x;
        commands.entity(entity).insert(Shadowed).with_child((
            shadow,
            shadow_transform(transform.translation.z),
            DropShadow,
        ));
    }
}

/// Keep each drop shadow on its sprite's current frame, and on the shadow layer when its
/// object changes layer (a chair turned to face north).
fn follow(
    sources: Query<(&Sprite, &Transform), Without<DropShadow>>,
    mut shadows: Query<(&ChildOf, &mut Sprite, &mut Transform), With<DropShadow>>,
) {
    for (parent, mut shadow, mut place) in &mut shadows {
        let Ok((source, object)) = sources.get(parent.parent()) else {
            continue;
        };
        let z = SHADOW_Z - object.translation.z;
        if place.translation.z != z {
            place.translation.z = z;
        }
        if let (Some(from), Some(to)) = (source.texture_atlas.as_ref(), shadow.texture_atlas.as_mut())
            && to.index != from.index
        {
            to.index = from.index;
        }
        if shadow.flip_x != source.flip_x {
            shadow.flip_x = source.flip_x;
        }
    }
}

/// Other people standing in the dark are drawn black, clothes and all.
#[allow(clippy::type_complexity)]
fn silhouettes(
    levels: Res<LightLevels>,
    mut people: Query<
        (&GridPos, &mut Sprite, Option<&Children>),
        (With<Player>, Without<OwnPlayer>, Without<Layer>),
    >,
    mut layers: Query<&mut Sprite, (With<Layer>, Without<Player>)>,
) {
    for (pos, mut sprite, children) in &mut people {
        let dark = levels.brightness(pos.0).is_some_and(|b| b < SILHOUETTE_BELOW);
        let colour = if dark { Color::BLACK } else { Color::WHITE };
        if sprite.color != colour {
            sprite.color = colour;
        }
        for child in children.into_iter().flatten() {
            if let Ok(mut layer) = layers.get_mut(*child)
                && layer.color != colour
            {
                layer.color = colour;
            }
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
        assert!(!casts_shadow("fire"), "is light");
    }

    #[test]
    fn shadows_lie_on_one_layer_at_the_foot() {
        // Whatever layer its object stands on, the shadow ends up on the shadow layer,
        // flattened, with its foot at the object's foot.
        for object_z in [0.9, 1.0, 3.0, 3.15] {
            let t = shadow_transform(object_z);
            assert!((object_z + t.translation.z - SHADOW_Z).abs() < 1e-6);
            let foot = t.translation.y - t.scale.y * TILE_PX / 2.0;
            assert!((foot - (-TILE_PX / 2.0 + SHADOW_OFFSET.y)).abs() < 1e-4);
        }
    }
}
