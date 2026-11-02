//! Conveyor belts and switches. Spec: docs/reference/conveyors.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::conveyors::{Conveyor, ConveyorSwitch};
use rs13_classic::machines::MACHINE_TICKS;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_world::{Appearance, GridPos, ObjectSpec, TileMap, TilePos};

/// A belt running east along row 1 into a wall, its switch below it, and a corner at its
/// west end that turns things coming down from the north onto it.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'e': (tile: (id: "floor"), objects: [(id: "conveyor", dir: East, vars: {"id": "belt"})]),
        'c': (tile: (id: "floor"), objects: [(id: "conveyor", dir: NorthEast, vars: {"id": "belt"})]),
        's': (tile: (id: "floor"), objects: [(id: "conveyor-switch", vars: {"id": "belt"})]),
        'o': (tile: (id: "floor"), objects: [(id: "conveyor-switch", vars: {"id": "belt", "oneway": "1"})]),
    },
    rows: [
        "WWWWWWW",
        "Wceee.W",
        "W.so..W",
        "WWWWWWW",
    ],
)"#;

const CORNER: TilePos = TilePos::new(1, 2, 0);
const START: TilePos = TilePos::new(2, 2, 0);
const BY_SWITCH: TilePos = TilePos::new(2, 1, 0);

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), BY_SWITCH);
    app.world_mut().run_schedule(FixedUpdate);
    (app, mob)
}

fn cycles(app: &mut App, n: u64) {
    for _ in 0..n * MACHINE_TICKS {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn pos(app: &App, e: Entity) -> TilePos {
    app.world().get::<GridPos>(e).unwrap().0
}

fn switch_at(app: &mut App, at: TilePos) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &GridPos, &ConveyorSwitch)>();
    q.iter(app.world()).find(|(_, p, _)| p.0 == at).unwrap().0
}

fn throw(app: &mut App, mob: Entity, switch: Entity) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(switch));
    app.world_mut().run_schedule(FixedUpdate);
}

#[test]
fn a_running_belt_carries_things_until_a_wall_stops_them() {
    let (mut app, mob) = setup();
    let crowbar = spawn_object(app.world_mut(), &ObjectSpec::new("crowbar"), START).unwrap();
    let switch = switch_at(&mut app, TilePos::new(2, 1, 0));
    cycles(&mut app, 2);
    assert_eq!(pos(&app, crowbar), START, "stopped belts carry nothing");

    // Forward: east a tile a cycle, and the belt shows it runs.
    throw(&mut app, mob, switch);
    cycles(&mut app, 1);
    assert_eq!(pos(&app, crowbar), TilePos::new(3, 2, 0));
    let belt_look = {
        let mut q = app.world_mut().query::<(&Conveyor, &Appearance)>();
        q.iter(app.world()).next().unwrap().1.state.clone()
    };
    assert!(belt_look.starts_with("conveyor1-"), "{belt_look}");
    cycles(&mut app, 4);
    // Off the last belt onto the floor, and it stays there.
    assert_eq!(pos(&app, crowbar), TilePos::new(5, 2, 0));

    // The other switch with its id shows the same; off, then reverse.
    let other = switch_at(&mut app, TilePos::new(3, 1, 0));
    assert_eq!(app.world().get::<ConveyorSwitch>(other).unwrap().position, 1);
    throw(&mut app, mob, switch);
    throw(&mut app, mob, switch);
    assert_eq!(app.world().get::<ConveyorSwitch>(switch).unwrap().position, -1);
    let wrench = spawn_object(app.world_mut(), &ObjectSpec::new("wrench"), TilePos::new(3, 2, 0)).unwrap();
    cycles(&mut app, 1);
    assert_eq!(pos(&app, wrench), START);
}

#[test]
fn a_corner_turns_things_and_people_ride_too() {
    let (mut app, mob) = setup();
    let oneway = switch_at(&mut app, TilePos::new(3, 1, 0));
    throw(&mut app, mob, oneway);
    // A north-east corner carries things east running forward.
    let rider = spawn_player_mob(app.world_mut(), CORNER);
    cycles(&mut app, 1);
    assert_eq!(pos(&app, rider), START);
    cycles(&mut app, 1);
    assert_eq!(pos(&app, rider), TilePos::new(3, 2, 0));
    // The one-way switch only goes forward and off.
    throw(&mut app, mob, oneway);
    assert_eq!(app.world().get::<ConveyorSwitch>(oneway).unwrap().position, 0);
    throw(&mut app, mob, oneway);
    assert_eq!(app.world().get::<ConveyorSwitch>(oneway).unwrap().position, 1);
}
