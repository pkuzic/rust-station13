//! Pipe vents: open ends of pipe networks. Spec: docs/reference/pipes.md.

use bevy_app::{App, FixedUpdate};
use rs13_classic::ClassicPlugin;
use rs13_classic::atmos::StationAtmos;
use rs13_classic::machines::{MACHINE_TICKS, PipeNets};
use rs13_classic::objects::spawn_map;
use rs13_world::{TileMap, TilePos};

/// An air tank piped to an open pipe end in a small room, and the same in open space.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        ' ': (tile: (id: "space")),
        'T': (tile: (id: "floor"), objects: [(id: "air-tank", dir: East)]),
        '-': (tile: (id: "floor"), objects: [(id: "pipe", dir: East)]),
        'V': (tile: (id: "floor"), objects: [(id: "pipe-vent", dir: West)]),
        't': (tile: (id: "space"), objects: [(id: "air-tank", dir: East)]),
        '=': (tile: (id: "space"), objects: [(id: "pipe", dir: East)]),
        'v': (tile: (id: "space"), objects: [(id: "pipe-vent", dir: West)]),
    },
    rows: [
        "WWWWWWW      ",
        "WT-V..W  t=v ",
        "WWWWWWW      ",
    ],
)"#;

const ROOM_VENT: TilePos = TilePos::new(3, 1, 0);
const SPACE_VENT: TilePos = TilePos::new(11, 1, 0);

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

fn pipe_pressure(app: &App, at: TilePos) -> f64 {
    app.world().resource::<PipeNets>().at(at).unwrap().pressure()
}

fn room_pressure(app: &App) -> f64 {
    app.world().resource::<StationAtmos>().0.pressure_at(ROOM_VENT)
}

#[test]
fn an_open_pipe_end_lets_the_network_out_into_the_room() {
    let mut app = setup();
    let (pipe_before, room_before) = (pipe_pressure(&app, ROOM_VENT), room_pressure(&app));
    assert!(pipe_before > 10.0 * room_before, "{pipe_before} vs {room_before}");
    cycles(&mut app, 5);
    let (pipe_after, room_after) = (pipe_pressure(&app, ROOM_VENT), room_pressure(&app));
    assert!(
        room_after > room_before + 10.0,
        "the room fills: {room_before} → {room_after}"
    );
    assert!(
        pipe_after < pipe_before,
        "the network empties: {pipe_before} → {pipe_after}"
    );
}

#[test]
fn in_space_the_gas_is_lost() {
    let mut app = setup();
    let before = pipe_pressure(&app, SPACE_VENT);
    cycles(&mut app, 5);
    let after = pipe_pressure(&app, SPACE_VENT);
    assert!(after < before * 0.99, "{before} → {after}");
}
