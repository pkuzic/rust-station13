//! The classic SS13 game build, modelled on NSV Luna.
//!
//! Luna is a behaviour reference only: see `docs/GAME_REFERENCE.md`. No DM code is
//! translated into this crate.

pub mod access;
pub mod actions;
pub mod alarms;
pub mod apc;
pub mod atmos;
pub mod blastdoors;
pub mod cables;
pub mod cameras;
pub mod cells;
pub mod chairs;
pub mod closets;
pub mod combat;
pub mod decks;
pub mod dispensers;
pub mod disposals;
pub mod doors;
pub mod environment;
pub mod equipment;
pub mod extinguishers;
pub mod firealarms;
pub mod firedoors;
pub mod fires;
pub mod flashlights;
pub mod floodlights;
pub mod floors;
pub mod food;
pub mod freezers;
pub mod furniture;
pub mod gas_panel;
pub mod generators;
pub mod hands;
pub mod health;
pub mod igniters;
pub mod jetpacks;
pub mod lighting;
pub mod lights;
pub mod machines;
pub mod medical;
pub mod meters;
pub mod movement;
pub mod multiz;
pub mod names;
pub mod objects;
pub mod paperwork;
pub mod portables;
pub mod power;
pub mod power_monitor;
pub mod pulling;
pub mod radio;
pub mod rng;
pub mod smes_panel;
pub mod smoking;
pub mod sound;
pub mod sparks;
pub mod speech;
pub mod status_displays;
pub mod storage;
pub mod supermatter;
pub mod teg;
pub mod time;
pub mod vending;
pub mod verbs;
pub mod walllockers;
pub mod welding;
pub mod windows;
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
    /// NSV Luna's toxins lab, with its burn chamber (`maps/luna_toxins.import.ron`).
    pub const LUNA_TOXINS: &str = include_str!("../maps/luna_toxins.ron");
    /// NSV Luna's incinerator (`maps/luna_incinerator.import.ron`).
    pub const LUNA_INCINERATOR: &str = include_str!("../maps/luna_incinerator.ron");
    /// All of NSV Luna's station, bottom deck first: Luna's decks 4, 3, 2 and 1
    /// (`maps/luna_deck*.import.ron`), joined by its ladders and stairs.
    pub const LUNA_STATION: [&str; 4] = [
        include_str!("../maps/luna_deck4.ron"),
        include_str!("../maps/luna_deck3.ron"),
        include_str!("../maps/luna_deck2.ron"),
        include_str!("../maps/luna_deck1.ron"),
    ];
}

/// Server-side game rules. Runs every fixed tick.
pub struct ClassicPlugin;

