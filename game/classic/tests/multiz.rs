//! Ladders and stairs between decks, on a station of two decks. Spec:
//! docs/reference/multiz.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::atmos::StationAtmos;
use rs13_classic::movement::MoveIntent;
use rs13_classic::multiz::Ladder;
use rs13_classic::objects::{spawn_player_mob, spawn_station};
use rs13_world::{Dir, GridPos, TileMap, TilePos};

/// Deck 0, the lower: a room with the bottom end of a ladder at (1, 1) and the bottom
/// stairs at (4, 1)–(4, 2): entry (4, 1), active (4, 2).
const LOWER: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'L': (tile: (id: "floor"), objects: [(id: "ladder", vars: {"istop": "0"})]),
        'e': (tile: (id: "floor"), objects: [(id: "stairs-entry-bottom", dir: North)]),
        'a': (tile: (id: "floor"), objects: [(id: "stairs-up", dir: North)]),
    },
    rows: [
        "WWWWWW",
        "W...aW",
        "WL..eW",
        "WWWWWW",
    ],
)"#;

/// Deck 1, the upper: the top end of the ladder above the bottom end, and the top stairs
/// mirrored over the bottom ones: entry (4, 2) over the bottom active tile, active (4, 1)
/// over the bottom entry.
const UPPER: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'L': (tile: (id: "floor"), objects: [(id: "ladder", vars: {"istop": "1"})]),
        'e': (tile: (id: "floor"), objects: [(id: "stairs-entry", dir: North)]),
        'a': (tile: (id: "floor"), objects: [(id: "stairs-down", dir: North)]),
    },
    rows: [
        "WWWWWW",
        "W...eW",
        "WL..aW",
        "WWWWWW",
    ],
)"#;

fn station() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let decks = vec![
        TileMap::from_ron(LOWER, 0).unwrap(),
        TileMap::from_ron(UPPER, 1).unwrap(),
    ];
    let problems = spawn_station(app.world_mut(), decks);
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().run_schedule(FixedUpdate);
    app
}

fn at(app: &App, mob: Entity) -> TilePos {
    app.world().get::<GridPos>(mob).unwrap().0
}

fn walk(app: &mut App, mob: Entity, dir: Dir) {
    app.world_mut().get_mut::<MoveIntent>(mob).unwrap().dir = Some(dir);
    for _ in 0..12 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    app.world_mut().get_mut::<MoveIntent>(mob).unwrap().dir = None;
    for _ in 0..4 {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn ladder(app: &mut App, down: bool) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &Ladder)>();
    q.iter(app.world()).find(|(_, l)| l.down == down).unwrap().0
}

fn click(app: &mut App, mob: Entity, target: Entity) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(FixedUpdate);
}

#[test]
fn a_ladder_takes_you_up_and_back_down() {
    let mut app = station();
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(2, 1, 0));
    let bottom = ladder(&mut app, false);
    click(&mut app, mob, bottom);
    assert_eq!(at(&app, mob), TilePos::new(1, 1, 1), "up to the top end");
    let top = ladder(&mut app, true);
    click(&mut app, mob, top);
    assert_eq!(at(&app, mob), TilePos::new(1, 1, 0), "and back down");
}

#[test]
fn stairs_take_you_up_from_their_entry_and_down_again() {
    let mut app = station();
    // Beside the bottom stairs, not on their entry: walking into them does nothing.
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(3, 2, 0));
    walk(&mut app, mob, Dir::East);
    assert_eq!(at(&app, mob), TilePos::new(3, 2, 0), "the active tile is solid");
    // Onto the entry, then north into the active tile: up a deck, onto the top entry.
    walk(&mut app, mob, Dir::South);
    walk(&mut app, mob, Dir::East);
    assert_eq!(at(&app, mob), TilePos::new(4, 1, 0), "on the entry");
    walk(&mut app, mob, Dir::North);
    assert_eq!(at(&app, mob), TilePos::new(4, 2, 1), "up the stairs");
    // From the top entry south into the top active tile: back down, onto the bottom entry.
    walk(&mut app, mob, Dir::South);
    assert_eq!(at(&app, mob), TilePos::new(4, 1, 0), "down the stairs");
}

#[test]
fn each_deck_has_its_own_air() {
    let app = station();
    let atmos = &app.world().resource::<StationAtmos>().0;
    assert_ne!(
        atmos.zone_at(TilePos::new(2, 1, 0)),
        atmos.zone_at(TilePos::new(2, 1, 1))
    );
}
