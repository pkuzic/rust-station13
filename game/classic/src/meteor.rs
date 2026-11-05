//! Meteors, and the mode that's all of them (Luna's `gamemodes/meteor`). Spec:
//! `docs/reference/meteor-blob.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, Dense, GridPos, MapLayer, TilePos};

use crate::explosions::{explode, strike};
use crate::objects::NetIds;
use crate::rng::GameRng;
use crate::round::{Phase, Round};
use crate::sound::SoundEvents;
use crate::time::WorldTime;

pub const MODE: &str = "meteor";
pub const METEOR: &str = "meteor";
/// How often a wave comes, ds (Luna `meteor_wave_delay`).
const WAVE_DS: f32 = 600.0;
/// How many in a wave (Luna's `0 to meteors_in_wave`), and when in it they come, ds.
const IN_WAVE: usize = 3;
const WAVE_SPREAD_MIN: i32 = 10;
const WAVE_SPREAD_MAX: i32 = 100;
/// How long between steps, ds (Luna's `walk_towards` with a lag of 1).
const STEP_DS: f32 = 1.0;
/// Chance in percent that one explodes as it hits (Luna `Bump`).
const BLOW_CHANCE: f32 = 15.0;

/// A meteor on its way.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Meteor {
    pub goal: TilePos,
    next_step: f32,
}

/// When the next wave is, and those of this wave yet to come.
#[derive(Resource, Debug, Default, Clone, PartialEq)]
pub struct MeteorWaves {
    next_wave: f32,
    coming: Vec<f32>,
}

fn is_meteor_round(world: &World) -> bool {
    world
        .get_resource::<Round>()
        .is_some_and(|round| matches!(round.phase, Phase::Playing { mode: MODE }))
}

/// One meteor, from a side of a random deck (Luna's `spawn_meteor`). None if no place on that side
/// of any deck is open space after a few tries.
pub fn spawn_meteor(world: &mut World) -> Option<Entity> {
    let mut maps = world.query::<&MapLayer>();
    let decks: Vec<(i32, i32, i32)> = maps
        .iter(world)
        .map(|m| (m.0.z, m.0.width as i32, m.0.height as i32))
        .collect();
    if decks.is_empty() {
        return None;
    }
    for _ in 0..50 {
        let mut rng = world.resource_mut::<GameRng>();
        let (z, width, height) = decks[rng.range(0, decks.len() as i32 - 1) as usize];
        let (across, up) = (rng.range(0, width - 1), rng.range(0, height - 1));
        let (other_across, other_up) = (rng.range(0, width - 1), rng.range(0, height - 1));
        let (start, goal) = match rng.range(0, 3) {
            0 => ((across, height - 1), (other_across, 0)),
            1 => ((width - 1, up), (0, other_up)),
            2 => ((across, 0), (other_across, height - 1)),
            _ => ((0, up), (width - 1, other_up)),
        };
        let start = TilePos::new(start.0, start.1, z);
        let goal = TilePos::new(goal.0, goal.1, z);
        let mut maps = world.query::<&MapLayer>();
        let open = maps
            .iter(world)
            .find(|m| m.0.z == z)
            .and_then(|m| m.0.tile(start))
            .is_some_and(|t| t.id == "space");
        if !open {
            continue;
        }
        let id = world.resource_mut::<NetIds>().alloc();
        return Some(
            world
                .spawn((
                    GridPos(start),
                    id,
                    Appearance::new(METEOR, ""),
                    Meteor { goal, next_step: 0.0 },
                ))
                .id(),
        );
    }
    None
}

/// The tile a step towards this one lands on.
fn step_towards(from: TilePos, to: TilePos) -> TilePos {
    TilePos::new(
        from.x + (to.x - from.x).signum(),
        from.y + (to.y - from.y).signum(),
        from.z,
    )
}

/// A meteor meets something: it's hit, and that's the end of the meteor (Luna's `Bump`).
fn hit(world: &mut World, meteor: Entity, at: TilePos, was_at: TilePos) {
    world
        .resource_mut::<SoundEvents>()
        .play(crate::sound::METEOR_IMPACT, at, 40.0);
    strike(world, at, 2);
    if world.resource_mut::<GameRng>().prob(BLOW_CHANCE) {
        explode(world, was_at, 2, 5, 8, 10);
    }
    world.entity_mut(meteor).despawn();
}

/// Waves in a meteor round, and every meteor's flight.
pub fn run_meteors(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    if is_meteor_round(world) {
        let due = now >= world.resource::<MeteorWaves>().next_wave;
        if due {
            let mut rng = world.resource_mut::<GameRng>();
            let coming: Vec<f32> = (0..IN_WAVE)
                .map(|_| now + rng.range(WAVE_SPREAD_MIN, WAVE_SPREAD_MAX) as f32)
                .collect();
            let mut waves = world.resource_mut::<MeteorWaves>();
            waves.next_wave = now + WAVE_DS;
            waves.coming.extend(coming);
        }
    }
    let mut waves = world.resource_mut::<MeteorWaves>();
    let before = waves.coming.len();
    waves.coming.retain(|at| now < *at);
    let ready = before - waves.coming.len();
    for _ in 0..ready {
        spawn_meteor(world);
    }
    let mut meteors = world.query::<(Entity, &GridPos, &Meteor)>();
    let flying: Vec<(Entity, TilePos, Meteor)> = meteors
        .iter(world)
        .filter(|(_, _, m)| now >= m.next_step)
        .map(|(e, p, m)| (e, p.0, *m))
        .collect();
    if flying.is_empty() {
        return;
    }
    let mut dense = world.query_filtered::<&GridPos, With<Dense>>();
    let blocked: Vec<TilePos> = dense.iter(world).map(|p| p.0).collect();
    for (meteor, at, flight) in flying {
        if at == flight.goal {
            world.entity_mut(meteor).despawn();
            continue;
        }
        let next = step_towards(at, flight.goal);
        let mut maps = world.query::<&MapLayer>();
        let tile = maps
            .iter(world)
            .find(|m| m.0.z == next.z)
            .and_then(|m| m.0.tile(next).cloned());
        let Some(tile) = tile else {
            world.entity_mut(meteor).despawn();
            continue;
        };
        if tile.solid || blocked.contains(&next) {
            hit(world, meteor, next, at);
            continue;
        }
        world.entity_mut(meteor).insert((
            GridPos(next),
            Meteor {
                next_step: now + STEP_DS,
                ..flight
            },
        ));
    }
}

/// The word of warning as a meteor round starts (Luna's `announce`).
pub fn warning() -> LocMsg {
    LocMsg::new("round-mode-meteor")
}
