//! The thermo-electric generator and thermal plates. Spec: docs/reference/engine.md.

use bevy_app::{App, FixedUpdate};
use rs13_classic::ClassicPlugin;
use rs13_classic::atmos::StationAtmos;
use rs13_classic::machines::{MACHINE_TICKS, PipeNets};
use rs13_classic::objects::spawn_map;
use rs13_classic::power::PowerSupply;
use rs13_classic::teg::Teg;
use rs13_simulation::atmos::gas::T20C;
use rs13_world::{TileMap, TilePos};

/// A generator between two air tanks, each through its generator input; below, a thermal
/// plate in a closed room, on a short pipe.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'h': (tile: (id: "floor"), objects: [(id: "air-tank", dir: East)]),
        '<': (tile: (id: "floor"), objects: [(id: "generator-input", dir: West)]),
        'G': (tile: (id: "floor"), objects: [(id: "teg", vars: {"transferpercent": "70", "outputpercent": "1"})]),
        '>': (tile: (id: "floor"), objects: [(id: "generator-input", dir: East)]),
        'c': (tile: (id: "floor"), objects: [(id: "air-tank", dir: West)]),
        't': (tile: (id: "floor"), objects: [(id: "pipe", dir: East)]),
        'P': (tile: (id: "floor"), objects: [(id: "thermal-plate", dir: West)]),
    },
    rows: [
        "WWWWWWW",
        "Wh<G>cW",
        "WWWWWWW",
        "WtP...W",
        "WWWWWWW",
    ],
)"#;

const HOT_TANK: TilePos = TilePos::new(1, 3, 0);
const COLD_TANK: TilePos = TilePos::new(5, 3, 0);
const PLATE_TANK: TilePos = TilePos::new(1, 1, 0);
const ROOM: TilePos = TilePos::new(4, 1, 0);

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().run_schedule(FixedUpdate);
    app
}

fn cycles(app: &mut App, n: u64) {
    for _ in 0..n * MACHINE_TICKS {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn set_temperature(app: &mut App, at: TilePos, t: f64) {
    let mut nets = app.world_mut().resource_mut::<PipeNets>();
    let id = nets.net_at[&at];
    nets.nets[id].mix.temperature = t;
}

fn temperature(app: &App, at: TilePos) -> f64 {
    app.world().resource::<PipeNets>().at(at).unwrap().mix.temperature
}

fn generator(app: &mut App) -> (Teg, f64) {
    let mut q = app.world_mut().query::<(&Teg, &PowerSupply)>();
    let (teg, supply) = q.single(app.world()).unwrap();
    (*teg, supply.watts)
}

#[test]
fn equal_loops_make_no_power() {
    let mut app = setup();
    cycles(&mut app, 2);
    assert_eq!(generator(&mut app).1, 0.0);
}

#[test]
fn a_hot_and_a_cold_loop_make_power_and_even_out() {
    let mut app = setup();
    set_temperature(&mut app, HOT_TANK, 900.0);
    set_temperature(&mut app, COLD_TANK, 100.0);
    cycles(&mut app, 1);
    let (teg, watts) = generator(&mut app);
    assert!(watts > 0.0, "it makes power");
    assert_eq!(teg.last, watts);
    let [(hot, _), (cold, _)] = teg.loops.unwrap();
    assert!(hot > cold);
    cycles(&mut app, 5);
    assert!(temperature(&app, HOT_TANK) < 900.0, "the hot loop cools");
    assert!(temperature(&app, COLD_TANK) > 100.0, "the cold loop warms");
}

#[test]
fn heat_taken_from_the_hot_loop_is_power_plus_heat_given_to_the_cold() {
    let mut app = setup();
    set_temperature(&mut app, HOT_TANK, 900.0);
    set_temperature(&mut app, COLD_TANK, 100.0);
    let energy = |app: &App, at| {
        let nets = app.world().resource::<PipeNets>();
        nets.at(at).unwrap().mix.thermal_energy()
    };
    let (hot0, cold0) = (energy(&app, HOT_TANK), energy(&app, COLD_TANK));
    cycles(&mut app, 1);
    let watts = generator(&mut app).1;
    let (hot1, cold1) = (energy(&app, HOT_TANK), energy(&app, COLD_TANK));
    let electricity = watts * rs13_classic::power::POWER_DT;
    let lost = hot0 - hot1;
    let gained = cold1 - cold0;
    assert!(
        (lost - gained - electricity).abs() / lost < 1e-6,
        "{lost} = {gained} + {electricity}"
    );
}

#[test]
fn a_thermal_plate_carries_a_rooms_heat_into_its_pipes() {
    let mut app = setup();
    // Heat the room.
    {
        let mut atmos = app.world_mut().resource_mut::<StationAtmos>();
        let mut air = atmos.0.remove_at(ROOM, 1e9);
        air.temperature = 800.0;
        atmos.0.add_at(ROOM, &air);
    }
    {
        let mut nets = app.world_mut().resource_mut::<PipeNets>();
        let id = nets.net_at[&PLATE_TANK];
        let volume = nets.nets[id].volume;
        nets.nets[id].mix = rs13_classic::machines::air_at_pressure(volume, 101.325);
    }
    let before = temperature(&app, PLATE_TANK);
    assert!((before - T20C).abs() < 1.0);
    cycles(&mut app, 10);
    assert!(
        temperature(&app, PLATE_TANK) > before + 5.0,
        "{}",
        temperature(&app, PLATE_TANK)
    );
}
