//! Object verbs from the right-click menu. Spec: docs/reference/hud.md (Right-click menu).

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::chairs::Chair;
use rs13_classic::closets::{Closet, ClosetLock as Lock};
use rs13_classic::flashlights::Flashlight;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::verbs::{ROTATE_CHAIR, TOGGLE_LIGHT, TOGGLE_LOCK, TOGGLE_OPEN};
use rs13_world::{Dir, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'c': (tile: (id: "floor"), objects: ["closet"]),
        'l': (tile: (id: "floor"), objects: ["locker-engineer"]),
        'h': (tile: (id: "floor"), objects: [(id: "chair", dir: South)]),
    },
    rows: [
        "WWWWWWW",
        "W.c.l.W",
        "W..h..W",
        "WWWWWWW",
    ],
)"#;

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().run_schedule(FixedUpdate);
    app
}

fn verb(app: &mut App, mob: Entity, target: Entity, verb: &'static str) -> Vec<String> {
    let before = app.world().get::<Outbox>(mob).unwrap().0.len();
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::Verb(target, verb));
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(FixedUpdate);
    app.world().get::<Outbox>(mob).unwrap().0[before..]
        .iter()
        .map(|m| m.id.clone())
        .collect()
}

fn find<C: Component>(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, (With<C>, Without<Lock>)>()
        .iter(app.world())
        .next()
        .unwrap()
}

#[test]
fn toggle_open_works_a_closet_even_with_something_in_hand() {
    let mut app = setup();
    let closet = find::<Closet>(&mut app);
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(2, 1, 0));
    let wrench = spawn_object(app.world_mut(), &ObjectSpec::new("wrench"), TilePos::new(2, 1, 0)).unwrap();
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(wrench));
    app.world_mut().run_schedule(FixedUpdate);
    verb(&mut app, mob, closet, TOGGLE_OPEN);
    assert!(app.world().get::<Closet>(closet).unwrap().opened);
    verb(&mut app, mob, closet, TOGGLE_OPEN);
    assert!(
        !app.world().get::<Closet>(closet).unwrap().opened,
        "and the wrench wasn't put in it"
    );
}

#[test]
fn toggle_lock_wants_the_access_and_rotate_turns_a_chair() {
    let mut app = setup();
    let locker = app
        .world_mut()
        .query_filtered::<Entity, With<Lock>>()
        .single(app.world())
        .unwrap();
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(4, 1, 0));
    assert_eq!(verb(&mut app, mob, locker, TOGGLE_LOCK), ["locker-access-denied"]);

    let chair = find::<Chair>(&mut app);
    assert_eq!(app.world().get::<Chair>(chair).unwrap().dir, Dir::South);
    verb(&mut app, mob, chair, ROTATE_CHAIR);
    assert_eq!(app.world().get::<Chair>(chair).unwrap().dir, Dir::East);
}

#[test]
fn toggle_light_needs_the_light_in_hand() {
    let mut app = setup();
    let at = TilePos::new(1, 2, 0);
    let mob = spawn_player_mob(app.world_mut(), at);
    let light = spawn_object(app.world_mut(), &ObjectSpec::new("flashlight"), at).unwrap();
    verb(&mut app, mob, light, TOGGLE_LIGHT);
    assert!(!app.world().get::<Flashlight>(light).unwrap().on, "not carried");
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(light));
    app.world_mut().run_schedule(FixedUpdate);
    verb(&mut app, mob, light, TOGGLE_LIGHT);
    assert!(app.world().get::<Flashlight>(light).unwrap().on);
}
