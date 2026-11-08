//! Small particle effects (`docs/reference/lighting.md`, Particles): short bursts of a few
//! square pixels in the sprites' own palette, drawn only on this client.
//!
//! - **Sparks** where sparks are heard (`sparks`: shocks, sparkers, broken machines), and
//!   where a lit light breaks: a spray of bright orange flecks that glow and fall.
//! - **Glass** where glass shatters (`shatter`): a few pale shards.
//! - **Dust** where a wall comes down or something is taken apart (`deconstruct`, a wall
//!   tile turning into anything else): a slow grey cloud.
//! - **Steam** over a shower's mist: wisps that rise and fade.
//!
//! What starts them is what the server already sends: sounds (`audio.rs`), appearances and
//! the map. They change nothing and nobody else sees them differently.

use bevy::prelude::*;
use rs13_world::{Appearance, GridPos, MapLayer, TilePos};

use crate::audio::Heard;
use crate::{TILE_PX, world_pos};

/// Sparks glow over the darkness (`lighting.rs`, 20) like glowing things (`glow.rs`).
const GLOW_Z: f32 = 20.3;
/// Dust, glass and steam are lit like everything else: under the darkness, over what stands.
const LIT_Z: f32 = 15.0;
/// How often a shower's mist puts out a wisp of steam, seconds.
const STEAM_EVERY: f32 = 0.3;

/// One fleck: how it moves, how long it lasts, and its colour at the start.
#[derive(Component)]
struct Particle {
    velocity: Vec2,
    /// Pulls it down the screen, px/s² (falling flecks), or up (negative: rising steam).
    fall: f32,
    /// Slows it, per second.
    drag: f32,
    age: f32,
    life: f32,
    color: LinearRgba,
}

/// A small, fast, good enough random source for where flecks fly.
#[derive(Resource)]
struct Dice(u64);

impl Dice {
    fn next(&mut self) -> f32 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let bits = self.0.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 40;
        bits as f32 / (1u64 << 24) as f32
    }

    /// Between `lo` and `hi`.
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next()
    }

    /// A direction, any way round.
    fn dir(&mut self) -> Vec2 {
        Vec2::from_angle(self.range(0.0, std::f32::consts::TAU))
    }
}

/// The kinds of burst.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Burst {
    Sparks,
    Glass,
    Dust,
    Steam,
}

/// What a heard sound sets off, if anything.
pub fn burst_for_sound(sound: &str) -> Option<Burst> {
    match sound {
        "sparks" => Some(Burst::Sparks),
        "shatter" => Some(Burst::Glass),
        "deconstruct" => Some(Burst::Dust),
        _ => None,
    }
}

pub struct ParticlesPlugin;

impl Plugin for ParticlesPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Dice(0x9e37_79b9_7f4a_7c15))
            .init_resource::<Walls>()
            .add_systems(
                Update,
                (from_sounds, from_lights, from_walls, from_mist, move_particles),
            );
    }
}

fn spawn(commands: &mut Commands, dice: &mut Dice, burst: Burst, at: Vec2) {
    let (count, z) = match burst {
        Burst::Sparks => (10, GLOW_Z),
        Burst::Glass => (6, LIT_Z),
        Burst::Dust => (14, LIT_Z),
        Burst::Steam => (2, LIT_Z),
    };
    for _ in 0..count {
        let (velocity, fall, drag, life, color, size, from) = match burst {
            Burst::Sparks => {
                // Brighter than white, so the bloom spreads them (`glow.rs`).
                let heat = dice.range(0.0, 1.0);
                (
                    dice.dir() * dice.range(40.0, 110.0),
                    160.0,
                    2.0,
                    dice.range(0.25, 0.55),
                    LinearRgba::rgb(1.9, 1.1 + 0.5 * heat, 0.3 * heat),
                    1.0,
                    Vec2::ZERO,
                )
            }
            Burst::Glass => (
                dice.dir() * dice.range(25.0, 60.0),
                90.0,
                3.0,
                dice.range(0.4, 0.7),
                LinearRgba::new(0.75, 0.88, 0.95, 0.9),
                1.0,
                Vec2::ZERO,
            ),
            Burst::Dust => {
                let grey = dice.range(0.35, 0.55);
                (
                    dice.dir() * dice.range(6.0, 22.0),
                    -4.0,
                    1.2,
                    dice.range(0.8, 1.6),
                    LinearRgba::new(grey, grey, grey * 0.95, 0.55),
                    2.0,
                    dice.dir() * dice.range(0.0, TILE_PX * 0.35),
                )
            }
            Burst::Steam => (
                Vec2::new(dice.range(-5.0, 5.0), dice.range(12.0, 20.0)),
                -4.0,
                0.3,
                dice.range(1.2, 2.0),
                LinearRgba::new(0.92, 0.94, 0.96, 0.5),
                2.0,
                // From the top of the mist.
                Vec2::new(dice.range(-11.0, 11.0), dice.range(2.0, 10.0)),
            ),
        };
        commands.spawn((
            Sprite::from_color(Color::LinearRgba(color), Vec2::splat(size)),
            Transform::from_translation((at + from).extend(z)),
            Particle {
                velocity,
                fall,
                drag,
                age: 0.0,
                life,
                color,
            },
        ));
    }
}

