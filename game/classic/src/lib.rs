//! The classic SS13 game build, modelled on NSV Luna.
//!
//! Luna is a behaviour reference only: see `docs/GAME_REFERENCE.md`. No DM code is
//! translated into this crate.

pub mod access;
pub mod actions;
pub mod alarms;
pub mod atmos;
pub mod cables;
pub mod combat;
pub mod doors;
pub mod environment;
pub mod equipment;
pub mod floors;
pub mod furniture;
pub mod hands;
pub mod health;
pub mod machines;
pub mod movement;
pub mod names;
pub mod objects;
pub mod power;
pub mod rng;
pub mod speech;
pub mod storage;
pub mod time;
pub mod welding;
pub mod wires;

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
    /// NSV Luna's engineering section, imported from `NSV_Luna.dmm` (see
    /// `maps/luna_engineering.import.ron`).
    pub const LUNA_ENGINEERING: &str = include_str!("../maps/luna_engineering.ron");
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
            .init_resource::<atmos::WindCooldowns>()
            .init_resource::<machines::PipeNets>()
            .init_resource::<power::Powernets>()
            .init_resource::<power::AreaPower>()
            .init_resource::<wires::WireLayout>()
            .init_resource::<wires::PendingShocks>()
            .init_resource::<alarms::AreaSeals>()
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
                    furniture::finish_table_work,
                    equipment::process_equipment,
                    wires::process_wire_ops,
                    alarms::process_alarm_ops,
                    wires::resolve_shocks,
                    combat::resolve_attacks,
                    speech::speak,
                    combat::deliver_visible,
                )
                    .chain()
                    .in_set(TickSet::Commands),
            )
            .add_systems(
                FixedUpdate,
                (
                    wires::update_wiring,
                    alarms::apply_seals,
                    doors::update_doors,
                    movement::move_mobs,
                )
                    .chain()
                    .in_set(TickSet::Gameplay),
            )
            .add_systems(
                FixedUpdate,
                (
                    machines::rebuild_networks,
                    power::rebuild_powernets,
                    power::power_cycle,
                    machines::run_machines,
                    alarms::process_alarms,
                    atmos::step_atmos,
                    atmos::space_wind,
                    atmos::drift,
                    environment::sense_surroundings,
                    environment::life,
                    health::update_vitals,
                    atmos::publish_atmos_view,
                )
                    .chain()
                    .in_set(TickSet::Simulation),
            )
            .add_systems(
                FixedUpdate,
                (
                    hands::sync_hands_view,
                    equipment::sync_worn_view,
                    wires::update_panel_views,
                    alarms::update_alarm_views,
                    storage::update_storage_views,
                    machines::update_machine_looks,
                    power::update_power_looks,
                    cables::update_cable_looks,
                    floors::hide_under_floors,
                    welding::update_welder_looks,
                )
                    .in_set(TickSet::Publish),
            );
    }
}
