//! The classic SS13 game build, modelled on NSV Luna.
//!
//! Luna is a behaviour reference only: see `docs/GAME_REFERENCE.md`. No DM code is
//! translated into this crate.

pub mod access;
pub mod actions;
pub mod combat;
pub mod doors;
pub mod hands;
pub mod health;
pub mod movement;
pub mod names;
pub mod objects;
pub mod rng;
pub mod speech;
pub mod time;

use bevy_app::{App, FixedUpdate, Plugin};
use bevy_ecs::prelude::*;
use rs13_simulation::TickSet;

/// The NSV Luna commit (github.com/SS13/luna) that defines reference behaviour.
pub const REFERENCE_COMMIT: &str = "e13cf12";

/// Localisation files shipped with the classic build.
pub mod locales {
    pub const EN_MESSAGES: &str = include_str!("../locales/en/messages.ftl");
    pub const EN_NOUNS: &str = include_str!("../locales/en/nouns.ron");
    pub const RU_MESSAGES: &str = include_str!("../locales/ru/messages.ftl");
    pub const RU_NOUNS: &str = include_str!("../locales/ru/nouns.ron");
}

/// Maps shipped with the classic build.
pub mod maps {
    pub const ENGINEERING_CHAMBER: &str = include_str!("../maps/engineering_chamber.ron");
}

/// Server-side game rules. Runs every fixed tick.
pub struct ClassicPlugin;

impl Plugin for ClassicPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<time::WorldTime>()
            .init_resource::<objects::NetIds>()
            .init_resource::<objects::ItemProtos>()
            .init_resource::<access::AccessIds>()
            .init_resource::<rng::GameRng>()
            .init_resource::<combat::PendingAttacks>()
            .init_resource::<combat::VisibleEvents>()
            .configure_sets(
                FixedUpdate,
                (
                    TickSet::Commands,
                    TickSet::Gameplay,
                    TickSet::Simulation,
                    TickSet::Publish,
                )
                    .chain(),
            )
            .add_systems(FixedUpdate, time::advance_time.before(TickSet::Commands))
            .add_systems(
                FixedUpdate,
                (
                    actions::process_actions,
                    combat::resolve_attacks,
                    speech::speak,
                    combat::deliver_visible,
                )
                    .chain()
                    .in_set(TickSet::Commands),
            )
            .add_systems(
                FixedUpdate,
                (doors::update_doors, movement::move_mobs)
                    .chain()
                    .in_set(TickSet::Gameplay),
            )
            .add_systems(FixedUpdate, health::update_vitals.in_set(TickSet::Simulation))
            .add_systems(FixedUpdate, hands::sync_hands_view.in_set(TickSet::Publish));
    }
}
