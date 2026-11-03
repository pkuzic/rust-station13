//! Brig door timers. Spec: docs/reference/brig.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::brig::{BrigOp, BrigPanelOut, DoorTimer, MAX_SECONDS, PendingBrigOps};
use rs13_classic::flashes::Weakened;
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::machines::MACHINE_TICKS;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::windoors::{Swing, Windoor};
use rs13_world::{GridPos, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'd': (tile: (id: "floor"), objects: [(id: "brig-windoor", dir: East, vars: {"id": "Cell 1"})]),
        't': (tile: (id: "floor"), objects: [(id: "door-timer", vars: {"id": "Cell 1"})]),
        'f': (tile: (id: "floor"), objects: [(id: "flasher", vars: {"id": "Cell 1"})]),
    },
    rows: [
        "WWWWWWWW",
        "W.f.d.tW",
        "W......W",
        "WWWWWWWW",
    ],
)"#;

const AT_TIMER: TilePos = TilePos::new(5, 2, 0);
const IN_CELL: TilePos = TilePos::new(2, 2, 0);

fn setup() -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let officer = spawn_player_mob(app.world_mut(), AT_TIMER);
    let mut q = app.world_mut().query_filtered::<Entity, With<DoorTimer>>();
    let timer = q.single(app.world()).unwrap();
    app.world_mut().run_schedule(FixedUpdate);
    (app, officer, timer)
}

fn cycles(app: &mut App, n: u64) {
    for _ in 0..n * MACHINE_TICKS {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn give_card(app: &mut App, mob: Entity) {
    let spec = ObjectSpec {
        access: vec!["brig".to_owned()],
        ..ObjectSpec::new("id-card")
    };
    let card = spawn_object(app.world_mut(), &spec, AT_TIMER).unwrap();
    app.world_mut()
        .entity_mut(card)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    let mut hands = app.world_mut().get_mut::<Hands>(mob).unwrap();
    let active = hands.active;
    *hands.slot_mut(active) = Some(card);
}

fn press(app: &mut App, mob: Entity, timer: Entity, op: BrigOp) {
    app.world_mut()
        .get_mut::<PendingBrigOps>(mob)
        .unwrap()
        .0
        .push((timer, op));
    app.world_mut().run_schedule(FixedUpdate);
}

fn door_open(app: &mut App) -> bool {
    let mut q = app.world_mut().query::<&Windoor>();
    matches!(q.single(app.world()).unwrap().swing, Swing::Open { .. })
}

fn door_shut(app: &mut App) -> bool {
    let mut q = app.world_mut().query::<&Windoor>();
    q.single(app.world()).unwrap().swing == Swing::Closed
}

#[test]
fn a_sentence_shuts_the_door_and_serving_it_opens_it() {
    let (mut app, officer, timer) = setup();
    give_card(&mut app, officer);
    cycles(&mut app, 2);
    assert!(door_open(&mut app), "no sentence: the cell stands open");

    press(&mut app, officer, timer, BrigOp::Adjust(60));
    press(&mut app, officer, timer, BrigOp::Adjust(-40));
    assert_eq!(app.world().get::<DoorTimer>(timer).unwrap().length, 20);
    press(&mut app, officer, timer, BrigOp::Toggle);
    cycles(&mut app, 2);
    assert!(door_shut(&mut app));
    let time = app
        .world()
        .resource::<rs13_classic::time::WorldTime>()
        .deciseconds();
    let left = app.world().get::<DoorTimer>(timer).unwrap().seconds_left(time);
    assert!(left > 10 && left <= 20, "{left}");

    cycles(&mut app, 10);
    assert_eq!(app.world().get::<DoorTimer>(timer).unwrap().release_at, None);
    assert_eq!(app.world().get::<DoorTimer>(timer).unwrap().length, 0);
    cycles(&mut app, 2);
    assert!(door_open(&mut app));
}

#[test]
fn stopping_early_opens_the_door_and_keeps_whats_left() {
    let (mut app, officer, timer) = setup();
    give_card(&mut app, officer);
    press(&mut app, officer, timer, BrigOp::Adjust(100));
    press(&mut app, officer, timer, BrigOp::Toggle);
    cycles(&mut app, 2);
    assert!(door_shut(&mut app));
    press(&mut app, officer, timer, BrigOp::Toggle);
    cycles(&mut app, 2);
    assert!(door_open(&mut app));
    let kept = app.world().get::<DoorTimer>(timer).unwrap().length;
    assert!(kept > 90 && kept < 100, "{kept}");
}

#[test]
fn sentences_stay_within_limits() {
    let (mut app, officer, timer) = setup();
    give_card(&mut app, officer);
    press(&mut app, officer, timer, BrigOp::Adjust(-5));
    assert_eq!(app.world().get::<DoorTimer>(timer).unwrap().length, 0);
    press(&mut app, officer, timer, BrigOp::Adjust(MAX_SECONDS + 60));
    assert_eq!(app.world().get::<DoorTimer>(timer).unwrap().length, MAX_SECONDS);
}

#[test]
fn without_brig_access_it_refuses() {
    let (mut app, officer, timer) = setup();
    press(&mut app, officer, timer, BrigOp::Adjust(60));
    assert_eq!(app.world().get::<DoorTimer>(timer).unwrap().length, 0);
    let last = app
        .world()
        .get::<Outbox>(officer)
        .unwrap()
        .0
        .last()
        .unwrap()
        .id
        .clone();
    assert_eq!(last, "brig-denied");
}

#[test]
fn the_panel_flashes_the_cell() {
    let (mut app, officer, timer) = setup();
    give_card(&mut app, officer);
    let prisoner = spawn_player_mob(app.world_mut(), IN_CELL);
    app.world_mut()
        .get_mut::<PendingActions>(officer)
        .unwrap()
        .0
        .push(Action::UseOn(timer));
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(FixedUpdate);
    let view = app
        .world()
        .get::<BrigPanelOut>(officer)
        .unwrap()
        .0
        .clone()
        .expect("open");
    assert!(view.has_flash);
    assert!(!view.flash_charging);

    press(&mut app, officer, timer, BrigOp::Flash);
    assert!(app.world().get::<Weakened>(prisoner).is_some());
    app.world_mut().run_schedule(FixedUpdate);
    let view = app
        .world()
        .get::<BrigPanelOut>(officer)
        .unwrap()
        .0
        .clone()
        .expect("open");
    assert!(view.flash_charging);
}