impl Plugin for ClassicPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<time::WorldTime>()
            .init_resource::<objects::NetIds>()
            .init_resource::<pulling::Stepped>()
            .init_resource::<pulling::PendingPulls>()
            .init_resource::<objects::ItemProtos>()
            .init_resource::<access::AccessIds>()
            .init_resource::<rng::GameRng>()
            .init_resource::<combat::PendingAttacks>()
            .init_resource::<medical::PendingTreatments>()
            .init_resource::<combat::VisibleEvents>()
            .init_resource::<atmos::WindCooldowns>()
            .init_resource::<machines::PipeNets>()
            .init_resource::<power::Powernets>()
            .init_resource::<power::AreaPower>()
            .init_resource::<power::GridTotals>()
            .init_resource::<wires::WireLayout>()
            .init_resource::<wires::PendingShocks>()
            .init_resource::<alarms::AreaSeals>()
            .init_resource::<firealarms::AreaFires>()
            .init_resource::<fires::FireSprites>()
            .init_resource::<windows::PaneEdges>()
            .init_resource::<igniters::PendingSparks>()
            .init_resource::<sparks::SparkShowers>()
            .init_resource::<extinguishers::PendingSprays>()
            .init_resource::<supermatter::Beams>()
            .init_resource::<supermatter::CoreAlerts>()
            .init_resource::<lights::AreaLights>()
            .init_resource::<lighting::TileLight>()
            .init_resource::<closets::ClosetProtos>()
            .init_resource::<closets::PendingClosetExits>()
            .init_resource::<sound::SoundEvents>()
            .init_resource::<sound::SoundBarriers>()
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
                    apc::process_apc_ops,
                    supermatter::process_laser_ops,
                    freezers::process_freezer_ops,
                    gas_panel::process_gas_ops,
                    vending::process_vend_ops,
                    cameras::process_camera_ops,
                    paperwork::process_paper_ops,
                    smes_panel::process_smes_ops,
                    wires::resolve_shocks,
                    combat::resolve_attacks,
                    medical::resolve_treatments,
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
                    firedoors::update_firedoors,
                    doors::update_doors,
                    blastdoors::run_blast_doors,
                    windows::update_pane_edges,
                    pulling::start_pulls,
                    movement::move_mobs,
                    pulling::follow_pullers,
                    pulling::drop_lost_pulls,
                    closets::leave_closets,
                    movement::sync_facing,
                )
                    .chain()
                    .in_set(TickSet::Gameplay),
            )
            .add_systems(
                FixedUpdate,
                (
                    machines::rebuild_networks,
                    power::rebuild_powernets,
                    generators::run_generators,
                    freezers::run_freezers,
                    teg::run_tegs,
                    power::power_cycle,
                    cells::run_chargers,
                    lights::run_lights,
                    machines::run_machines,
                    meters::update_meters,
                    alarms::process_alarms,
                    firealarms::process_fire_alarms,
                    fires::expose_flames,
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
                    igniters::run_igniters,
                    sparks::run_sparks,
                    extinguishers::run_puffs,
                    portables::run_portables,
                    vending::run_vending,
                    smoking::burn_flames,
                    food::get_hungry,
                    disposals::run_bins,
                    disposals::run_bundles,
                    disposals::run_outlets,
                )
                    .chain()
                    .after(fires::expose_flames)
                    .before(atmos::step_atmos)
                    .in_set(TickSet::Simulation),
            )
            .add_systems(
                FixedUpdate,
                (
                    supermatter::fire_lasers,
                    supermatter::update_beam_power,
                    supermatter::run_supermatter,
                    supermatter::broadcast_core_alerts,
                )
                    .chain()
                    .after(machines::run_machines)
                    .before(atmos::step_atmos)
                    .in_set(TickSet::Simulation),
            )
            .add_systems(
                FixedUpdate,
                (
                    hands::sync_hands_view,
                    equipment::sync_worn_view,
                    // Machine windows and open containers.
                    (
                        wires::update_panel_views,
                        alarms::update_alarm_views,
                        apc::update_apc_views,
                        power_monitor::update_monitor_views,
                        supermatter::update_laser_views,
                        freezers::update_freezer_views,
                        gas_panel::update_gas_views,
                        vending::update_vending_views,
                        cameras::update_camera_views,
                        paperwork::update_clipboard_views,
                        smes_panel::update_smes_views,
                        storage::update_storage_views,
                    ),
                    machines::update_machine_looks,
                    machines::update_canister_looks,
                    power::update_power_looks,
                    cables::update_cable_looks,
                    floors::hide_under_floors,
                    welding::update_welder_looks,
                    // Sprites that follow what things are doing.
                    (
                        extinguishers::update_extinguisher_looks,
                        extinguishers::update_cabinet_looks,
                        portables::update_portable_looks,
                        vending::update_vending_looks,
                        smoking::update_smoking_looks,
                        cameras::update_camera_looks,
                        paperwork::update_paperwork_looks,
                        status_displays::update_displays,
                    ),
                    lights::update_light_looks,
                    lights::update_light_sources,
                    flashlights::update_flashlight_looks,
                    flashlights::update_flashlight_sources,
                    closets::update_closet_looks,
                    firedoors::update_firedoor_looks,
                )
                    .in_set(TickSet::Publish),
            )
            .add_systems(
                FixedUpdate,
                supermatter::update_computer_looks.in_set(TickSet::Publish),
            )
            .add_systems(FixedUpdate, fires::update_fire_sprites.in_set(TickSet::Publish))
            .add_systems(
                FixedUpdate,
                (
                    dispensers::update_dispenser_looks,
                    cells::update_cell_looks,
                    jetpacks::update_jetpack_looks,
                    chairs::update_chairs,
                    power_monitor::update_monitor_looks,
                    igniters::update_igniter_looks,
                    floodlights::run_floodlights,
                )
                    .in_set(TickSet::Publish),
            )
            // After the fixtures say whether they are lit: rules next tick see this light.
            .add_systems(FixedUpdate, lighting::update_tile_light.after(TickSet::Publish))
            // Last: sounds made anywhere this tick reach their listeners.
            .add_systems(
                FixedUpdate,
                (
                    sound::door_sounds,
                    sound::closet_sounds,
                    sound::light_sounds,
                    sound::deliver_sounds,
                )
                    .chain()
                    .after(TickSet::Publish),
            );
    }
}
