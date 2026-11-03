//! Fixtures that only stand there. Spec: docs/reference/scenery.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::conveyors::Conveyor;
use rs13_classic::movement::MoveIntent;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::scenery::PlasticFlaps;
use rs13_world::{Appearance, Dense, Dir, GridPos, ObjectSpec, Opaque, TileMap, TilePos};

const MAP: &str = r#"(
    legend: { 'W': (tile: (id: "wall", solid: true)), '.': (tile: (id: "floor")) },
    rows: ["WWWWW", "W...W", "WWWWW"],
)"#;
const AT: TilePos = TilePos::new(1, 1, 0);

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app
}

#[test]
fn each_blocks_what_it_should() {
    let mut app = setup();
    for (proto, dense, opaque) in [
        ("shuttle-engine", true, true),
        ("bookcase", true, false),
        ("plastic-flaps", true, true),
        ("mirror", false, false),
    ] {
        let thing = spawn_object(app.world_mut(), &ObjectSpec::new(proto), AT).unwrap();
        assert_eq!(app.world().get::<Dense>(thing).is_some(), dense, "{proto}");
        assert_eq!(app.world().get::<Opaque>(thing).is_some(), opaque, "{proto}");
    }
}

/// A belt running east, with flaps across it, and a wall further on.
#[test]
fn flaps_let_a_conveyor_carry_things_through() {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let map = r#"(
        legend: {
            'W': (tile: (id: "wall", solid: true)),
            'c': (tile: (id: "floor"), objects: [(id: "conveyor", dir: East), "crowbar"]),
            'f': (tile: (id: "floor"), objects: [(id: "conveyor", dir: East), "plastic-flaps"]),
            'e': (tile: (id: "floor"), objects: [(id: "conveyor", dir: East)]),
            '.': (tile: (id: "floor")),
        },
        rows: ["WWWWWWW", "Wcfe..W", "WWWWWWW"],
    )"#;
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(map, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mut q = app.world_mut().query::<(Entity, &Appearance)>();
    let bar = q
        .iter(app.world())
        .find(|(_, a)| a.sprite == "crowbar")
        .map(|(e, _)| e)
        .unwrap();
    let mut belts = app.world_mut().query::<&mut Conveyor>();
    for mut belt in belts.iter_mut(app.world_mut()) {
        belt.operating = 1;
    }
    for _ in 0..400 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    let at = app.world().get::<GridPos>(bar).unwrap().0;
    assert!(at.x > 2, "{at:?}: through the flaps");
}

#[test]
fn flaps_stop_someone_walking_into_them() {
    let mut app = setup();
    let flaps = spawn_object(
        app.world_mut(),
        &ObjectSpec::new("plastic-flaps"),
        TilePos::new(2, 1, 0),
    )
    .unwrap();
    let mob = spawn_player_mob(app.world_mut(), AT);
    app.world_mut().get_mut::<MoveIntent>(mob).unwrap().dir = Some(Dir::East);
    for _ in 0..40 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    assert_eq!(app.world().get::<GridPos>(mob).unwrap().0, AT);
    assert!(app.world().get::<PlasticFlaps>(flaps).is_some());
}
