//! Gas mixers, and gas pumps set up by Luna's map settings. Spec: docs/reference/pipes.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::machines::{GasMixer, MACHINE_TICKS, PipeNets, Pump, Switch};
use rs13_classic::objects::spawn_map;
use rs13_simulation::atmos::Gas;
use rs13_world::{Appearance, Dir, TileMap, TilePos};

/// An oxygen tank behind a mixer facing east, a nitrogen tank on its side (south), the
/// outlet east. Below, an air tank behind a pump as Luna's map sets one: on, to 4000 kPa.
fn map(mixer_on: &str) -> String {
    format!(
        r#"(
    legend: {{
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'O': (tile: (id: "floor"), objects: [(id: "oxygen-pressure-tank", dir: East)]),
        'N': (tile: (id: "floor"), objects: [(id: "nitrogen-pressure-tank", dir: North)]),
        'T': (tile: (id: "floor"), objects: [(id: "air-tank", dir: East)]),
        '-': (tile: (id: "floor"), objects: [(id: "pipe", dir: East)]),
        '|': (tile: (id: "floor"), objects: [(id: "pipe", dir: North)]),
        'M': (tile: (id: "floor"), objects: [(id: "gas-mixer", dir: East, vars: {{"on": "{mixer_on}", "target_pressure": "500", "node1_concentration": "0.25", "node2_concentration": "0.75"}})]),
        'P': (tile: (id: "floor"), objects: [(id: "pump", dir: East, vars: {{"on": "1", "target_pressure": "4000"}})]),
    }},
    rows: [
        "WWWWWWW",
        "WO-M-.W",
        "W..|..W",
        "W..N..W",
        "WWWWWWW",
        "WT-P-.W",
        "WWWWWWW",
    ],
)"#
    )
}

const MIXED: TilePos = TilePos::new(4, 5, 0);
const PUMPED: TilePos = TilePos::new(4, 1, 0);

fn setup(mixer_on: &str) -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(&map(mixer_on), 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().run_schedule(FixedUpdate);
    app
}

fn cycles(app: &mut App, n: u64) {
    for _ in 0..n * MACHINE_TICKS {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn moles(app: &App, at: TilePos, gas: Gas) -> f64 {
    app.world().resource::<PipeNets>().at(at).unwrap().mix.get(gas)
}

fn pressure(app: &App, at: TilePos) -> f64 {
    app.world().resource::<PipeNets>().at(at).unwrap().pressure()
}

fn mixer(app: &mut App) -> Entity {
    let mut q = app.world_mut().query_filtered::<Entity, With<GasMixer>>();
    q.single(app.world()).unwrap()
}

#[test]
fn a_mixer_fills_its_outlet_in_its_shares_up_to_its_target() {
    let mut app = setup("1");
    let m = mixer(&mut app);
    assert_eq!(app.world().get::<Appearance>(m).unwrap().state, "on-east");
    cycles(&mut app, 3);
    let (o2, n2) = (moles(&app, MIXED, Gas::Oxygen), moles(&app, MIXED, Gas::Nitrogen));
    let share = o2 / (o2 + n2);
    assert!((share - 0.25).abs() < 0.01, "a quarter oxygen: {share}");
    let out = pressure(&app, MIXED);
    assert!((out - 500.0).abs() < 5.0, "up to its target: {out}");
}

#[test]
fn a_mixer_left_off_does_nothing_until_switched_on() {
    let mut app = setup("0");
    let m = mixer(&mut app);
    cycles(&mut app, 2);
    assert_eq!(pressure(&app, MIXED), 0.0);
    assert_eq!(app.world().get::<Appearance>(m).unwrap().state, "off-east");
    app.world_mut().get_mut::<Switch>(m).unwrap().0 = true;
    cycles(&mut app, 2);
    assert!(pressure(&app, MIXED) > 400.0);
    assert_eq!(app.world().get::<Appearance>(m).unwrap().state, "on-east");
}

#[test]
fn a_pump_from_lunas_map_keeps_its_target_and_facing() {
    let mut app = setup("0");
    let mut q = app.world_mut().query::<(&Pump, &Appearance)>();
    let (pump, look) = q.single(app.world()).unwrap();
    assert_eq!((pump.dir, pump.target), (Dir::East, 4000.0));
    assert_eq!(look.state, "on-east");
    cycles(&mut app, 3);
    let out = pressure(&app, PUMPED);
    assert!((out - 4000.0).abs() < 40.0, "{out}");
}
