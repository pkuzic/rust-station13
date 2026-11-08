//! Teleporters. Spec: docs/reference/teleporters.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::filing::{PendingPicks, PickListOut};
use rs13_classic::hands::Hands;
use rs13_classic::movement::MoveIntent;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::teleporters::{self, Portal, TeleConsole, TeleHub};
use rs13_world::{Dir, GridPos, HumanLook, ObjectSpec, TileMap, TilePos};

/// Console, station and hub in a row, a beacon far off in its own area.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'c': (tile: (id: "floor"), objects: ["teleporter-console"]),
        's': (tile: (id: "floor"), objects: ["teleporter-station"]),
        'h': (tile: (id: "floor"), objects: ["teleporter-hub"]),
        'b': (tile: (id: "floor"), objects: ["tracking-beacon"]),
    },
    area_legend: { 't': "teleporter", 'f': "far" },
    area_rows: [
        "tttttttffffffff",
        "tttttttffffffff",
        "tttttttffffffff",
        "tttttttffffffff",
        "tttttttffffffff",
        "tttttttffffffff",
    ],
    rows: [
        "WWWWWWWWWWWWWWW",
        "W.....W.......W",
        "Wcsh..W.......W",
        "W.....W....b..W",
        "W.....W.......W",
        "WWWWWWWWWWWWWWW",
    ],
)"#;

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    tick(&mut app, 1);
    app
}

fn tick(app: &mut App, n: u32) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn at(x: i32, y: i32) -> TilePos {
    TilePos::new(x, y, 0)
}

fn person(app: &mut App, x: i32, y: i32) -> Entity {
    let mob = spawn_player_mob(app.world_mut(), at(x, y));
    app.world_mut().entity_mut(mob).insert(HumanLook::default());
    mob
}

fn act(app: &mut App, mob: Entity, action: Action) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
    tick(app, 1);
}

fn pick(app: &mut App, mob: Entity, target: Entity, line: usize) {
    app.world_mut()
        .get_mut::<PendingPicks>(mob)
        .unwrap()
        .0
        .push((target, line));
    tick(app, 1);
}

fn find<C: Component>(app: &mut App) -> Entity {
    let world = app.world_mut();
    let mut q = world.query_filtered::<Entity, With<C>>();
    q.iter(world).next().unwrap()
}

fn rows(app: &App, mob: Entity) -> Vec<String> {
    app.world()
        .get::<PickListOut>(mob)
        .and_then(|o| o.0.as_ref())
        .map(|v| v.rows.clone())
        .unwrap_or_default()
}

fn told(app: &App, mob: Entity, id: &str) -> bool {
    app.world()
        .get::<Outbox>(mob)
        .unwrap()
        .0
        .iter()
        .any(|m| m.id == id)
}

fn pos(app: &App, e: Entity) -> TilePos {
    app.world().get::<GridPos>(e).unwrap().0
}

/// Walk one step a way, then stop.
fn walk(app: &mut App, mob: Entity, dir: Dir) {
    app.world_mut().get_mut::<MoveIntent>(mob).unwrap().dir = Some(dir);
    tick(app, 1);
    app.world_mut().get_mut::<MoveIntent>(mob).unwrap().dir = None;
    tick(app, 6);
}

