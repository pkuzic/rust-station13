//! Calling the escape pods. Spec: docs/reference/round.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::access::IdCard;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::doors::{Airlock, DoorState};
use rs13_classic::escape::{
    CommsOp, CommsPanelOut, DisplayMode, EscapePods, PendingCommsOps, PodState, StationAlert,
};
use rs13_classic::jobs::HasJob;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_world::{Appearance, DisplayName, LightSource, ObjectSpec, TileMap, TilePos};

/// A pod with a door, and a room with a console, joined up.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'd': (tile: (id: "floor"), objects: ["shuttle-door"]),
        'c': (tile: (id: "floor"), objects: ["comms-console"]),
    },
    rows: [
        "WWWWWWWW",
        "W..d...W",
        "W.....cW",
        "WWWWWWWW",
    ],
    area_legend: { 'p': "shuttle-escape-station-pod1", 's': "bridge" },
    area_rows: [
        "pppppppp",
        "ppppssss",
        "pppsssss",
        "pppsssss",
    ],
)"#;
const NEAR: TilePos = TilePos::new(5, 1, 0);
const IN_POD: TilePos = TilePos::new(1, 2, 0);

fn setup(access: &str) -> (App, Entity, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mut pods = EscapePods::default();
    pods.launch_ds = 1000.0;
    pods.transit_ds = 100.0;
    app.insert_resource(pods);
    let mob = spawn_player_mob(app.world_mut(), NEAR);
    app.world_mut().entity_mut(mob).insert((
        HasJob("captain".to_owned()),
        DisplayName("Left Behind".to_owned()),
    ));
    let card = spawn_object(app.world_mut(), &ObjectSpec::new("id-card"), NEAR).unwrap();
    app.world_mut().entity_mut(card).insert(IdCard {
        access: vec![access.to_owned()],
    });
    app.world_mut().run_schedule(FixedUpdate);
    click(&mut app, mob, card);
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<rs13_classic::escape::CommsConsole>>();
    let console = q.single(app.world()).unwrap();
    let mut q = app.world_mut().query_filtered::<Entity, With<Airlock>>();
    let door = q.single(app.world()).unwrap();
    (app, mob, console, door)
}

fn click(app: &mut App, mob: Entity, target: Entity) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    app.world_mut().run_schedule(FixedUpdate);
}

fn press(app: &mut App, mob: Entity, console: Entity, call: bool) {
    app.world_mut()
        .get_mut::<PendingCommsOps>(mob)
        .unwrap()
        .0
        .push((console, if call { CommsOp::Call } else { CommsOp::Stop }));
    app.world_mut().run_schedule(FixedUpdate);
}

fn ticks(app: &mut App, n: u32) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn said(app: &App, mob: Entity) -> Vec<String> {
    app.world()
        .get::<Outbox>(mob)
        .unwrap()
        .0
        .iter()
        .map(|m| m.id.clone())
        .collect()
}

fn state(app: &App) -> PodState {
    app.world().resource::<EscapePods>().state
}

#[test]
fn only_a_head_can_call_them_and_the_doors_stay_bolted_till_then() {
    let (mut app, mob, console, door) = setup("kitchen");
    assert!(app.world().get::<Airlock>(door).unwrap().bolted);
    press(&mut app, mob, console, true);
    assert_eq!(state(&app), PodState::Docked);
    assert!(said(&app, mob).contains(&"comms-denied".to_owned()));
    assert!(app.world().get::<Airlock>(door).unwrap().bolted);
}

#[test]
fn a_call_unlocks_the_doors_tells_everyone_and_can_be_stopped() {
    let (mut app, mob, console, door) = setup("heads");
    click(&mut app, mob, console);
    ticks(&mut app, 1);
    assert_eq!(
        app.world()
            .get::<CommsPanelOut>(mob)
            .unwrap()
            .0
            .as_ref()
            .unwrap()
            .launch_in,
        None
    );
    press(&mut app, mob, console, true);
    assert!(matches!(state(&app), PodState::Launching { .. }));
    assert!(!app.world().get::<Airlock>(door).unwrap().bolted);
    assert!(said(&app, mob).contains(&"pods-called".to_owned()));
    ticks(&mut app, 1);
    assert!(
        app.world()
            .get::<CommsPanelOut>(mob)
            .unwrap()
            .0
            .as_ref()
            .unwrap()
            .launch_in
            .is_some()
    );

    press(&mut app, mob, console, false);
    assert_eq!(state(&app), PodState::Docked);
    assert!(app.world().get::<Airlock>(door).unwrap().bolted);
    assert!(said(&app, mob).contains(&"pods-recalled".to_owned()));
}

#[test]
fn too_late_to_stop_it_near_the_end() {
    let (mut app, mob, console, _) = setup("heads");
    press(&mut app, mob, console, true);
    ticks(&mut app, 2 * 750);
    press(&mut app, mob, console, false);
    assert!(
        matches!(state(&app), PodState::Launching { .. }),
        "{:?}",
        state(&app)
    );
}

