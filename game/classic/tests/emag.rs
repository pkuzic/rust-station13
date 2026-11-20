//! The emag. Spec: docs/reference/emag.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::closets::{Closet, ClosetLock};
use rs13_classic::doors::{Airlock, DoorState};
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::power::{Apc, PowerUser};
use rs13_classic::rng::GameRng;
use rs13_classic::traitor::GOODS;
use rs13_classic::vending::{PendingVendOps, Vending};
use rs13_classic::windoors::{Swing, Windoor};
use rs13_world::{GridPos, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
    },
    rows: [
        "WWWWWWW",
        "W.....W",
        "W.....W",
        "W.....W",
        "WWWWWWW",
    ],
)"#;

const HERE: TilePos = TilePos::new(2, 2, 0);
const NEXT: TilePos = TilePos::new(3, 2, 0);

fn setup(machine: &str) -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    tick(&mut app, 1);
    let mob = spawn_player_mob(app.world_mut(), HERE);
    let machine = spawn_object(app.world_mut(), &ObjectSpec::new(machine), NEXT).unwrap();
    tick(&mut app, 1);
    power(&mut app, machine);
    (app, mob, machine)
}

/// No APC in this room: power it by hand.
fn power(app: &mut App, machine: Entity) {
    if let Some(mut p) = app.world_mut().get_mut::<PowerUser>(machine) {
        p.powered = true;
    }
}

fn tick(app: &mut App, n: u32) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn seconds(app: &mut App, machine: Entity, ds: f32) {
    for _ in 0..(ds * 2.0) as u32 {
        power(app, machine);
        tick(app, 1);
    }
}

fn hold(app: &mut App, mob: Entity, id: &str) {
    let item = spawn_object(app.world_mut(), &ObjectSpec::new(id), HERE).unwrap();
    let hand = app.world().get::<Hands>(mob).unwrap().active;
    if let Some(old) = app
        .world_mut()
        .get_mut::<Hands>(mob)
        .unwrap()
        .slot_mut(hand)
        .take()
    {
        app.world_mut().entity_mut(old).despawn();
    }
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    *app.world_mut().get_mut::<Hands>(mob).unwrap().slot_mut(hand) = Some(item);
}

fn empty_hand(app: &mut App, mob: Entity) {
    let hand = app.world().get::<Hands>(mob).unwrap().active;
    if let Some(old) = app
        .world_mut()
        .get_mut::<Hands>(mob)
        .unwrap()
        .slot_mut(hand)
        .take()
    {
        app.world_mut().entity_mut(old).despawn();
    }
}

fn use_on(app: &mut App, mob: Entity, target: Entity) -> Vec<String> {
    let before = app.world().get::<Outbox>(mob).unwrap().0.len();
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    power(app, target);
    tick(app, 2);
    app.world().get::<Outbox>(mob).unwrap().0[before..]
        .iter()
        .map(|m| m.id.clone())
        .collect()
}

fn said(said: &[String], id: &str) -> bool {
    said.iter().any(|s| s == id)
}

#[test]
fn the_uplink_sells_the_emag_for_three_crystals() {
    assert!(GOODS.contains(&("emag", 3)));
}

#[test]
fn an_emagged_airlock_opens_and_never_moves_again() {
    let (mut app, mob, door) = setup("airlock");
    hold(&mut app, mob, "emag");
    use_on(&mut app, mob, door);
    seconds(&mut app, door, 10.0);
    let state = |app: &App| app.world().get::<Airlock>(door).unwrap().state;
    assert!(matches!(state(&app), DoorState::Open { .. }), "{:?}", state(&app));
    assert!(app.world().get::<Airlock>(door).unwrap().broken);
    // No autoclose, no hand, no crowbar.
    seconds(&mut app, door, 200.0);
    empty_hand(&mut app, mob);
    use_on(&mut app, mob, door);
    hold(&mut app, mob, "crowbar");
    use_on(&mut app, mob, door);
    seconds(&mut app, door, 10.0);
    assert!(matches!(state(&app), DoorState::Open { .. }), "{:?}", state(&app));
}

#[test]
fn a_bolted_airlock_breaks_but_stays_shut() {
    let (mut app, mob, door) = setup("airlock");
    app.world_mut().get_mut::<Airlock>(door).unwrap().bolted = true;
    hold(&mut app, mob, "emag");
    use_on(&mut app, mob, door);
    seconds(&mut app, door, 10.0);
    let airlock = app.world().get::<Airlock>(door).unwrap().clone();
    assert!(airlock.broken);
    assert_eq!(airlock.state, DoorState::Closed);
    app.world_mut().get_mut::<Airlock>(door).unwrap().bolted = false;
    empty_hand(&mut app, mob);
    use_on(&mut app, mob, door);
    seconds(&mut app, door, 10.0);
    assert_eq!(app.world().get::<Airlock>(door).unwrap().state, DoorState::Closed);
}

