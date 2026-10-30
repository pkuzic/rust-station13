//! Pipe pressure meters. Spec: docs/reference/meters.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::meters::Meter;
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_classic::power::PowerUser;
use rs13_world::{Appearance, TileMap, TilePos};

/// A powered room with an air tank feeding a pipe that has a meter on it, and a meter
/// with no pipe under it.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "plating")),
        'B': (tile: (id: "plating"), objects: ["smes", (id: "cable", ends: [East])]),
        'Q': (tile: (id: "plating"), objects: ["apc", (id: "cable", ends: [West])]),
        'T': (tile: (id: "plating"), objects: [(id: "air-tank", dir: East)]),
        'm': (tile: (id: "plating"), objects: [(id: "pipe", dir: East), "meter"]),
        'x': (tile: (id: "plating"), objects: ["meter"]),
    },
    area_legend: { 'r': "room" },
    area_rows: [
        "         ",
        " rrrrrrr ",
        " rrrrrrr ",
        " rrrrrrr ",
        "         ",
    ],
    rows: [
        "WWWWWWWWW",
        "WBQ.....W",
        "WTm....xW",
        "W.......W",
        "WWWWWWWWW",
    ],
)"#;

const PIPE_METER: TilePos = TilePos::new(2, 2, 0);
const LONE_METER: TilePos = TilePos::new(7, 2, 0);

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(3, 1, 0));
    for _ in 0..5 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    (app, mob)
}

fn meter_at(app: &mut App, pos: TilePos) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &rs13_world::GridPos, &Meter)>();
    q.iter(app.world()).find(|(_, p, _)| p.0 == pos).unwrap().0
}

fn click(app: &mut App, mob: Entity, target: Entity) -> Option<rs13_i18n::LocMsg> {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    app.world_mut().run_schedule(FixedUpdate);
    app.world().get::<Outbox>(mob).unwrap().0.last().cloned()
}

#[test]
fn a_meter_shows_and_reads_its_pipe() {
    let (mut app, mob) = setup();
    let meter = meter_at(&mut app, PIPE_METER);
    assert!(app.world().get::<PowerUser>(meter).unwrap().powered);
    // The tank's air is well above 59 atm: the needle is off the scale.
    assert_eq!(app.world().get::<Appearance>(meter).unwrap().state, "meter4");

    let said = click(&mut app, mob, meter).unwrap();
    assert_eq!(said.id, "meter-reading");

    // Too far away to read.
    app.world_mut().get_mut::<rs13_world::GridPos>(mob).unwrap().0 = TilePos::new(6, 1, 0);
    assert_eq!(click(&mut app, mob, meter).unwrap().id, "meter-too-far");
}

#[test]
fn a_meter_without_a_pipe_blinks() {
    let (mut app, mob) = setup();
    let meter = meter_at(&mut app, LONE_METER);
    assert_eq!(app.world().get::<Appearance>(meter).unwrap().state, "meterX");
    app.world_mut().get_mut::<rs13_world::GridPos>(mob).unwrap().0 = TilePos::new(6, 1, 0);
    assert_eq!(click(&mut app, mob, meter).unwrap().id, "meter-no-pipe");
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::Examine(meter));
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(
        app.world().get::<Outbox>(mob).unwrap().0.last().unwrap().id,
        "examine-meter-no-pipe"
    );
}

#[test]
fn an_unpowered_meter_reads_zero_and_says_nothing() {
    let (mut app, mob) = setup();
    let meter = meter_at(&mut app, PIPE_METER);
    let mut apcs = app.world_mut().query::<&mut rs13_classic::power::Apc>();
    for mut apc in apcs.iter_mut(app.world_mut()) {
        apc.operating = false;
    }
    // The next power cycle cuts it off.
    for _ in 0..2 * rs13_classic::machines::MACHINE_TICKS {
        app.world_mut().run_schedule(FixedUpdate);
    }
    assert_eq!(app.world().get::<Appearance>(meter).unwrap().state, "meter0");
    let before = app.world().get::<Outbox>(mob).unwrap().0.len();
    click(&mut app, mob, meter);
    assert_eq!(app.world().get::<Outbox>(mob).unwrap().0.len(), before);
}