#[test]
fn they_launch_and_arrive_and_the_round_says_who_got_away() {
    let (mut app, mob, console, door) = setup("heads");
    let escapee = spawn_player_mob(app.world_mut(), IN_POD);
    app.world_mut()
        .entity_mut(escapee)
        .insert((HasJob("clown".to_owned()), DisplayName("Got Away".to_owned())));
    press(&mut app, mob, console, true);
    ticks(&mut app, 2 * 1010);
    assert!(
        matches!(state(&app), PodState::Transit { .. }),
        "{:?}",
        state(&app)
    );
    assert!(said(&app, mob).contains(&"pods-launched".to_owned()));
    assert!(
        said(&app, mob).contains(&"pods-seconds".to_owned()),
        "a countdown"
    );
    ticks(&mut app, 2 * 110);
    assert_eq!(state(&app), PodState::Arrived);
    assert!(app.world().get::<Airlock>(door).unwrap().bolted);
    assert_eq!(app.world().get::<Airlock>(door).unwrap().state, DoorState::Closed);

    let told = &app.world().get::<Outbox>(mob).unwrap().0;
    let line = |id: &str| {
        told.iter()
            .find(|m| m.id == id)
            .unwrap_or_else(|| panic!("{id}: {told:?}"))
    };
    assert!(told.iter().any(|m| m.id == "round-over"));
    assert!(format!("{:?}", line("round-escaped")).contains("Got Away"));
    assert!(format!("{:?}", line("round-stranded")).contains("Left Behind"));
}

/// Luna's own pods, on decks 2 and 4 (`LUNA_STATION` runs from deck 4 up), start bolted.
#[test]
fn the_stations_pods_have_doors_that_start_bolted() {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    for z in [0, 2] {
        let map = TileMap::from_ron(rs13_classic::maps::LUNA_STATION[z as usize], z).unwrap();
        spawn_map(app.world_mut(), map);
    }
    app.world_mut().run_schedule(FixedUpdate);
    let mut doors = app.world_mut().query::<(&Airlock, &rs13_world::Appearance)>();
    let bolted: Vec<bool> = doors
        .iter(app.world())
        .filter(|(_, look)| look.sprite == rs13_classic::doors::SHUTTLE_DOOR)
        .map(|(door, _)| door.bolted)
        .collect();
    assert!(bolted.iter().filter(|b| **b).count() >= 2, "{bolted:?}");
}

fn op(app: &mut App, mob: Entity, console: Entity, op: CommsOp) {
    app.world_mut()
        .get_mut::<PendingCommsOps>(mob)
        .unwrap()
        .0
        .push((console, op));
    app.world_mut().run_schedule(FixedUpdate);
}

#[test]
fn red_alert_and_the_displays_are_for_heads() {
    let (mut app, mob, console, _) = setup("kitchen");
    op(&mut app, mob, console, CommsOp::RedAlert);
    assert!(!app.world().resource::<StationAlert>().red);
    op(&mut app, mob, console, CommsOp::Display(DisplayMode::Blank));
    assert_eq!(app.world().resource::<StationAlert>().display, DisplayMode::Pods);
}

#[test]
fn red_alert_turns_the_lights_red_and_back() {
    let (mut app, mob, console, _) = setup("heads");
    let light = spawn_object(app.world_mut(), &ObjectSpec::new("light"), NEAR).unwrap();
    for _ in 0..80 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    let was = app.world().get::<LightSource>(light).expect("lit").0;
    op(&mut app, mob, console, CommsOp::RedAlert);
    ticks(&mut app, 2);
    let red = app.world().get::<LightSource>(light).unwrap().0;
    assert_eq!((red[1], red[2]), (0, 0), "{red:?}");
    assert!(red[0] > 0);
    op(&mut app, mob, console, CommsOp::RedAlert);
    ticks(&mut app, 2);
    assert_eq!(app.world().get::<LightSource>(light).unwrap().0, was);
}

#[test]
fn calling_the_pods_declares_red_alert() {
    let (mut app, mob, console, _) = setup("heads");
    press(&mut app, mob, console, true);
    assert!(app.world().resource::<StationAlert>().red);
}

fn glyphs(app: &mut App) -> Vec<String> {
    let mut q = app
        .world_mut()
        .query::<(&Appearance, &rs13_classic::status_displays::Glyph)>();
    let mut all: Vec<String> = q.iter(app.world()).map(|(a, _)| a.state.clone()).collect();
    all.sort();
    all
}

#[test]
fn displays_show_the_countdown_a_message_or_a_picture() {
    let (mut app, mob, console, _) = setup("heads");
    spawn_object(app.world_mut(), &ObjectSpec::new("status-display"), NEAR).unwrap();
    ticks(&mut app, 40);
    assert!(glyphs(&mut app).is_empty(), "blank till they're called");
    press(&mut app, mob, console, true);
    ticks(&mut app, 4);
    assert!(glyphs(&mut app).contains(&"E".to_owned()), "ETL");
    assert!(glyphs(&mut app).contains(&":".to_owned()), "and a time");

    op(
        &mut app,
        mob,
        console,
        CommsOp::Display(DisplayMode::Message("hi".to_owned(), String::new())),
    );
    ticks(&mut app, 2);
    assert_eq!(glyphs(&mut app), ["H", "I"], "in capitals");
    op(
        &mut app,
        mob,
        console,
        CommsOp::Display(DisplayMode::Picture("biohazard")),
    );
    ticks(&mut app, 2);
    assert_eq!(glyphs(&mut app), ["biohazard"]);
    op(&mut app, mob, console, CommsOp::Display(DisplayMode::Blank));
    ticks(&mut app, 2);
    assert!(glyphs(&mut app).is_empty());
}
