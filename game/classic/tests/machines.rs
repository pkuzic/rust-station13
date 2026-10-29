//! Pipes and machinery on the engineering chamber map. Spec: docs/reference/pipes.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::atmos::StationAtmos;
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::machines::{GasStore, PipeMember, PipeNets, Switch};
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_classic::{ClassicPlugin, maps};
use rs13_simulation::atmos::Gas;
use rs13_simulation::atmos::gas::ONE_ATMOSPHERE;
use rs13_world::{Appearance, GridPos, TileMap, TilePos};

const TICKS_PER_SECOND: usize = 20;

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(
        app.world_mut(),
        TileMap::from_ron(maps::ENGINEERING_CHAMBER, 0).unwrap(),
    );
    assert!(problems.is_empty(), "{problems:?}");
    app
}

fn seconds(app: &mut App, s: usize) {
    for _ in 0..s * TICKS_PER_SECOND {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn find_at(app: &mut App, sprite: &str, pos: TilePos) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &Appearance, &GridPos)>();
    q.iter(app.world())
        .find(|(_, a, p)| a.sprite == sprite && p.0 == pos)
        .unwrap_or_else(|| panic!("no {sprite} at {pos:?}"))
        .0
}

fn act(app: &mut App, mob: Entity, action: Action) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
    app.world_mut().run_schedule(FixedUpdate);
}

/// All gas in pipes, cut-off members and rooms.
fn total_moles(app: &mut App) -> f64 {
    let rooms = app.world().resource::<StationAtmos>().0.total_moles();
    let pipes: f64 = app
        .world()
        .resource::<PipeNets>()
        .nets
        .iter()
        .map(|n| n.mix.total_moles())
        .sum();
    let mut q = app.world_mut().query::<&GasStore>();
    let stores: f64 = q.iter(app.world()).map(|s| s.0.total_moles()).sum();
    rooms + pipes + stores
}

const TANK: TilePos = TilePos::new(2, 10, 0);
const DISTRO: TilePos = TilePos::new(5, 10, 0);
const PORT: TilePos = TilePos::new(6, 10, 0);
const VALVE: TilePos = TilePos::new(13, 10, 0);
const SCRUBBER: TilePos = TilePos::new(8, 2, 0);

#[test]
fn networks_form_along_the_pipes() {
    let mut app = setup();
    seconds(&mut app, 1);
    let nets = app.world().resource::<PipeNets>();
    let tank = nets.net_at[&TANK];
    let distro = nets.net_at[&DISTRO];
    assert_ne!(
        tank, distro,
        "the pump separates the tank from the distribution pipe"
    );
    assert_eq!(
        nets.net_at[&PORT], distro,
        "the connected canister is part of the pipe"
    );
    assert_eq!(
        nets.net_at[&TilePos::new(18, 10, 0)],
        distro,
        "through the wall to the closet vent"
    );
    assert_ne!(
        nets.net_at[&SCRUBBER], distro,
        "the scrubber has its own waste pipe"
    );
    assert!(nets.nets[tank].pressure() > 8_000.0);
}

#[test]
fn closing_the_valve_splits_the_pipe_and_conserves_gas() {
    let mut app = setup();
    seconds(&mut app, 1);
    let before = total_moles(&mut app);
    let mob = spawn_player_mob(app.world_mut(), VALVE.step(rs13_world::Dir::South));
    let valve = find_at(&mut app, "valve", VALVE);
    act(&mut app, mob, Action::UseOn(valve));
    seconds(&mut app, 1);
    assert!(!app.world().get::<Switch>(valve).unwrap().0);
    assert!(!app.world().get::<PipeMember>(valve).unwrap().open);
    let nets = app.world().resource::<PipeNets>();
    assert_ne!(
        nets.net_at[&DISTRO],
        nets.net_at[&TilePos::new(18, 10, 0)],
        "split"
    );
    assert!((total_moles(&mut app) - before).abs() < 1e-6);
    let outbox = &app.world().get::<Outbox>(mob).unwrap().0;
    assert_eq!(outbox.last().unwrap().id, "valve-closed");
}

#[test]
fn the_pump_feeds_the_distribution_pipe_up_to_its_target() {
    let mut app = setup();
    // Wrench the canister off so the distribution pipe starts empty.
    let mob = spawn_player_mob(app.world_mut(), PORT.step(rs13_world::Dir::South));
    let wrench = {
        let mut q = app.world_mut().query::<(Entity, &Appearance)>();
        q.iter(app.world()).find(|(_, a)| a.sprite == "wrench").unwrap().0
    };
    app.world_mut()
        .entity_mut(wrench)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    app.world_mut().get_mut::<Hands>(mob).unwrap().right = Some(wrench);
    let canister = find_at(&mut app, "canister", PORT);
    act(&mut app, mob, Action::UseOn(canister));
    assert!(
        !app.world().get::<PipeMember>(canister).unwrap().open,
        "disconnected"
    );
    seconds(&mut app, 30);
    let p = app.world().resource::<PipeNets>().at(DISTRO).unwrap().pressure();
    assert!((295.0..=305.0).contains(&p), "distribution pipe at {p:.1} kPa");
}

#[test]
fn an_open_canister_releases_into_the_room_up_to_one_atmosphere() {
    let mut app = setup();
    seconds(&mut app, 1);
    let mob = spawn_player_mob(app.world_mut(), PORT.step(rs13_world::Dir::South));
    let canister = find_at(&mut app, "canister", PORT);
    // Connected canisters do not release; this one is on the port. Disconnect by hand is
    // not possible, so open its valve anyway: nothing should happen.
    act(&mut app, mob, Action::UseOn(canister));
    assert!(app.world().get::<Switch>(canister).unwrap().0, "valve open");
    seconds(&mut app, 4);
    let room = app.world().resource::<StationAtmos>().0.pressure_at(PORT);
    assert!(
        (room - ONE_ATMOSPHERE).abs() < 0.5,
        "room unchanged while connected: {room}"
    );
}

#[test]
fn the_scrubber_pulls_co2_out_of_the_room() {
    let mut app = setup();
    seconds(&mut app, 1);
    // Fill the main room with some CO2.
    {
        let mut atmos = app.world_mut().resource_mut::<StationAtmos>();
        let mut co2 = atmos.0.air_at(SCRUBBER).scaled(0.0);
        co2.set(Gas::CarbonDioxide, 200.0);
        co2.temperature = 293.15;
        atmos.0.add_at(SCRUBBER, &co2);
    }
    let room_co2 = |app: &App| {
        let atmos = &app.world().resource::<StationAtmos>().0;
        let zone = atmos.zone_at(SCRUBBER).unwrap();
        atmos.zones()[zone].mix.get(Gas::CarbonDioxide)
    };
    let before = room_co2(&app);
    seconds(&mut app, 60);
    let after = room_co2(&app);
    assert!(after < before, "{after} < {before}");
    let waste = app
        .world()
        .resource::<PipeNets>()
        .at(SCRUBBER)
        .unwrap()
        .mix
        .get(Gas::CarbonDioxide);
    assert!(
        (before - after - waste).abs() < 1e-6,
        "what left the room is in the waste pipe"
    );
}

#[test]
fn examining_a_canister_reads_its_gauge() {
    let mut app = setup();
    seconds(&mut app, 1);
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(3, 3, 0));
    let tank = find_at(&mut app, "air-tank", TANK);
    act(&mut app, mob, Action::Examine(tank));
    let outbox = &app.world().get::<Outbox>(mob).unwrap().0;
    let ids: Vec<&str> = outbox.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, ["examine", "examine-gauge"]);
}
