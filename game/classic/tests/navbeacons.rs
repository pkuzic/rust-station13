//! Navigation beacons, circuit boards and rechargers. Spec: docs/reference/navbeacons.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::hands::Hands;
use rs13_classic::navbeacons::NavBeacon;
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_classic::power::PowerUser;
use rs13_world::{Appearance, GridPos, Hidden, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'n': (tile: (id: "floor"), objects: [(id: "navbeacon", vars: {"location": "Bridge", "codes_txt": "patrol;next_patrol=Chapel"})]),
        'd': (tile: (id: "plating"), objects: [(id: "navbeacon", vars: {"freq": "1337", "codes_txt": "delivery;dir=4"})]),
        'r': (tile: (id: "floor"), objects: ["recharger"]),
        'c': (tile: (id: "floor"), objects: ["circuit-board"]),
    },
    rows: [
        "WWWWWWW",
        "Wndrc.W",
        "WWWWWWW",
    ],
)"#;

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().run_schedule(FixedUpdate);
    app
}

#[test]
fn beacons_keep_what_the_map_set_and_lie_under_the_floor() {
    let mut app = setup();
    let mut q = app.world_mut().query::<(&NavBeacon, &GridPos, Has<Hidden>)>();
    let beacons: Vec<_> = q.iter(app.world()).collect();
    let bridge = beacons.iter().find(|(_, p, _)| p.0.x == 1).unwrap();
    assert_eq!(bridge.0.frequency, 1445);
    assert_eq!(bridge.0.location, "Bridge");
    assert_eq!(bridge.0.codes["next_patrol"], "Chapel");
    assert!(bridge.2, "under floor tiles");
    let delivery = beacons.iter().find(|(_, p, _)| p.0.x == 2).unwrap();
    assert_eq!(delivery.0.frequency, 1337);
    assert_eq!(delivery.0.codes["dir"], "4");
    assert!(!delivery.2, "on bare plating");
}

#[test]
fn a_recharger_stands_ready_and_draws_power() {
    let mut app = setup();
    let mut q = app.world_mut().query::<(&Appearance, &PowerUser)>();
    let (look, _) = q
        .iter(app.world())
        .find(|(a, _)| a.sprite == "recharger")
        .expect("a recharger");
    assert_eq!(look.state, "recharger0");
}

#[test]
fn a_circuit_board_can_be_picked_up() {
    let mut app = setup();
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(5, 1, 0));
    let mut q = app.world_mut().query::<(Entity, &Appearance)>();
    let board = q
        .iter(app.world())
        .find(|(_, a)| a.sprite == "circuit-board")
        .unwrap()
        .0;
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(board));
    app.world_mut().run_schedule(FixedUpdate);
    let hands = app.world().get::<Hands>(mob).unwrap();
    assert_eq!(hands.active_item(), Some(board));
}
