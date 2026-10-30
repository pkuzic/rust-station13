//! Round time, counted in fixed ticks.

use bevy_ecs::prelude::*;
use rs13_core::{TICK_RATE_HZ, Tick};

/// Ticks elapsed since the round started. Luna measures time in deciseconds
/// (`world.time`); `deciseconds()` gives the equivalent.
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct WorldTime {
    pub tick: Tick,
}

impl WorldTime {
    pub fn deciseconds(&self) -> f32 {
        self.tick.0 as f32 * 10.0 / TICK_RATE_HZ as f32
    }
}

/// One fixed tick, in deciseconds.
pub const TICK_DS: f32 = 10.0 / TICK_RATE_HZ as f32;

pub fn advance_time(mut time: ResMut<WorldTime>) {
    time.tick = time.tick.next();
}