#[test]
fn an_emagged_window_door_swings_open_for_good() {
    let (mut app, mob, door) = setup("windoor");
    hold(&mut app, mob, "emag");
    use_on(&mut app, mob, door);
    seconds(&mut app, door, 20.0);
    let windoor = app.world().get::<Windoor>(door).unwrap().clone();
    assert!(windoor.broken);
    assert!(matches!(windoor.swing, Swing::Open { .. }), "{:?}", windoor.swing);
    empty_hand(&mut app, mob);
    use_on(&mut app, mob, door);
    seconds(&mut app, door, 20.0);
    assert!(!app.world().get::<Windoor>(door).unwrap().sealed());
}

#[test]
fn an_emag_breaks_a_locker_open_and_a_card_does_nothing_after() {
    let (mut app, mob, locker) = setup("locker-doctor");
    hold(&mut app, mob, "emag");
    use_on(&mut app, mob, locker);
    let lock = app.world().get::<ClosetLock>(locker).unwrap().clone();
    assert!(lock.broken && !lock.locked);
    let again = use_on(&mut app, mob, locker);
    assert!(said(&again, "locker-broken"), "{again:?}");
    empty_hand(&mut app, mob);
    use_on(&mut app, mob, locker);
    assert!(app.world().get::<Closet>(locker).unwrap().opened);
}

#[test]
fn an_emag_unlocks_a_secure_crate() {
    let (mut app, mob, crate_) = setup("crate-secure");
    if let Some(mut lock) = app.world_mut().get_mut::<ClosetLock>(crate_) {
        lock.locked = true;
    }
    hold(&mut app, mob, "emag");
    let told = use_on(&mut app, mob, crate_);
    assert!(said(&told, "emag-crate"), "{told:?}");
    assert!(!app.world().get::<ClosetLock>(crate_).unwrap().locked);
}

#[test]
fn an_emagged_vending_machine_sells_without_access_even_once_mended() {
    let (mut app, mob, machine) = setup("vending-medical");
    hold(&mut app, mob, "emag");
    let told = use_on(&mut app, mob, machine);
    assert!(said(&told, "emag-vending"), "{told:?}");
    let stock = |app: &App| -> u16 { app.world().get::<Vending>(machine).unwrap().stock[0] };
    let full = stock(&app);
    app.world_mut()
        .get_mut::<PendingVendOps>(mob)
        .unwrap()
        .0
        .push((machine, 0));
    power(&mut app, machine);
    tick(&mut app, 2);
    assert_eq!(stock(&app), full - 1);
}

#[test]
fn an_emag_breaks_an_apc_interface_half_the_time() {
    let mut broke = 0;
    for seed in 0..20 {
        let (mut app, mob, apc) = setup("apc");
        *app.world_mut().resource_mut::<GameRng>() = GameRng::seeded(seed);
        hold(&mut app, mob, "emag");
        let told = use_on(&mut app, mob, apc);
        let state = app.world().get::<Apc>(apc).unwrap().clone();
        if state.emagged {
            broke += 1;
            assert!(!state.locked);
            assert!(said(&told, "emag-apc"), "{told:?}");
            hold(&mut app, mob, "screwdriver");
            let screw = use_on(&mut app, mob, apc);
            assert!(said(&screw, "apc-interface-broken"), "{screw:?}");
            hold(&mut app, mob, "id-card");
            let card = use_on(&mut app, mob, apc);
            assert!(said(&card, "apc-interface-broken"), "{card:?}");
        } else {
            assert!(state.locked);
            assert!(said(&told, "emag-apc-unlock-failed"), "{told:?}");
        }
    }
    assert!((4..=16).contains(&broke), "{broke}");
}

#[test]
fn an_emag_does_nothing_with_the_apc_cover_open() {
    let (mut app, mob, apc) = setup("apc");
    app.world_mut().get_mut::<Apc>(apc).unwrap().opened = true;
    hold(&mut app, mob, "emag");
    let told = use_on(&mut app, mob, apc);
    assert!(said(&told, "apc-close-cover-first"), "{told:?}");
    assert!(!app.world().get::<Apc>(apc).unwrap().emagged);
}
