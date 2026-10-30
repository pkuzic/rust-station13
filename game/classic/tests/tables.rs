//! Tables and racks: putting things on them and taking them apart.
//! Spec: docs/reference/tables.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::furniture::Table;
use rs13_classic::hands::Hands;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_world::{Appearance, GridPos, ObjectSpec, TileMap, TilePos};

/// A table, a reinforced table and a rack around a bit of floor.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        't': (tile: (id: "floor"), objects: [(id: "table", dir: West, state: Some("tabledir"))]),
        'r': (tile: (id: "floor"), objects: ["table-reinforced"]),
        'k': (tile: (id: "floor"), objects: ["rack"]),
    },
    rows: [
        "WWWWWW",
        "Wtrk.W",
        "W....W",
        "WWWWWW",
    ],
)"#;

const TABLE: TilePos = TilePos::new(1, 2, 0);
const REINFORCED: TilePos = TilePos::new(2, 2, 0);
const RACK: TilePos = TilePos::new(3, 2, 0);
const STAND: TilePos = TilePos::new(2, 1, 0);

/// Taking a table apart takes 5 seconds: 100 ticks at 20 per second.
const WORK_TICKS: usize = 100;

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), STAND);
    tick(&mut app, 1);
    (app, mob)
}

fn tick(app: &mut App, n: usize) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn act(app: &mut App, mob: Entity, action: Action) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
    tick(app, 1);
}

fn give(app: &mut App, mob: Entity, proto: &str) -> Entity {
    let item = spawn_object(app.world_mut(), &ObjectSpec::new(proto), STAND).unwrap();
    act(app, mob, Action::UseOn(item));
    assert_eq!(active(app, mob), Some(item));
    item
}

fn active(app: &App, mob: Entity) -> Option<Entity> {
    app.world().get::<Hands>(mob).unwrap().active_item()
}

fn last_told(app: &App, mob: Entity) -> Option<String> {
    app.world()
        .get::<Outbox>(mob)
        .unwrap()
        .0
        .last()
        .map(|m| m.id.clone())
}

/// The things on `pos`, by sprite.
fn on(app: &mut App, pos: TilePos) -> Vec<String> {
    let mut q = app.world_mut().query::<(&GridPos, &Appearance)>();
    let mut found: Vec<String> = q
        .iter(app.world())
        .filter(|(p, _)| p.0 == pos)
        .map(|(_, a)| a.sprite.clone())
        .collect();
    found.sort();
    found
}

fn find(app: &mut App, sprite: &str) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &Appearance)>();
    q.iter(app.world())
        .find(|(_, a)| a.sprite == sprite)
        .map(|(e, _)| e)
        .unwrap()
}

#[test]
fn tables_keep_their_mapped_look() {
    let (mut app, _) = setup();
    let table = find(&mut app, "table");
    assert_eq!(
        app.world().get::<Appearance>(table).unwrap().state,
        "tabledir-west"
    );
}

#[test]
fn things_go_on_tables_and_racks() {
    let (mut app, mob) = setup();
    let table = find(&mut app, "table");
    let rack = find(&mut app, "rack");
    give(&mut app, mob, "crowbar");
    act(&mut app, mob, Action::UseOn(table));
    assert_eq!(active(&app, mob), None);
    assert_eq!(on(&mut app, TABLE), ["crowbar", "table"]);

    give(&mut app, mob, "screwdriver");
    act(&mut app, mob, Action::UseOn(rack));
    assert_eq!(on(&mut app, RACK), ["rack", "screwdriver"]);
}

#[test]
fn a_wrench_takes_a_table_apart_if_you_stay() {
    let (mut app, mob) = setup();
    let table = find(&mut app, "table");
    give(&mut app, mob, "wrench");
    act(&mut app, mob, Action::UseOn(table));
    assert_eq!(last_told(&app, mob).as_deref(), Some("table-disassembling"));

    // Stepping away spoils it.
    app.world_mut().get_mut::<GridPos>(mob).unwrap().0 = TilePos::new(1, 1, 0);
    tick(&mut app, WORK_TICKS);
    assert_eq!(on(&mut app, TABLE), ["table"]);

    act(&mut app, mob, Action::UseOn(table));
    tick(&mut app, WORK_TICKS - 2);
    assert_eq!(on(&mut app, TABLE), ["table"], "not yet");
    tick(&mut app, 2);
    assert_eq!(on(&mut app, TABLE), ["table-parts"]);

    // The parts and a wrench make a metal sheet.
    let parts = find(&mut app, "table-parts");
    act(&mut app, mob, Action::UseOn(parts));
    assert_eq!(on(&mut app, TABLE), ["metal-sheet"]);
}

#[test]
fn a_reinforced_table_needs_welding_first() {
    let (mut app, mob) = setup();
    let table = find(&mut app, "table-reinforced");
    let wrench = give(&mut app, mob, "wrench");
    // Strong, the wrench is just put down on it.
    act(&mut app, mob, Action::UseOn(table));
    assert_eq!(on(&mut app, REINFORCED), ["table-reinforced", "wrench"]);

    give(&mut app, mob, "welder");
    act(&mut app, mob, Action::UseOn(table));
    assert_eq!(last_told(&app, mob).as_deref(), Some("table-weakening"));
    tick(&mut app, WORK_TICKS);
    assert!(app.world().get::<Table>(table).unwrap().weakened);
    assert_eq!(last_told(&app, mob).as_deref(), Some("table-weakened"));

    act(&mut app, mob, Action::Drop);
    act(&mut app, mob, Action::UseOn(wrench));
    act(&mut app, mob, Action::UseOn(table));
    assert_eq!(
        last_told(&app, mob).as_deref(),
        Some("table-disassembling-reinforced")
    );
    tick(&mut app, WORK_TICKS);
    assert_eq!(on(&mut app, REINFORCED), ["table-parts-reinforced"]);
}

#[test]
fn racks_come_apart_and_go_back_up() {
    let (mut app, mob) = setup();
    let rack = find(&mut app, "rack");
    give(&mut app, mob, "wrench");
    act(&mut app, mob, Action::UseOn(rack));
    assert_eq!(on(&mut app, RACK), ["rack-parts"]);
    act(&mut app, mob, Action::Drop);

    // Used in hand, the parts become a rack where you stand.
    let parts = find(&mut app, "rack-parts");
    act(&mut app, mob, Action::UseOn(parts));
    act(&mut app, mob, Action::UseOn(parts));
    assert_eq!(active(&app, mob), None);
    tick(&mut app, 1);
    assert!(on(&mut app, STAND).contains(&"rack".to_owned()));
}
