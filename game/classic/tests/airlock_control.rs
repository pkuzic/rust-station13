//! Cycling airlocks. Spec: docs/reference/airlock-control.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::airlock_control::{
    Access, AccessButton, AirlockConsole, Cycle, DoorTag, PRESSURIZED, Program,
};
use rs13_classic::atmos::StationAtmos;
use rs13_classic::doors::{Airlock, DoorState};
use rs13_classic::machines::{MACHINE_TICKS, Switch, Vent};
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_world::{GridPos, TileMap, TilePos};

/// Inside, a door, a chamber with a pump, a door, and space; the console and both buttons sit
/// inside, for the test's reach. Below, an access lock: two doors and a chamber.
const MAP: &str = r#"(
    legend: {
        ' ': (tile: (id: "space")),
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'c': (tile: (id: "floor"), objects: [(id: "airlock-console", vars: {"id_tag": "lock", "exterior_door_tag": "out", "interior_door_tag": "in", "airpump_tag": "pump"})]),
        'i': (tile: (id: "floor"), objects: [(id: "access-button", vars: {"master_tag": "lock", "command": "cycle_interior"})]),
        'e': (tile: (id: "floor"), objects: [(id: "access-button", vars: {"master_tag": "lock", "command": "cycle_exterior"})]),
        'I': (tile: (id: "floor"), objects: [(id: "airlock", vars: {"id_tag": "in"})]),
        'O': (tile: (id: "floor"), objects: [(id: "airlock", vars: {"id_tag": "out"})]),
        'p': (tile: (id: "floor"), objects: [(id: "vent", dir: South, vars: {"id": "pump"})]),
        'q': (tile: (id: "wall", solid: true), objects: [(id: "pipe", dir: North)]),
        'a': (tile: (id: "floor"), objects: [(id: "access-console", vars: {"id_tag": "virology", "exterior_door_tag": "vout", "interior_door_tag": "vin"})]),
        'b': (tile: (id: "floor"), objects: [(id: "access-button", vars: {"master_tag": "virology", "command": "cycle_exterior"})]),
        'x': (tile: (id: "floor"), objects: [(id: "airlock", vars: {"id_tag": "vin"})]),
        'y': (tile: (id: "floor"), objects: [(id: "airlock", vars: {"id_tag": "vout"})]),
    },
    rows: [
        "WWWWWWWW ",
        "Wcie.IpO ",
        "WWWWWWqW ",
        "WWWWWWqW ",
        "WabW.x.yW",
        "WWWWWWWWW",
    ],
)"#;

const CHAMBER: TilePos = TilePos::new(6, 4, 0);

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(4, 4, 0));
    (app, mob)
}

fn cycles(app: &mut App, n: u64) {
    for _ in 0..n * MACHINE_TICKS {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn door(app: &mut App, tag: &str) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &DoorTag)>();
    q.iter(app.world()).find(|(_, t)| t.0 == tag).unwrap().0
}

fn is_open(app: &mut App, tag: &str) -> bool {
    let d = door(app, tag);
    matches!(
        app.world().get::<Airlock>(d).unwrap().state,
        DoorState::Open { .. }
    )
}

fn is_shut(app: &mut App, tag: &str) -> bool {
    let d = door(app, tag);
    app.world().get::<Airlock>(d).unwrap().state == DoorState::Closed
}

fn bolted(app: &mut App, tag: &str) -> bool {
    let d = door(app, tag);
    app.world().get::<Airlock>(d).unwrap().bolted
}

fn press(app: &mut App, mob: Entity, command: &str, master: &str) {
    let mut q = app.world_mut().query::<(Entity, &AccessButton)>();
    let button = q
        .iter(app.world())
        .find(|(_, b)| b.command == command && b.master == master)
        .unwrap()
        .0;
    let at = app.world().get::<GridPos>(button).unwrap().0;
    app.world_mut().get_mut::<GridPos>(mob).unwrap().0 = at;
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(button));
    app.world_mut().run_schedule(FixedUpdate);
}

fn pumped_state(app: &mut App) -> Cycle {
    let mut q = app.world_mut().query::<&AirlockConsole>();
    q.iter(app.world())
        .find_map(|c| match c.program {
            Program::Pumped { state, .. } => Some(state),
            _ => None,
        })
        .unwrap()
}

