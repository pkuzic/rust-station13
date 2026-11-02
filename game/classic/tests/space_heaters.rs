//! Space heaters. Spec: docs/reference/space-heaters.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::atmos::StationAtmos;
use rs13_classic::cells::PowerCell;
use rs13_classic::hands::Hands;
use rs13_classic::heaters::{HeaterOp, HeaterPanelOut, PendingHeaterOps, SpaceHeater, WARMEST};
use rs13_classic::machines::MACHINE_TICKS;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_simulation::atmos::gas::T0C;
use rs13_world::{Appearance, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'h': (tile: (id: "floor"), objects: ["space-heater"]),
    },
    rows: [
        "WWWW",
        "W.hW",
        "WWWW",
    ],
)"#;

const HERE: TilePos = TilePos::new(1, 1, 0);
const HEATER: TilePos = TilePos::new(2, 1, 0);

fn setup() -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), HERE);
    app.world_mut().run_schedule(FixedUpdate);
    let heater = app
        .world_mut()
        .query_filtered::<Entity, With<SpaceHeater>>()
        .single(app.world())
        .unwrap();
    (app, mob, heater)
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

fn op(app: &mut App, mob: Entity, heater: Entity, op: HeaterOp) -> Vec<String> {
    let before = app.world().get::<Outbox>(mob).unwrap().0.len();
    app.world_mut()
        .get_mut::<PendingHeaterOps>(mob)
        .unwrap()
        .0
        .push((heater, op));
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(FixedUpdate);
    app.world().get::<Outbox>(mob).unwrap().0[before..]
        .iter()
        .map(|m| m.id.clone())
        .collect()
}

fn temperature(app: &App, at: TilePos) -> f64 {
    app.world().resource::<StationAtmos>().0.air_at(at).temperature
}

fn cycles(app: &mut App, n: u64) {
    for _ in 0..n * MACHINE_TICKS {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

#[test]
fn a_heater_warms_the_air_on_its_cell_and_stops_at_its_setting() {
    let (mut app, mob, heater) = setup();
    let start = temperature(&app, HEATER);
    assert_eq!(act(&mut app, mob, Action::UseOn(heater)), ["heater-switched-on"]);
    assert_eq!(app.world().get::<Appearance>(heater).unwrap().state, "sheater1");
    let cell = app.world().get::<SpaceHeater>(heater).unwrap().cell.unwrap();
    let full = app.world().get::<PowerCell>(cell).unwrap().charge;
    cycles(&mut app, 10);
    let warmed = temperature(&app, HEATER);
    assert!(warmed > start + 2.0, "warmer: {start} -> {warmed}");
    assert!(app.world().get::<PowerCell>(cell).unwrap().charge < full);
    // It never heats past its setting (50 °C) by more than one cycle's worth.
    cycles(&mut app, 600);
    let hot = temperature(&app, HEATER);
    assert!(hot < 50.0 + T0C + 30.0, "{hot}");

    // An empty cell switches it off.
    app.world_mut().get_mut::<PowerCell>(cell).unwrap().charge = 0.0;
    cycles(&mut app, 1);
    assert!(!app.world().get::<SpaceHeater>(heater).unwrap().on);
}

#[test]
fn its_hatch_opens_to_its_cell_and_its_setting() {
    let (mut app, mob, heater) = setup();
    let cell = spawn_object(app.world_mut(), &ObjectSpec::new("power-cell"), HERE).unwrap();
    act(&mut app, mob, Action::UseOn(cell));
    assert_eq!(act(&mut app, mob, Action::UseOn(heater)), ["heater-hatch-closed"]);
    act(&mut app, mob, Action::SwapHands);
    let driver = spawn_object(app.world_mut(), &ObjectSpec::new("screwdriver"), HERE).unwrap();
    act(&mut app, mob, Action::UseOn(driver));
    assert_eq!(act(&mut app, mob, Action::UseOn(heater)), ["heater-hatch-opened"]);
    assert_eq!(
        app.world().get::<Appearance>(heater).unwrap().state,
        "sheater-open"
    );
    act(&mut app, mob, Action::SwapHands);
    assert_eq!(act(&mut app, mob, Action::UseOn(heater)), ["heater-has-cell"]);
    act(&mut app, mob, Action::Drop);

    // An empty hand opens its window; its cell comes out, and the new one goes in.
    act(&mut app, mob, Action::UseOn(heater));
    app.world_mut().run_schedule(FixedUpdate);
    let view = app
        .world()
        .get::<HeaterPanelOut>(mob)
        .unwrap()
        .0
        .clone()
        .expect("window");
    assert!(view.has_cell);
    assert_eq!(view.setting, 50);
    assert_eq!(op(&mut app, mob, heater, HeaterOp::Cell), ["heater-cell-out"]);
    assert!(app.world().get::<SpaceHeater>(heater).unwrap().cell.is_none());
    act(&mut app, mob, Action::Drop);
    act(&mut app, mob, Action::UseOn(cell));
    assert_eq!(op(&mut app, mob, heater, HeaterOp::Cell), ["heater-cell-in"]);
    assert_eq!(app.world().get::<SpaceHeater>(heater).unwrap().cell, Some(cell));
    assert_eq!(app.world().get::<Hands>(mob).unwrap().active_item(), None);

    // The setting goes 5 degrees at a time, up to 90.
    for _ in 0..20 {
        op(&mut app, mob, heater, HeaterOp::Setting(1));
    }
    assert_eq!(app.world().get::<SpaceHeater>(heater).unwrap().setting, WARMEST);

    // Shut, the window closes, and a click switches it on.
    act(&mut app, mob, Action::SwapHands);
    assert_eq!(act(&mut app, mob, Action::UseOn(heater)), ["heater-hatch-shut"]);
    app.world_mut().run_schedule(FixedUpdate);
    assert!(app.world().get::<HeaterPanelOut>(mob).unwrap().0.is_none());
}
