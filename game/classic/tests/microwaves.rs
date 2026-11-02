//! Microwaves. Spec: docs/reference/microwaves.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::closets::InCloset;
use rs13_classic::food::Snack;
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::health::Body;
use rs13_classic::microwaves::{
    BURNT_MESS, COOK_DS, DONK_POCKET, Microwave, WARM_BITES, WARM_DS, Warm, run_microwaves,
};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::power::PowerUser;
use rs13_world::{Appearance, GridPos, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'm': (tile: (id: "floor"), objects: ["microwave"]),
    },
    rows: [
        "WWWWW",
        "W.m.W",
        "W...W",
        "WWWWW",
    ],
)"#;
const STANDING: TilePos = TilePos::new(2, 1, 0);
const OVEN_AT: TilePos = TilePos::new(2, 2, 0);

fn setup() -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), STANDING);
    let mut q = app.world_mut().query_filtered::<Entity, With<Microwave>>();
    let oven = q.single(app.world()).unwrap();
    app.world_mut().get_mut::<PowerUser>(oven).unwrap().powered = true;
    (app, mob, oven)
}

fn ticks(app: &mut App, n: u32) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn hold(app: &mut App, mob: Entity, proto: &str) -> Entity {
    let item = spawn_object(app.world_mut(), &ObjectSpec::new(proto), STANDING).unwrap();
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    let mut hands = app.world_mut().get_mut::<Hands>(mob).unwrap();
    let active = hands.active;
    *hands.slot_mut(active) = Some(item);
    item
}

fn click(app: &mut App, mob: Entity, oven: Entity) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(oven));
    ticks(app, 1);
}

fn cook(app: &mut App) {
    // A little longer than it takes: 2 ticks a decisecond.
    ticks(app, (COOK_DS * 2.0) as u32 + 4);
}

#[test]
fn a_donk_pocket_comes_out_warm_and_heals_as_its_eaten_until_it_cools() {
    let (mut app, mob, oven) = setup();
    let pocket = hold(&mut app, mob, DONK_POCKET);
    click(&mut app, mob, oven);
    assert!(app.world().get::<InCloset>(pocket).is_some());
    assert_eq!(app.world().get::<Hands>(mob).unwrap().active_item(), None);

    click(&mut app, mob, oven);
    assert_eq!(app.world().get::<Appearance>(oven).unwrap().state, "mw1");
    cook(&mut app);
    assert_eq!(app.world().get::<Appearance>(oven).unwrap().state, "mw");
    assert_eq!(app.world().get::<GridPos>(pocket).unwrap().0, OVEN_AT);
    assert!(app.world().get::<InCloset>(pocket).is_none());
    assert!(app.world().get::<Warm>(pocket).is_some());
    assert_eq!(app.world().get::<Snack>(pocket).unwrap().bites_left, WARM_BITES);

    // A bite while warm heals.
    app.world_mut().get_mut::<Body>(mob).unwrap().oxy = 10.0;
    let mut item = app.world_mut().entity_mut(pocket);
    item.remove::<GridPos>().insert(HeldBy(mob));
    let mut hands = app.world_mut().get_mut::<Hands>(mob).unwrap();
    let active = hands.active;
    *hands.slot_mut(active) = Some(pocket);
    click(&mut app, mob, mob);
    assert_eq!(app.world().get::<Body>(mob).unwrap().oxy, 7.5);
    let mut hands = app.world_mut().get_mut::<Hands>(mob).unwrap();
    *hands.slot_mut(active) = None;

    ticks(&mut app, (WARM_DS * 2.0) as u32 + 2);
    assert!(app.world().get::<Warm>(pocket).is_none(), "it cools");
}

#[test]
fn two_things_at_once_burn_to_one_mess() {
    let (mut app, mob, oven) = setup();
    let a = hold(&mut app, mob, DONK_POCKET);
    click(&mut app, mob, oven);
    let b = hold(&mut app, mob, DONK_POCKET);
    click(&mut app, mob, oven);
    click(&mut app, mob, oven);
    cook(&mut app);
    assert!(app.world().get_entity(a).is_err());
    assert!(app.world().get_entity(b).is_err());
    let mut q = app.world_mut().query::<(&Appearance, &GridPos)>();
    let mess = q.iter(app.world()).find(|(a, _)| a.sprite == BURNT_MESS).unwrap();
    assert_eq!(mess.1.0, OVEN_AT);
}

#[test]
fn it_only_takes_what_it_can_cook() {
    let (mut app, mob, oven) = setup();
    let wrench = hold(&mut app, mob, "wrench");
    click(&mut app, mob, oven);
    assert_eq!(app.world().get::<Hands>(mob).unwrap().active_item(), Some(wrench));
    let last = app
        .world()
        .get::<Outbox>(mob)
        .unwrap()
        .0
        .last()
        .unwrap()
        .id
        .clone();
    assert_eq!(last, "microwave-unsuitable");
}

#[test]
fn a_power_cut_stops_it_with_the_food_still_inside() {
    let (mut app, mob, oven) = setup();
    let pocket = hold(&mut app, mob, DONK_POCKET);
    click(&mut app, mob, oven);
    click(&mut app, mob, oven);
    assert!(app.world().get::<Microwave>(oven).unwrap().done_at.is_some());
    // No APC on this map says otherwise, so what we set holds for the tick.
    app.world_mut().get_mut::<PowerUser>(oven).unwrap().powered = false;
    run_microwaves(app.world_mut());
    assert!(app.world().get::<Microwave>(oven).unwrap().done_at.is_none());
    assert_eq!(app.world().get::<Appearance>(oven).unwrap().state, "mw");
    assert!(app.world().get::<InCloset>(pocket).is_some());
}
