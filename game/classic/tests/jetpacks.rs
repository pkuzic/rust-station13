//! Jetpacks. Spec: docs/reference/jetpacks.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::atmos::Drift;
use rs13_classic::equipment::{Equipment, TankGas, WornBy};
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::jetpacks::{Jetpack, THRUST_MOLES};
use rs13_classic::movement::MoveIntent;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_world::{Appearance, Dir, GridPos, ObjectSpec, Slot, TileMap, TilePos};

/// Open space with a wall in one corner, far from the middle.
const MAP: &str = r#"(
    legend: {
        ' ': (tile: (id: "space")),
        'W': (tile: (id: "wall", solid: true)),
    },
    rows: [
        "W        ",
        "         ",
        "         ",
        "         ",
        "         ",
        "         ",
        "         ",
    ],
)"#;

const MIDDLE: TilePos = TilePos::new(4, 3, 0);

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), MIDDLE);
    app.world_mut().run_schedule(FixedUpdate);
    (app, mob)
}

fn ticks(app: &mut App, n: u64) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn wear_jetpack(app: &mut App, mob: Entity, on: bool) -> Entity {
    let pack = spawn_object(app.world_mut(), &ObjectSpec::new("jetpack"), MIDDLE).unwrap();
    app.world_mut()
        .entity_mut(pack)
        .remove::<GridPos>()
        .insert((WornBy(mob), Jetpack { on }));
    app.world_mut()
        .get_mut::<Equipment>(mob)
        .unwrap()
        .0
        .insert(Slot::Back, pack);
    pack
}

/// Ask to move east for one tick, then wait out the step.
fn step_east(app: &mut App, mob: Entity) {
    app.world_mut().get_mut::<MoveIntent>(mob).unwrap().dir = Some(Dir::East);
    ticks(app, 1);
    app.world_mut().get_mut::<MoveIntent>(mob).unwrap().dir = None;
    ticks(app, 10);
}

fn at(app: &App, mob: Entity) -> TilePos {
    app.world().get::<GridPos>(mob).unwrap().0
}

fn moles(app: &App, pack: Entity) -> f64 {
    app.world().get::<TankGas>(pack).unwrap().mix.total_moles()
}

#[test]
fn without_a_jetpack_there_is_nothing_to_push_off() {
    let (mut app, mob) = setup();
    step_east(&mut app, mob);
    assert_eq!(at(&app, mob), MIDDLE);
    // Nor with one switched off.
    wear_jetpack(&mut app, mob, false);
    step_east(&mut app, mob);
    assert_eq!(at(&app, mob), MIDDLE);
}

#[test]
fn a_jetpack_pushes_its_wearer_and_uses_gas_each_step() {
    let (mut app, mob) = setup();
    let pack = wear_jetpack(&mut app, mob, true);
    let before = moles(&app, pack);
    step_east(&mut app, mob);
    assert_eq!(at(&app, mob), MIDDLE.step(Dir::East));
    let used = before - moles(&app, pack);
    assert!((used - THRUST_MOLES).abs() < 1e-9, "one push: {used}");

    // Out of gas, it does nothing.
    app.world_mut().get_mut::<TankGas>(pack).unwrap().mix =
        app.world().get::<TankGas>(pack).unwrap().mix.scaled(0.0);
    step_east(&mut app, mob);
    assert_eq!(at(&app, mob), MIDDLE.step(Dir::East));
}

#[test]
fn a_switched_on_jetpack_stops_drifting() {
    let (mut app, mob) = setup();
    app.world_mut().entity_mut(mob).insert(Drift(Dir::South));
    ticks(&mut app, 20);
    assert_ne!(at(&app, mob), MIDDLE, "drifts without one");

    let (mut app, mob) = setup();
    wear_jetpack(&mut app, mob, true);
    app.world_mut().entity_mut(mob).insert(Drift(Dir::South));
    ticks(&mut app, 20);
    assert_eq!(at(&app, mob), MIDDLE, "held still");
    assert!(app.world().get::<Drift>(mob).is_none());
}

#[test]
fn clicking_it_in_hand_switches_it() {
    let (mut app, mob) = setup();
    let pack = spawn_object(app.world_mut(), &ObjectSpec::new("jetpack"), MIDDLE).unwrap();
    app.world_mut()
        .entity_mut(pack)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    app.world_mut().get_mut::<Hands>(mob).unwrap().right = Some(pack);
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(pack));
    ticks(&mut app, 1);
    assert!(app.world().get::<Jetpack>(pack).unwrap().on);
    assert_eq!(app.world().get::<Appearance>(pack).unwrap().state, "on");
    let said: Vec<_> = app
        .world()
        .get::<Outbox>(mob)
        .unwrap()
        .0
        .iter()
        .map(|m| m.id.clone())
        .collect();
    assert!(said.contains(&"jetpack-on".to_owned()), "{said:?}");
}
