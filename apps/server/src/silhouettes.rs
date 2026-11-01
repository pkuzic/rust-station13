//! People in the dark (`docs/reference/lighting.md`, Silhouettes): someone standing on a
//! dark tile is sent to everyone else as a stand-in that carries only where they are,
//! which way they face and their body's pose, never their name, clothes or what they hold.
//! So a modified client learns no more than the black shape the client draws. Clicking
//! the stand-in acts on the person.

use bevy::prelude::*;
use rs13_classic::lighting::TileLight;
use rs13_classic::movement::Mob;
use rs13_classic::objects::NetIds;
use rs13_world::{Appearance, Facing, GridPos, Silhouette};

/// On a stand-in: the person it stands for.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shade(pub Entity);

/// On a person standing in the dark: their stand-in.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shaded(pub Entity);

/// Give each person on a dark tile a stand-in and keep it where they are; take it away
/// when they step into the light, leave the map (into a closet) or are gone.
#[allow(clippy::type_complexity)]
pub fn update_silhouettes(
    mut commands: Commands,
    light: Res<TileLight>,
    mut ids: ResMut<NetIds>,
    people: Query<
        (
            Entity,
            Option<&GridPos>,
            Option<&Facing>,
            &Appearance,
            Option<&Shaded>,
        ),
        (With<Mob>, Without<Shade>),
    >,
    mut shades: Query<(Entity, &Shade, &mut GridPos, &mut Facing, &mut Appearance), Without<Mob>>,
) {
    for (person, pos, facing, look, shaded) in &people {
        let dark = pos.is_some_and(|p| light.is_dark(p.0));
        match (dark, shaded) {
            (true, None) => {
                let Some(pos) = pos else { continue };
                let shade = commands
                    .spawn((
                        Shade(person),
                        Silhouette,
                        ids.alloc(),
                        *pos,
                        facing.copied().unwrap_or(Facing(0)),
                        look.clone(),
                    ))
                    .id();
                commands.entity(person).insert(Shaded(shade));
            }
            (false, Some(shaded)) => {
                commands.entity(shaded.0).despawn();
                commands.entity(person).remove::<Shaded>();
            }
            _ => {}
        }
    }
    for (shade, of, mut pos, mut facing, mut look) in &mut shades {
        let Ok((_, Some(person_pos), person_facing, person_look, Some(_))) = people.get(of.0) else {
            // Its person is gone or no longer in the dark: the loop above or this one
            // takes it away.
            if people.get(of.0).is_err() {
                commands.entity(shade).despawn();
            }
            continue;
        };
        pos.set_if_neq(*person_pos);
        facing.set_if_neq(person_facing.copied().unwrap_or(Facing(0)));
        if *look != *person_look {
            *look = person_look.clone();
        }
    }
}
