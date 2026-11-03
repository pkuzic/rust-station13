//! The round (Luna's `gameticker`): a wait in the lobby, then play under a mode picked at random.
//! Spec: `docs/reference/round.md`.

use bevy_ecs::prelude::*;

use crate::rng::GameRng;

/// How long the lobby waits, seconds (Luna `pregame_timeleft`).
pub const PREGAME_SECONDS: u32 = 180;

/// A game mode, and how likely it is against the others (Luna's `config.txt`, `PROBABILITY`). A mode
/// that isn't built yet is never picked.
pub struct Mode {
    pub id: &'static str,
    pub probability: u32,
    pub built: bool,
}

pub const MODES: [Mode; 9] = [
    Mode {
        id: "extended",
        probability: 5,
        built: true,
    },
    Mode {
        id: "traitor",
        probability: 8,
        built: false,
    },
    Mode {
        id: "meteor",
        probability: 2,
        built: false,
    },
    Mode {
        id: "malfunction",
        probability: 2,
        built: false,
    },
    Mode {
        id: "blob",
        probability: 0,
        built: false,
    },
    Mode {
        id: "nuclear",
        probability: 5,
        built: false,
    },
    Mode {
        id: "sandbox",
        probability: 0,
        built: false,
    },
    Mode {
        id: "restructuring",
        probability: 0,
        built: false,
    },
    Mode {
        id: "revolution",
        probability: 5,
        built: false,
    },
];

/// Pick a mode by its chance among those built (Luna `pick_random_mode`).
pub fn pick_mode(rng: &mut GameRng) -> &'static str {
    let built = || MODES.iter().filter(|m| m.built && m.probability > 0);
    let total: u32 = built().map(|m| m.probability).sum();
    let mut roll = rng.range(0, total as i32 - 1).max(0) as u32;
    for mode in built() {
        if roll < mode.probability {
            return mode.id;
        }
        roll -= mode.probability;
    }
    "extended"
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Phase {
    /// Waiting in the lobby: ds until the round starts.
    Pregame {
        starts_at: f32,
    },
    Playing {
        mode: &'static str,
    },
    /// Over: ds until the server restarts (Luna's 60 seconds).
    Finished {
        restart_at: f32,
    },
}

/// How long after the end of a round the server restarts, ds.
pub const RESTART_DS: f32 = 600.0;

/// Where the round has got to. Only there on a server running rounds.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct Round {
    pub phase: Phase,
}

impl Round {
    pub fn starting_in(seconds: u32) -> Self {
        Self {
            phase: Phase::Pregame {
                starts_at: seconds as f32 * 10.0,
            },
        }
    }
}