fn access_state(app: &mut App) -> Access {
    let mut q = app.world_mut().query::<&AirlockConsole>();
    q.iter(app.world())
        .find_map(|c| match c.program {
            Program::Access { state, .. } => Some(state),
            _ => None,
        })
        .unwrap()
}

fn pressure(app: &App, at: TilePos) -> f64 {
    app.world().resource::<StationAtmos>().0.pressure_at(at)
}

#[test]
fn a_pumped_airlock_starts_open_inside_and_bolted_shut_outside() {
    let (mut app, _) = setup();
    cycles(&mut app, 8);
    assert_eq!(pumped_state(&mut app), Cycle::InOpen);
    assert!(is_open(&mut app, "in"));
    assert!(is_shut(&mut app, "out"));
    assert!(bolted(&mut app, "out"), "nobody opens it by hand");
    assert!(bolted(&mut app, "in"));
}

#[test]
fn cycling_out_shuts_the_inside_pumps_the_air_out_then_opens_the_outside() {
    let (mut app, mob) = setup();
    cycles(&mut app, 8);
    let before = pressure(&app, CHAMBER);
    assert!(before >= PRESSURIZED, "{before}");

    press(&mut app, mob, "cycle_exterior", "lock");
    let mut saw_both_shut = false;
    for _ in 0..60 {
        cycles(&mut app, 1);
        assert!(
            !(is_open(&mut app, "in") && is_open(&mut app, "out")),
            "never both open"
        );
        saw_both_shut |= is_shut(&mut app, "in") && is_shut(&mut app, "out");
        if pumped_state(&mut app) == Cycle::OutOpen {
            break;
        }
    }
    assert!(saw_both_shut);
    assert_eq!(pumped_state(&mut app), Cycle::OutOpen);
    assert!(
        pressure(&app, CHAMBER) < 10.0,
        "pumped out: {}",
        pressure(&app, CHAMBER)
    );
    assert!(
        pressure(&app, TilePos::new(4, 4, 0)) > PRESSURIZED,
        "the inside kept its air"
    );
    cycles(&mut app, 2);
    assert!(is_open(&mut app, "out"));

    let mut q = app.world_mut().query_filtered::<&Switch, With<Vent>>();
    assert!(q.iter(app.world()).all(|s| !s.0), "the pump rests");
}

#[test]
fn cycling_back_in_refills_the_chamber_from_what_was_pumped_out() {
    let (mut app, mob) = setup();
    cycles(&mut app, 8);
    press(&mut app, mob, "cycle_exterior", "lock");
    for _ in 0..60 {
        cycles(&mut app, 1);
        if pumped_state(&mut app) == Cycle::OutOpen {
            break;
        }
    }
    assert_eq!(pumped_state(&mut app), Cycle::OutOpen);
    // What was pumped into the pipes is all the air there is to come back.
    press(&mut app, mob, "cycle_interior", "lock");
    let mut reached = false;
    for _ in 0..60 {
        cycles(&mut app, 1);
        assert!(
            !(is_open(&mut app, "in") && is_open(&mut app, "out")),
            "never both open"
        );
        if pumped_state(&mut app) == Cycle::InOpen {
            reached = true;
            break;
        }
    }
    assert!(
        reached,
        "{:?} at {} kPa",
        pumped_state(&mut app),
        pressure(&app, CHAMBER)
    );
    assert!(bolted(&mut app, "out"));
    assert!(pressure(&app, CHAMBER) >= PRESSURIZED);
}

#[test]
fn an_access_lock_only_unlocks_one_door_at_a_time() {
    let (mut app, mob) = setup();
    cycles(&mut app, 8);
    assert_eq!(access_state(&mut app), Access::Locked);
    assert!(bolted(&mut app, "vin") && bolted(&mut app, "vout"));

    press(&mut app, mob, "cycle_exterior", "virology");
    cycles(&mut app, 3);
    assert_eq!(access_state(&mut app), Access::External);
    assert!(bolted(&mut app, "vin"));
    assert!(!bolted(&mut app, "vout"));
}