/// Where a tile's middle is drawn.
fn tile_middle(pos: TilePos) -> Vec2 {
    world_pos(pos, 0.0).truncate()
}

fn from_sounds(mut commands: Commands, mut dice: ResMut<Dice>, mut heard: MessageReader<Heard>) {
    for sound in heard.read() {
        if let Some(burst) = burst_for_sound(&sound.sound) {
            spawn(&mut commands, &mut dice, burst, tile_middle(sound.at));
        }
    }
}

/// A light fixture whose state turns to broken sparks; one that only burns out doesn't.
fn from_lights(
    mut commands: Commands,
    mut dice: ResMut<Dice>,
    lights: Query<(&Appearance, &GridPos, &Transform), Changed<Appearance>>,
    mut seen: Local<std::collections::HashMap<TilePos, bool>>,
) {
    for (look, pos, transform) in &lights {
        if !matches!(look.sprite.as_str(), "light" | "light-small") {
            continue;
        }
        let broken = look.state.contains("-broken");
        let was = seen.insert(pos.0, broken);
        if broken && was == Some(false) {
            spawn(
                &mut commands,
                &mut dice,
                Burst::Sparks,
                transform.translation.truncate(),
            );
        }
    }
}

/// The walls on each deck as last seen, to notice one coming down.
#[derive(Resource, Default)]
struct Walls(std::collections::HashSet<TilePos>);

fn is_wall(id: &str) -> bool {
    matches!(id, "wall" | "r-wall")
}

fn from_walls(
    mut commands: Commands,
    mut dice: ResMut<Dice>,
    mut walls: ResMut<Walls>,
    maps: Query<Ref<MapLayer>>,
) {
    for map in &maps {
        if !map.is_changed() {
            continue;
        }
        let first = map.is_added();
        let map = &map.0;
        for y in 0..map.height as i32 {
            for x in 0..map.width as i32 {
                let pos = TilePos::new(x, y, map.z);
                let wall = map.tile(pos).is_some_and(|t| is_wall(&t.id));
                if wall {
                    walls.0.insert(pos);
                } else if walls.0.remove(&pos) && !first {
                    spawn(&mut commands, &mut dice, Burst::Dust, tile_middle(pos));
                }
            }
        }
    }
}

/// Steam rising over a shower's mist (`plumbing.md`).
fn from_mist(
    mut commands: Commands,
    time: Res<Time>,
    mut dice: ResMut<Dice>,
    mists: Query<(&Appearance, &Transform, &Visibility)>,
    mut since: Local<f32>,
) {
    *since += time.delta_secs();
    if *since < STEAM_EVERY {
        return;
    }
    *since = 0.0;
    for (look, transform, visibility) in &mists {
        if look.sprite == "mist" && *visibility != Visibility::Hidden {
            spawn(
                &mut commands,
                &mut dice,
                Burst::Steam,
                transform.translation.truncate(),
            );
        }
    }
}

fn move_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut flecks: Query<(Entity, &mut Particle, &mut Transform, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (entity, mut fleck, mut place, mut sprite) in &mut flecks {
        fleck.age += dt;
        if fleck.age >= fleck.life {
            commands.entity(entity).despawn();
            continue;
        }
        let slow = (-fleck.drag * dt).exp();
        let fall = fleck.fall;
        fleck.velocity *= slow;
        fleck.velocity.y -= fall * dt;
        place.translation += (fleck.velocity * dt).extend(0.0);
        // Fade over the last half of its life.
        let left = 1.0 - fleck.age / fleck.life;
        let fade = (left * 2.0).min(1.0);
        let mut color = fleck.color;
        color.alpha *= fade;
        sprite.color = Color::LinearRgba(color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sounds_set_off_their_bursts() {
        assert_eq!(burst_for_sound("sparks"), Some(Burst::Sparks));
        assert_eq!(burst_for_sound("shatter"), Some(Burst::Glass));
        assert_eq!(burst_for_sound("deconstruct"), Some(Burst::Dust));
        assert_eq!(burst_for_sound("airlock"), None);
    }

    #[test]
    fn the_dice_stay_in_range() {
        let mut dice = Dice(12345);
        for _ in 0..1000 {
            let v = dice.range(2.0, 3.0);
            assert!((2.0..3.0).contains(&v), "{v}");
        }
    }
}
