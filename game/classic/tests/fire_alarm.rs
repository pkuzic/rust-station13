//! Fire alarms. Spec: docs/reference/fire-alarm.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::atmos::StationAtmos;
use rs13_classic::firealarms::{AreaFires, FireAlarm};
use rs13_classic::firedoors::Firedoor;
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::machines::MACHINE_TICKS;
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_classic::power::Apc;
use rs13_world::{Appearance, GridPos, TileMap, TilePos};

/// Two rooms, "west" (powered, with the fire alarm on its north wall) and "east", with a
/// firedoor on the edge between them.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        'F': (tile: (id: "wall", solid: true), objects: ["fire-alarm"]),
        '.': (tile: (id: "floor")),
        'B': (tile: (id: "floor"), objects: ["smes", (id: "cable", ends: [East])]),
        'Q': (tile: (id: "floor"), objects: ["apc", (id: "cable", ends: [West])]),
        'f': (tile: (id: "floor"), objects: [(id: "firedoor", dir: East)]),
        'c': (tile: (id: "floor"), objects: ["wirecutters"]),
        'r': (tile: (id: "floor"), objects: ["crowbar"]),
    },
    area_legend: { 'a': "west", 'b': "east" },
    area_rows: [
        "  a     ",
        " aaaabb ",
        " aaaabb ",
        "        ",
    ],
    rows: [
        "WWFWWWWW",
        "WBQ.f..W",
        "Wcr.f..W",
        "WWWWWWWW",
    ],
)"#;

const NEXT_TO_ALARM: TilePos = TilePos::new(2, 2, 0);

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), NEXT_TO_ALARM);
    ticks(&mut app, 2 * MACHINE_TICKS);
    (app, mob)
}

fn ticks(app: &mut App, n: u64) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn find(app: &mut App, sprite: &str) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &Appearance)>();
    q.iter(app.world()).find(|(_, a)| a.sprite == sprite).unwrap().0
}

fn click(app: &mut App, mob: Entity, target: Entity) -> Option<String> {
    let before = app.world().get::<Outbox>(mob).unwrap().0.len();
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    // Long enough for the firedoors to finish moving.
    ticks(app, 20);
    let outbox = &app.world().get::<Outbox>(mob).unwrap().0;
    (outbox.len() > before).then(|| outbox[before].id.clone())
}

fn hold(app: &mut App, mob: Entity, sprite: &str) {
    let item = find(app, sprite);
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    app.world_mut().get_mut::<Hands>(mob).unwrap().right = Some(item);
}

fn doors_closed(app: &mut App) -> Vec<bool> {
    let mut q = app.world_mut().query::<&Firedoor>();
    q.iter(app.world()).map(|d| d.closed).collect()
}

fn on_fire(app: &App) -> bool {
    app.world().resource::<AreaFires>().0.contains("west")
}

#[test]
fn pulling_the_alarm_closes_the_firedoors_and_resetting_opens_them() {
    let (mut app, mob) = setup();
    let alarm = find(&mut app, "fire-alarm");
    assert_eq!(app.world().get::<Appearance>(alarm).unwrap().state, "fire0");
    assert_eq!(doors_closed(&mut app), [false, false]);

    assert_eq!(click(&mut app, mob, alarm).as_deref(), Some("fire-alarm-raised"));
    assert!(on_fire(&app));
    assert_eq!(
        doors_closed(&mut app),
        [true, true],
        "the firedoors next to the area close"
    );
    assert_eq!(app.world().get::<Appearance>(alarm).unwrap().state, "fire1");

    assert_eq!(click(&mut app, mob, alarm).as_deref(), Some("fire-alarm-reset"));
    assert!(!on_fire(&app));
    assert_eq!(doors_closed(&mut app), [false, false]);
}

#[test]
fn hitting_it_with_anything_sets_it_off() {
    let (mut app, mob) = setup();
    let alarm = find(&mut app, "fire-alarm");
    hold(&mut app, mob, "crowbar");
    click(&mut app, mob, alarm);
    assert!(on_fire(&app));
    // A tool only ever raises it; the panel resets it.
    click(&mut app, mob, alarm);
    assert!(on_fire(&app));
}

#[test]
fn heat_sets_it_off_unless_its_detector_is_cut() {
    let (mut app, mob) = setup();
    let alarm = find(&mut app, "fire-alarm");
    hold(&mut app, mob, "wirecutters");
    click(&mut app, mob, alarm);
    assert!(!app.world().get::<FireAlarm>(alarm).unwrap().detecting);
    let heat = |app: &mut App| {
        let mut atmos = app.world_mut().resource_mut::<StationAtmos>();
        let mut air = atmos.0.remove_at(NEXT_TO_ALARM, 1e9);
        air.temperature = 600.0;
        atmos.0.add_at(NEXT_TO_ALARM, &air);
    };
    heat(&mut app);
    ticks(&mut app, MACHINE_TICKS);
    assert!(!on_fire(&app), "no detector, no alarm");

    click(&mut app, mob, alarm);
    assert!(app.world().get::<FireAlarm>(alarm).unwrap().detecting);
    heat(&mut app);
    ticks(&mut app, MACHINE_TICKS);
    assert!(on_fire(&app), "above 200 °C");
}

#[test]
fn without_power_it_does_nothing() {
    let (mut app, mob) = setup();
    let alarm = find(&mut app, "fire-alarm");
    let mut apcs = app.world_mut().query::<&mut Apc>();
    for mut apc in apcs.iter_mut(app.world_mut()) {
        apc.operating = false;
    }
    ticks(&mut app, 2 * MACHINE_TICKS);
    assert_eq!(app.world().get::<Appearance>(alarm).unwrap().state, "firep");
    assert_eq!(click(&mut app, mob, alarm), None);
    assert!(!on_fire(&app));
}
