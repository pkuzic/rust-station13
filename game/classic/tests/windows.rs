//! Window panes along one edge of their tile. Spec: docs/reference/windows.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::atmos::StationAtmos;
use rs13_classic::lighting::TileLight;
use rs13_classic::movement::MoveIntent;
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_world::{Dir, GridPos, TileMap, TilePos};

/// Two rooms, south (y = 1) and north (y = 3), joined by one floor tile at (2, 2) with a
/// pane along its north edge. A lamp in the north room.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'l': (tile: (id: "floor"), objects: [(id: "light", dir: North)]),
        'p': (tile: (id: "floor"), objects: [(id: "window-pane", dir: North)]),
    },
    rows: [
        "WWWWW",
        "W.l.W",
        "WWpWW",
        "W...W",
        "WWWWW",
    ],
)"#;

const SOUTH: TilePos = TilePos::new(2, 1, 0);
const PANE: TilePos = TilePos::new(2, 2, 0);
const NORTH: TilePos = TilePos::new(2, 3, 0);

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().run_schedule(FixedUpdate);
    app
}

fn walk(app: &mut App, mob: Entity, dir: Dir) -> TilePos {
    app.world_mut().get_mut::<MoveIntent>(mob).unwrap().dir = Some(dir);
    for _ in 0..20 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    app.world_mut().get_mut::<MoveIntent>(mob).unwrap().dir = None;
    app.world_mut().run_schedule(FixedUpdate);
    app.world().get::<GridPos>(mob).unwrap().0
}

#[test]
fn a_pane_stops_people_crossing_its_edge_either_way() {
    let mut app = setup();
    let mob = spawn_player_mob(app.world_mut(), SOUTH);
    assert_eq!(walk(&mut app, mob, Dir::North), PANE, "onto the pane's tile");
    assert_eq!(walk(&mut app, mob, Dir::North), PANE, "but not across its edge");
    let other = spawn_player_mob(app.world_mut(), NORTH);
    assert_eq!(
        walk(&mut app, other, Dir::South),
        NORTH,
        "nor across it from the far side"
    );
}

#[test]
fn a_pane_seals_its_edge_against_air_but_not_against_light() {
    let app = setup();
    let atmos = &app.world().resource::<StationAtmos>().0;
    assert_eq!(atmos.zone_at(SOUTH), atmos.zone_at(PANE), "its own side is open");
    assert_ne!(
        atmos.zone_at(PANE),
        atmos.zone_at(NORTH),
        "the far side is sealed off"
    );
    let light = app.world().resource::<TileLight>();
    assert!(!light.is_dark(PANE), "the lamp's light comes through the glass");
}
