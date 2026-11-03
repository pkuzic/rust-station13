//! The autolathe. Spec: docs/reference/autolathe.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::autolathe::{Autolathe, LathePanelOut, MAX_GLASS, MAX_METAL, PendingLatheOps, RECIPES};
use rs13_classic::hands::{Hands, Stack};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_world::{Appearance, GridPos, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'a': (tile: (id: "floor"), objects: ["autolathe"]),
    },
    rows: ["WWWWW", "Wa..W", "WWWWW"],
)"#;
const NEAR: TilePos = TilePos::new(2, 1, 0);

fn setup() -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), NEAR);
    app.world_mut().run_schedule(FixedUpdate);
    let mut q = app.world_mut().query_filtered::<Entity, With<Autolathe>>();
    let lathe = q.single(app.world()).unwrap();
    (app, mob, lathe)
}

fn click(app: &mut App, mob: Entity, target: Entity) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    app.world_mut().run_schedule(FixedUpdate);
}

fn feed(app: &mut App, mob: Entity, lathe: Entity, proto: &str) {
    let sheet = rs13_classic::objects::spawn_stack(app.world_mut(), proto, NEAR, 1).unwrap();
    app.world_mut().run_schedule(FixedUpdate);
    click(app, mob, sheet);
    click(app, mob, lathe);
}

fn line_of(proto: &str) -> usize {
    RECIPES.iter().position(|(id, ..)| *id == proto).unwrap()
}

fn press(app: &mut App, mob: Entity, lathe: Entity, line: usize) {
    app.world_mut()
        .get_mut::<PendingLatheOps>(mob)
        .unwrap()
        .0
        .push((lathe, line));
    app.world_mut().run_schedule(FixedUpdate);
}

fn count(app: &mut App, sprite: &str) -> usize {
    let mut q = app.world_mut().query::<(&Appearance, &GridPos)>();
    q.iter(app.world()).filter(|(a, _)| a.sprite == sprite).count()
}

#[test]
fn every_recipe_can_be_made() {
    let (mut app, ..) = setup();
    for (proto, ..) in RECIPES {
        spawn_object(app.world_mut(), &ObjectSpec::new(proto), NEAR).expect(proto);
    }
}

#[test]
fn sheets_go_in_and_are_used_up() {
    let (mut app, mob, lathe) = setup();
    feed(&mut app, mob, lathe, "metal-sheet");
    let state = app.world().get::<Autolathe>(lathe).unwrap();
    assert_eq!(state.metal, 3750);
    assert!(app.world().get::<Hands>(mob).unwrap().active_item().is_none());
    assert_eq!(count(&mut app, "metal-sheet"), 0);
}

#[test]
fn a_stack_goes_in_as_far_as_it_fits() {
    let (mut app, mob, lathe) = setup();
    let sheets = spawn_object(app.world_mut(), &ObjectSpec::new("glass-sheet"), NEAR).unwrap();
    app.world_mut().run_schedule(FixedUpdate);
    click(&mut app, mob, sheets);
    click(&mut app, mob, lathe);
    assert_eq!(
        app.world().get::<Autolathe>(lathe).unwrap().glass,
        MAX_GLASS,
        "as many as fit"
    );
    assert_eq!(
        app.world().get::<Stack>(sheets).unwrap().amount,
        30,
        "the rest stay in hand"
    );
}

#[test]
fn it_takes_the_cost_and_drops_the_thing_after_a_moment() {
    let (mut app, mob, lathe) = setup();
    feed(&mut app, mob, lathe, "metal-sheet");
    click(&mut app, mob, lathe);
    app.world_mut().run_schedule(FixedUpdate);
    let view = app
        .world()
        .get::<LathePanelOut>(mob)
        .unwrap()
        .0
        .clone()
        .expect("open");
    assert!(view.rows[line_of("crowbar")].can_make);
    assert!(!view.rows[line_of("beaker")].can_make, "no glass");

    let before = count(&mut app, "crowbar");
    press(&mut app, mob, lathe, line_of("crowbar"));
    assert_eq!(app.world().get::<Autolathe>(lathe).unwrap().metal, 3700);
    assert_eq!(count(&mut app, "crowbar"), before, "not yet");
    press(&mut app, mob, lathe, line_of("wrench"));
    assert_eq!(
        app.world().get::<Autolathe>(lathe).unwrap().metal,
        3700,
        "one at a time"
    );
    for _ in 0..40 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    assert_eq!(count(&mut app, "crowbar"), before + 1);
    assert!(app.world().get::<Autolathe>(lathe).unwrap().making.is_none());
}

#[test]
fn it_refuses_what_it_cant_afford_or_take() {
    let (mut app, mob, lathe) = setup();
    press(&mut app, mob, lathe, line_of("crowbar"));
    assert!(app.world().get::<Autolathe>(lathe).unwrap().making.is_none());

    let wrench = spawn_object(app.world_mut(), &ObjectSpec::new("wrench"), NEAR).unwrap();
    app.world_mut().run_schedule(FixedUpdate);
    click(&mut app, mob, wrench);
    click(&mut app, mob, lathe);
    assert_eq!(
        app.world().get::<Outbox>(mob).unwrap().0.last().unwrap().id,
        "lathe-refuses"
    );
}

#[test]
fn a_full_one_takes_no_more() {
    let (mut app, mob, lathe) = setup();
    app.world_mut().get_mut::<Autolathe>(lathe).unwrap().metal = MAX_METAL;
    feed(&mut app, mob, lathe, "metal-sheet");
    assert_eq!(app.world().get::<Autolathe>(lathe).unwrap().metal, MAX_METAL);
    assert_eq!(
        app.world().get::<Outbox>(mob).unwrap().0.last().unwrap().id,
        "lathe-full"
    );
}
