//! Portable air pumps and scrubbers. Spec: docs/reference/portables.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::atmos::StationAtmos;
use rs13_classic::machines::{GasStore, PipeMember, Switch};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::portables::{PUMP_TARGET, PUMP_VOLUME, PortablePump, PortableScrubber};
use rs13_simulation::atmos::gas::T20C;
use rs13_simulation::atmos::{Gas, GasMix};
use rs13_world::{Appearance, ObjectSpec, TileMap, TilePos};

/// A closed room with a pump, a scrubber, and a connector port.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'p': (tile: (id: "floor"), objects: ["portable-pump"]),
        's': (tile: (id: "floor"), objects: ["portable-scrubber"]),
        'c': (tile: (id: "floor"), objects: ["pipe", "connector"]),
    },
    rows: [
        "WWWWWWW",
        "W.p.s.W",
        "W.....W",
        "W...c.W",
        "WWWWWWW",
    ],
)"#;

const SCRUBBER_AT: TilePos = TilePos::new(4, 3, 0);

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().run_schedule(FixedUpdate);
    app
}

fn seconds(app: &mut App, s: u64) {
    for _ in 0..s * 20 {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn one<C: Component>(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<C>>()
        .single(app.world())
        .unwrap()
}

fn act(app: &mut App, mob: Entity, action: Action) -> Vec<String> {
    let before = app.world().get::<Outbox>(mob).unwrap().0.len();
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
    app.world_mut().run_schedule(FixedUpdate);
    app.world().get::<Outbox>(mob).unwrap().0[before..]
        .iter()
        .map(|m| m.id.clone())
        .collect()
}

/// The power switch in its window (`gas_panel.rs`).
fn power(app: &mut App, mob: Entity, machine: Entity) {
    app.world_mut()
        .get_mut::<rs13_classic::gas_panel::PendingGasOps>(mob)
        .unwrap()
        .0
        .push((machine, rs13_classic::gas_panel::GasOp::Toggle));
    app.world_mut().run_schedule(FixedUpdate);
}

#[test]
fn a_pump_siphons_the_room_into_its_tank() {
    let mut app = setup();
    let pump = one::<PortablePump>(&mut app);
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(2, 2, 0));
    let before = app.world().resource::<StationAtmos>().0.total_moles();
    power(&mut app, mob, pump);
    assert_eq!(app.world().get::<Switch>(pump), Some(&Switch(true)));
    seconds(&mut app, 30);
    assert_eq!(app.world().get::<Appearance>(pump).unwrap().state, "psiphon:1");
    let tank = app.world().get::<GasStore>(pump).unwrap().0;
    let kpa = tank.pressure(PUMP_VOLUME);
    assert!((kpa - PUMP_TARGET).abs() < 5.0, "up to its target: {kpa:.1} kPa");
    let room = app.world().resource::<StationAtmos>().0.total_moles();
    assert!(
        (before - room - tank.total_moles()).abs() < 1e-6,
        "what left the room is in the tank"
    );
}

#[test]
fn a_scrubber_takes_plasma_out_of_the_air() {
    let mut app = setup();
    let mut plasma = GasMix::default();
    plasma.temperature = T20C;
    plasma.set(Gas::Plasma, 20.0);
    app.world_mut()
        .resource_mut::<StationAtmos>()
        .0
        .add_at(SCRUBBER_AT, &plasma);
    let scrubber = one::<PortableScrubber>(&mut app);
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(4, 2, 0));
    power(&mut app, mob, scrubber);
    // Each cycle (2 s) it clears its tile's share of the room: a fifteenth of it here.
    seconds(&mut app, 120);
    let room = app.world().resource::<StationAtmos>().0.air_at(SCRUBBER_AT);
    assert!(
        room.get(Gas::Plasma) < 0.05,
        "scrubbed: {}",
        room.get(Gas::Plasma)
    );
    assert!(room.get(Gas::Oxygen) > 15.0, "the air stays");
    let tank = app.world().get::<GasStore>(scrubber).unwrap().0;
    assert!(
        tank.get(Gas::Plasma) > 19.0,
        "in its tank: {}",
        tank.get(Gas::Plasma)
    );
    assert_eq!(tank.get(Gas::Oxygen), 0.0);
}

#[test]
fn a_wrench_fastens_it_to_a_port() {
    let mut app = setup();
    let pump = one::<PortablePump>(&mut app);
    let port_at = TilePos::new(4, 1, 0);
    app.world_mut()
        .entity_mut(pump)
        .insert(rs13_world::GridPos(port_at));
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(3, 1, 0));
    let wrench = spawn_object(app.world_mut(), &ObjectSpec::new("wrench"), TilePos::new(3, 1, 0)).unwrap();
    act(&mut app, mob, Action::UseOn(wrench));
    assert_eq!(act(&mut app, mob, Action::UseOn(pump)), ["canister-connected"]);
    assert!(app.world().get::<PipeMember>(pump).unwrap().open);
    seconds(&mut app, 3);
    assert_eq!(
        app.world().get::<Appearance>(pump).unwrap().state,
        "psiphon:0+siphon-connector"
    );
    assert_eq!(act(&mut app, mob, Action::UseOn(pump)), ["canister-disconnected"]);
}
