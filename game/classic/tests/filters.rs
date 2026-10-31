//! Gas filters and outlet injectors, set up by their map settings. Spec:
//! docs/reference/pipes.md.

use bevy_app::{App, FixedUpdate};
use rs13_classic::ClassicPlugin;
use rs13_classic::atmos::StationAtmos;
use rs13_classic::machines::{FILTER_TARGET, MACHINE_TICKS, PipeNets};
use rs13_classic::objects::spawn_map;
use rs13_simulation::atmos::Gas;
use rs13_world::{TileMap, TilePos};

/// An air tank feeding a filter facing east. The filtered gas goes south, the rest east.
/// Below, a tank feeding an injector in a closed room.
fn map(filter_type: u8) -> String {
    format!(
        r#"(
    legend: {{
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'T': (tile: (id: "floor"), objects: [(id: "air-tank", dir: East)]),
        '-': (tile: (id: "floor"), objects: [(id: "pipe", dir: East)]),
        '|': (tile: (id: "floor"), objects: [(id: "pipe", dir: North)]),
        'F': (tile: (id: "floor"), objects: [(id: "gas-filter", dir: East, vars: {{"filter_type": "{filter_type}", "on": "1"}})]),
        'I': (tile: (id: "floor"), objects: [(id: "injector", dir: West, vars: {{"volume_rate": "500", "on": "1"}})]),
    }},
    rows: [
        "WWWWWWW",
        "WT-F-.W",
        "W..|..W",
        "WWWWWWW",
        "WT-I..W",
        "WWWWWWW",
    ],
)"#
    )
}

const OUT: TilePos = TilePos::new(4, 4, 0);
const SIDE: TilePos = TilePos::new(3, 3, 0);
const INJECTOR: TilePos = TilePos::new(3, 1, 0);

fn setup(filter_type: u8) -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(&map(filter_type), 0).unwrap());
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

#[test]
fn an_oxygen_filter_sends_oxygen_aside_and_the_rest_on() {
    let mut app = setup(1);
    cycles(&mut app, 3);
    assert!(moles(&app, SIDE, Gas::Oxygen) > 0.0, "oxygen goes aside");
    assert_eq!(moles(&app, SIDE, Gas::Nitrogen), 0.0);
    assert!(moles(&app, OUT, Gas::Nitrogen) > 0.0, "the rest goes on");
    assert_eq!(moles(&app, OUT, Gas::Oxygen), 0.0);
    let out = app.world().resource::<PipeNets>().at(OUT).unwrap().pressure();
    assert!((out - FILTER_TARGET).abs() < 1.0, "up to its target: {out}");
}

#[test]
fn lunas_plasma_filter_setting_takes_nothing_out() {
    let mut app = setup(5);
    cycles(&mut app, 3);
    assert_eq!(moles(&app, SIDE, Gas::Oxygen), 0.0);
    assert!(moles(&app, OUT, Gas::Oxygen) > 0.0);
}

#[test]
fn an_injector_lets_its_network_out_into_the_room() {
    let mut app = setup(1);
    let before = app.world().resource::<StationAtmos>().0.pressure_at(INJECTOR);
    cycles(&mut app, 3);
    let after = app.world().resource::<StationAtmos>().0.pressure_at(INJECTOR);
    assert!(after > before + 50.0, "{before} → {after}");
}
