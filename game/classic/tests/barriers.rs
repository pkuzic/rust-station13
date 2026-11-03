//! Deployable barriers. Spec: docs/reference/security.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::Entity;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::pulling::{PendingPulls, Pulling};
use rs13_classic::security::Barrier;
use rs13_world::{Appearance, Dense, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: { 'W': (tile: (id: "wall", solid: true)), '.': (tile: (id: "floor")) },
    rows: ["WWWWW", "W...W", "WWWWW"],
)"#;
const HERE: TilePos = TilePos::new(1, 1, 0);

#[test]
fn any_card_locks_and_unlocks_one_and_nothing_else_does() {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), HERE);
    let barrier = spawn_object(
        app.world_mut(),
        &ObjectSpec::new("barrier"),
        TilePos::new(2, 1, 0),
    )
    .unwrap();
    let card = spawn_object(app.world_mut(), &ObjectSpec::new("id-card"), HERE).unwrap();
    app.world_mut().run_schedule(FixedUpdate);
    let click = |app: &mut App, target| {
        app.world_mut()
            .get_mut::<PendingActions>(mob)
            .unwrap()
            .0
            .push(Action::UseOn(target));
        app.world_mut().run_schedule(FixedUpdate);
    };
    click(&mut app, barrier);
    assert!(
        !app.world().get::<Barrier>(barrier).unwrap().locked,
        "an empty hand does nothing"
    );

    click(&mut app, card);
    click(&mut app, barrier);
    assert!(app.world().get::<Barrier>(barrier).unwrap().locked);
    assert_eq!(app.world().get::<Appearance>(barrier).unwrap().state, "barrier1");
    assert_eq!(
        app.world().get::<Outbox>(mob).unwrap().0.last().unwrap().id,
        "barrier-locked"
    );
    assert!(app.world().get::<Dense>(barrier).is_some());

    click(&mut app, barrier);
    assert!(!app.world().get::<Barrier>(barrier).unwrap().locked);
    assert_eq!(app.world().get::<Appearance>(barrier).unwrap().state, "barrier0");
}

fn setup_with(proto: &str) -> (App, Entity, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), HERE);
    let barrier = spawn_object(
        app.world_mut(),
        &ObjectSpec::new("barrier"),
        TilePos::new(2, 1, 0),
    )
    .unwrap();
    let tool = spawn_object(app.world_mut(), &ObjectSpec::new(proto), HERE).unwrap();
    app.world_mut().run_schedule(FixedUpdate);
    click(&mut app, mob, tool);
    (app, mob, barrier, tool)
}

fn click(app: &mut App, mob: Entity, target: Entity) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    app.world_mut().run_schedule(FixedUpdate);
}

#[test]
fn hitting_one_wears_it_down_until_it_falls_apart() {
    let (mut app, mob, barrier, _) = setup_with("crowbar");
    click(&mut app, mob, barrier);
    assert_eq!(
        app.world().get::<Barrier>(barrier).unwrap().health,
        100.0 - 5.0 * 0.75
    );
    assert_eq!(
        app.world().get::<Outbox>(mob).unwrap().0.last().unwrap().id,
        "barrier-hit"
    );
    app.world_mut().get_mut::<Barrier>(barrier).unwrap().health = 1.0;
    click(&mut app, mob, barrier);
    assert!(app.world().get_entity(barrier).is_err());
    assert_eq!(
        app.world().get::<Outbox>(mob).unwrap().0.last().unwrap().id,
        "barrier-broken"
    );
}

#[test]
fn a_lit_welder_repairs_one() {
    let (mut app, mob, barrier, welder) = setup_with("welder");
    app.world_mut()
        .get_mut::<rs13_classic::welding::Welder>(welder)
        .unwrap()
        .lit = true;
    app.world_mut().get_mut::<Barrier>(barrier).unwrap().health = 10.0;
    click(&mut app, mob, barrier);
    assert_eq!(app.world().get::<Barrier>(barrier).unwrap().health, 100.0);
    assert_eq!(
        app.world().get::<Outbox>(mob).unwrap().0.last().unwrap().id,
        "barrier-repaired"
    );
}

#[test]
fn an_unlocked_one_can_be_pulled_and_locking_it_lets_go() {
    let (mut app, mob, barrier, _) = setup_with("id-card");
    app.world_mut()
        .resource_mut::<PendingPulls>()
        .0
        .push((mob, barrier));
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(app.world().get::<Pulling>(mob).unwrap().0, barrier);

    click(&mut app, mob, barrier);
    assert!(app.world().get::<Barrier>(barrier).unwrap().locked);
    app.world_mut().run_schedule(FixedUpdate);
    assert!(app.world().get::<Pulling>(mob).is_none());
    app.world_mut()
        .resource_mut::<PendingPulls>()
        .0
        .push((mob, barrier));
    app.world_mut().run_schedule(FixedUpdate);
    assert!(app.world().get::<Pulling>(mob).is_none(), "locked: it stays");
}
