//! Flashbangs and EMP grenades. Spec: docs/reference/grenades.md.

use std::collections::BTreeSet;

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::changeling::Deafened;
use rs13_classic::equipment::Equipment;
use rs13_classic::flashes::Weakened;
use rs13_classic::genetics::{Gene, Mutations};
use rs13_classic::grenades::{self, EarDamage, EyeDamage, Flying, Grenade};
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::radio::Radio;
use rs13_classic::weapons::Gun;
use rs13_world::{Appearance, GridPos, Hand, HumanLook, ObjectSpec, Slot, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
    },
    rows: [
        "WWWWWWWWWWWWWWWWWWWW",
        "W.................WW",
        "W.................WW",
        "W.......WWWWWW....WW",
        "W.........W.......WW",
        "W.........W.......WW",
        "WWWWWWWWWWWWWWWWWWWW",
    ],
)"#;

/// Ticks in a decisecond.
const TICKS_PER_DS: u32 = 2;

fn at(x: i32, y: i32) -> TilePos {
    TilePos::new(x, y, 0)
}

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

fn wait_ds(app: &mut App, ds: u32) {
    tick(app, ds * TICKS_PER_DS);
}

fn person(app: &mut App, pos: TilePos) -> Entity {
    let mob = spawn_player_mob(app.world_mut(), pos);
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

fn spawn(app: &mut App, id: &str, pos: TilePos) -> Entity {
    spawn_object(app.world_mut(), &ObjectSpec::new(id), pos).unwrap()
}

/// Put an item in a mob's active hand.
fn give(app: &mut App, mob: Entity, item: Entity) {
    let hand = app.world().get::<Hands>(mob).unwrap().active;
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    *app.world_mut().get_mut::<Hands>(mob).unwrap().slot_mut(hand) = Some(item);
}

fn wear(app: &mut App, mob: Entity, slot: Slot, id: &str) {
    let item = spawn(app, id, at(1, 1));
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(rs13_classic::equipment::WornBy(mob));
    app.world_mut()
        .get_mut::<Equipment>(mob)
        .unwrap()
        .0
        .insert(slot, item);
}

fn gone(app: &App, e: Entity) -> bool {
    app.world().get_entity(e).is_err()
}

fn down(app: &App, mob: Entity) -> bool {
    app.world().get::<Weakened>(mob).is_some()
}

#[test]
fn a_flashbang_primed_in_hand_goes_off_after_three_seconds() {
    let mut app = setup();
    let mob = person(&mut app, at(3, 2));
    let bang = spawn(&mut app, grenades::FLASHBANG, at(3, 2));
    give(&mut app, mob, bang);
    act(&mut app, mob, Action::UseOn(bang));
    assert_eq!(app.world().get::<Appearance>(bang).unwrap().state, "active");
    assert!(app.world().get::<Grenade>(bang).unwrap().goes_off_ds.is_some());
    wait_ds(&mut app, 25);
    assert!(!gone(&app, bang), "not yet");
    wait_ds(&mut app, 10);
    assert!(gone(&app, bang));
    assert_eq!(app.world().get::<Hands>(mob).unwrap().active_item(), None);
    // Held when it went off: the worst of it.
    assert!(down(&app, mob));
    assert!(app.world().get::<Deafened>(mob).is_some());
    assert!(app.world().get::<EyeDamage>(mob).unwrap().0 >= 2.0);
}

#[test]
fn a_screwdriver_sets_a_flashbang_to_ten_seconds() {
    let mut app = setup();
    let mob = person(&mut app, at(3, 2));
    let bang = spawn(&mut app, grenades::FLASHBANG, at(3, 2));
    let screwdriver = spawn(&mut app, "screwdriver", at(3, 2));
    give(&mut app, mob, screwdriver);
    act(&mut app, mob, Action::UseOn(bang));
    assert_eq!(
        app.world().get::<Grenade>(bang).unwrap().delay_ds,
        grenades::FLASHBANG_LONG_DS
    );
    app.world_mut().entity_mut(screwdriver).despawn();
    give(&mut app, mob, bang);
    act(&mut app, mob, Action::UseOn(bang));
    wait_ds(&mut app, 50);
    assert!(!gone(&app, bang));
    wait_ds(&mut app, 55);
    assert!(gone(&app, bang));
}

#[test]
fn a_thrown_flashbang_flies_there_and_knocks_down_those_who_see_it() {
    let mut app = setup();
    let thrower = person(&mut app, at(1, 4));
    let near = person(&mut app, at(10, 5));
    let shaded = person(&mut app, at(13, 4));
    wear(&mut app, shaded, Slot::Glasses, "sunglasses");
    let behind_wall = person(&mut app, at(12, 1));
    let bang = spawn(&mut app, grenades::FLASHBANG, at(1, 4));
    give(&mut app, thrower, bang);
    act(&mut app, thrower, Action::UseOnTile(at(10, 4)));
    // Out of the hand, primed, on its way.
    assert_eq!(app.world().get::<Hands>(thrower).unwrap().active_item(), None);
    assert!(app.world().get::<Flying>(bang).is_some());
    assert!(app.world().get::<Grenade>(bang).unwrap().goes_off_ds.is_some());
    wait_ds(&mut app, 3 * 9 + 2);
    assert_eq!(app.world().get::<GridPos>(bang).unwrap().0, at(10, 4));
    wait_ds(&mut app, 10);
    assert!(gone(&app, bang));
    assert!(down(&app, near), "right by it");
    assert!(!down(&app, shaded), "sunglasses at four tiles");
    assert!(app.world().get::<Deafened>(shaded).is_some());
    assert!(!down(&app, behind_wall), "can't see it");
    assert!(app.world().get::<Deafened>(behind_wall).is_none());
    // The thrower, nine tiles off, only dazzled and a little deafened.
    assert!(!down(&app, thrower));
}

#[test]
fn a_thrown_grenade_stops_at_a_wall() {
    let mut app = setup();
    let thrower = person(&mut app, at(8, 2));
    let bang = spawn(&mut app, grenades::FLASHBANG, at(8, 2));
    // Ten seconds, so it lies there long enough to look at.
    app.world_mut().get_mut::<Grenade>(bang).unwrap().delay_ds = grenades::FLASHBANG_LONG_DS;
    give(&mut app, thrower, bang);
    act(&mut app, thrower, Action::UseOnTile(at(14, 2)));
    wait_ds(&mut app, 30);
    assert_eq!(app.world().get::<GridPos>(bang).unwrap().0, at(9, 2));
    assert!(app.world().get::<Flying>(bang).is_none());
}

#[test]
fn a_clumsy_person_sets_it_off_at_once() {
    let mut app = setup();
    let mob = person(&mut app, at(3, 2));
    app.world_mut()
        .entity_mut(mob)
        .insert(Mutations(BTreeSet::from([Gene::Clumsy])));
    let bang = spawn(&mut app, grenades::FLASHBANG, at(3, 2));
    give(&mut app, mob, bang);
    act(&mut app, mob, Action::UseOn(bang));
    wait_ds(&mut app, 6);
    assert!(gone(&app, bang));
}

#[test]
fn ears_heal_slowly() {
    let mut app = setup();
    let mob = person(&mut app, at(3, 2));
    app.world_mut().entity_mut(mob).insert(EarDamage(10.0));
    wait_ds(&mut app, 200);
    let left = app.world().get::<EarDamage>(mob).unwrap().0;
    assert!(left < 10.0 && left > 9.0, "{left}");
}

#[test]
fn an_emp_grenade_drains_guns_and_kills_radios() {
    let mut app = setup();
    let mob = person(&mut app, at(3, 2));
    let radio = spawn(&mut app, "radio", at(3, 2));
    give(&mut app, mob, radio);
    let gun = spawn(&mut app, "energy-gun", at(5, 2));
    let charge = app.world().get::<Gun>(gun).unwrap().charge;
    let emp = spawn(&mut app, grenades::EMP_GRENADE, at(3, 2));
    let other = app.world().get::<Hands>(mob).unwrap().active;
    let other = if other == Hand::Left {
        Hand::Right
    } else {
        Hand::Left
    };
    app.world_mut().get_mut::<Hands>(mob).unwrap().active = other;
    give(&mut app, mob, emp);
    act(&mut app, mob, Action::UseOn(emp));
    wait_ds(&mut app, 52);
    assert!(gone(&app, emp));
    assert!(app.world().get::<Gun>(gun).unwrap().charge < charge);
    let r = app.world().get::<Radio>(radio).unwrap();
    assert!(!r.broadcasting && !r.listening);
}

#[test]
fn security_lockers_and_the_uplink_have_them() {
    let mut app = setup();
    let locker = spawn(&mut app, "locker-hos", at(3, 2));
    let _ = locker;
    let mut boxes = app.world_mut().query::<&Appearance>();
    assert!(
        boxes
            .iter(app.world())
            .any(|a| a.sprite == grenades::FLASHBANG_BOX)
    );
    assert!(
        rs13_classic::traitor::GOODS
            .iter()
            .any(|(id, _)| *id == grenades::EMP_BOX)
    );
}
