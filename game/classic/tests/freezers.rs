//! Freezers. Spec: docs/reference/engine.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::freezers::{COLDEST, Freezer, FreezerOp, FreezerPanelOut, PendingFreezerOps};
use rs13_classic::machines::{MACHINE_TICKS, PipeNets, air_at_pressure};
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_simulation::atmos::gas::T20C;
use rs13_world::{TileMap, TilePos};

/// A freezer on a short pipe of air, and room to stand beside it.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'p': (tile: (id: "floor"), objects: [(id: "pipe", dir: East)]),
        'F': (tile: (id: "floor"), objects: [(id: "freezer", dir: West)]),
    },
    rows: [
        "WWWWW",
        "WpF.W",
        "WWWWW",
    ],
)"#;

const PIPE: TilePos = TilePos::new(1, 1, 0);

fn setup() -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(3, 1, 0));
    app.world_mut().run_schedule(FixedUpdate);
    {
        let mut nets = app.world_mut().resource_mut::<PipeNets>();
        let id = nets.net_at[&PIPE];
        let volume = nets.nets[id].volume;
        nets.nets[id].mix = air_at_pressure(volume, 101.325);
    }
    let mut q = app.world_mut().query_filtered::<Entity, With<Freezer>>();
    let freezer = q.single(app.world()).unwrap();
    (app, mob, freezer)
}

fn cycles(app: &mut App, n: u64) {
    for _ in 0..n * MACHINE_TICKS {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn op(app: &mut App, mob: Entity, freezer: Entity, op: FreezerOp) {
    app.world_mut()
        .get_mut::<PendingFreezerOps>(mob)
        .unwrap()
        .0
        .push((freezer, op));
    app.world_mut().run_schedule(FixedUpdate);
}

fn gas_temperature(app: &App) -> f64 {
    app.world()
        .resource::<PipeNets>()
        .at(PIPE)
        .unwrap()
        .mix
        .temperature
}

#[test]
fn it_starts_off_at_twenty_degrees_and_does_nothing_until_set() {
    let (mut app, mob, freezer) = setup();
    assert_eq!(*app.world().get::<Freezer>(freezer).unwrap(), Freezer::default());
    op(&mut app, mob, freezer, FreezerOp::Toggle);
    cycles(&mut app, 5);
    assert!(
        (gas_temperature(&app) - T20C).abs() < 0.01,
        "set to 20 °C, it can't cool"
    );
}

#[test]
fn set_cold_and_on_it_chills_its_pipe() {
    let (mut app, mob, freezer) = setup();
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(freezer));
    cycles(&mut app, 1);
    assert!(
        app.world().get::<FreezerPanelOut>(mob).unwrap().0.is_some(),
        "the window opens"
    );

    for _ in 0..5 {
        op(&mut app, mob, freezer, FreezerOp::Setting(-100.0));
    }
    assert_eq!(
        app.world().get::<Freezer>(freezer).unwrap().setting,
        COLDEST,
        "no colder than -200 °C"
    );
    op(&mut app, mob, freezer, FreezerOp::Toggle);
    cycles(&mut app, 10);
    let t = gas_temperature(&app);
    assert!(t < T20C - 50.0, "{t} K");
    assert!(t >= COLDEST, "never past its setting");
}