#[test]
fn lock_on_engage_and_walk_into_the_hub() {
    let mut app = setup();
    let console = find::<TeleConsole>(&mut app);
    let hub = find::<TeleHub>(&mut app);
    let station = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, With<teleporters::TeleStation>>();
        q.iter(world).next().unwrap()
    };
    let id = app.world().get::<TeleConsole>(console).unwrap().id.clone();
    assert!((1000..=9999).contains(&id.parse::<i32>().unwrap()));
    let tech = person(&mut app, 2, 2);
    let watcher = person(&mut app, 4, 1);

    // Walking into the hub before it's on does nothing.
    let traveller = person(&mut app, 4, 3);
    walk(&mut app, traveller, Dir::West);
    assert_eq!(pos(&app, traveller), at(4, 3));

    act(&mut app, tech, Action::UseOn(console));
    tick(&mut app, 1);
    assert_eq!(rows(&app, tech), ["far"]);
    pick(&mut app, tech, console, 0);
    assert!(app.world().get::<TeleConsole>(console).unwrap().locked.is_some());
    tick(&mut app, 1);
    assert!(told(&app, watcher, "tele-locked-in"));

    act(&mut app, tech, Action::UseOn(station));
    tick(&mut app, 1);
    assert!(told(&app, watcher, "tele-engaged"));
    assert!(app.world().get::<TeleHub>(hub).unwrap().on);

    // In it goes, to the beacon (dead on, unless it goes wrong 5% of the time).
    walk(&mut app, traveller, Dir::West);
    let now = pos(&app, traveller);
    assert_ne!(now, at(4, 3), "thrown somewhere");
    // Off again.
    act(&mut app, tech, Action::UseOn(station));
    tick(&mut app, 1);
    assert!(!app.world().get::<TeleHub>(hub).unwrap().on);
    assert!(told(&app, watcher, "tele-disengaged"));
}

#[test]
fn a_hub_with_no_lock_says_so() {
    let mut app = setup();
    let station = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, With<teleporters::TeleStation>>();
        q.iter(world).next().unwrap()
    };
    let tech = person(&mut app, 2, 2);
    act(&mut app, tech, Action::UseOn(station));
    let traveller = person(&mut app, 4, 3);
    walk(&mut app, traveller, Dir::West);
    assert_eq!(pos(&app, traveller), at(4, 3));
    assert!(told(&app, traveller, "tele-no-lock"));
}

#[test]
fn the_hand_teleporter_opens_up_to_four_portals_to_a_locked_beacon() {
    let mut app = setup();
    let console = find::<TeleConsole>(&mut app);
    let tech = person(&mut app, 2, 2);
    act(&mut app, tech, Action::UseOn(console));
    pick(&mut app, tech, console, 0);
    let wizard = person(&mut app, 4, 4);
    let tele = spawn_object(app.world_mut(), &ObjectSpec::new("hand-tele"), at(4, 4)).unwrap();
    act(&mut app, wizard, Action::UseOn(tele));
    assert_eq!(
        app.world().get::<Hands>(wizard).unwrap().active_item(),
        Some(tele)
    );
    act(&mut app, wizard, Action::UseOn(tele));
    tick(&mut app, 1);
    let shown = rows(&app, wizard);
    let id = app.world().get::<TeleConsole>(console).unwrap().id.clone();
    assert_eq!(
        shown,
        [
            format!("msg:tele-row-inactive:{id}"),
            "msg:tele-row-none".to_owned()
        ]
    );
    for _ in 0..4 {
        act(&mut app, wizard, Action::UseOn(tele));
        pick(&mut app, wizard, tele, 0);
    }
    let portals = |app: &mut App| {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &Portal)>();
        q.iter(world).map(|(e, _)| e).collect::<Vec<_>>()
    };
    assert_eq!(portals(&mut app).len(), 4);
    act(&mut app, wizard, Action::UseOn(tele));
    pick(&mut app, wizard, tele, 0);
    assert!(told(&app, wizard, "tele-recharging"));
    assert_eq!(portals(&mut app).len(), 4);

    // Walking into one (it's dense) throws you beside the beacon, most of the time.
    let traveller = person(&mut app, 5, 4);
    walk(&mut app, traveller, Dir::West);
    assert_ne!(pos(&app, traveller), at(5, 4));
    // They close after 30 s.
    tick(
        &mut app,
        (teleporters::PORTAL_DS * rs13_core::TICK_RATE_HZ as f32 / 10.0) as u32 + 2,
    );
    assert!(portals(&mut app).is_empty());
}

#[test]
fn on_lunas_deck_the_console_station_and_hub_stand_in_a_row() {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let map = TileMap::from_ron(include_str!("../maps/luna_deck3.ron"), 3).unwrap();
    let _ = spawn_map(app.world_mut(), map);
    let console = find::<TeleConsole>(&mut app);
    let hub = find::<TeleHub>(&mut app);
    let (c, h) = (pos(&app, console), pos(&app, hub));
    assert_eq!((h.x - c.x, h.y - c.y), (2, 0), "{c:?} {h:?}");
}
