//! Handcuffs and turrets. Spec: docs/reference/security.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::equipment::Equipment;
use rs13_classic::flashes::Weakened;
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::health::Body;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::security::{BREAKOUT_DS, CUFF_DS, Cuffed, Turret, TurretControl, WantsResist};
use rs13_world::{GridPos, Hidden, ObjectSpec, Slot, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'g': (tile: (id: "floor")),
        't': (tile: (id: "floor"), objects: ["turret"]),
        'c': (tile: (id: "floor"), objects: ["turret-control"]),
        'A': (tile: (id: "floor"), objects: ["apc"]),
    },
    rows: [
        "WWWWWWWWWWW",
        "W....WAggtW",
        "W....WgcggW",
        "WWWWWWWWWWW",
    ],
    area_legend: { 'a': "turret-protected-ai", 'h': "hallway" },
    area_rows: [
        "           ",
        " hhhh aaaa ",
        " hhhh aaaa ",
        "           ",
    ],
)"#;
const OFFICER_AT: TilePos = TilePos::new(1, 1, 0);
const SUSPECT_AT: TilePos = TilePos::new(2, 1, 0);
const IN_CORE: TilePos = TilePos::new(6, 2, 0);

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().run_schedule(FixedUpdate);
    app
}

fn ticks(app: &mut App, n: u32) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn give(app: &mut App, mob: Entity, spec: &ObjectSpec) -> Entity {
    let item = spawn_object(app.world_mut(), spec, OFFICER_AT).unwrap();
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    let mut hands = app.world_mut().get_mut::<Hands>(mob).unwrap();
    let active = hands.active;
    *hands.slot_mut(active) = Some(item);
    item
}

fn click(app: &mut App, mob: Entity, target: Entity) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    ticks(app, 1);
}

fn cuff(app: &mut App, officer: Entity, suspect: Entity) -> Entity {
    let cuffs = give(app, officer, &ObjectSpec::new("handcuffs"));
    click(app, officer, suspect);
    assert!(app.world().get::<Cuffed>(suspect).is_none(), "not yet");
    ticks(app, (CUFF_DS * 2.0) as u32 + 2);
    assert!(app.world().get::<Cuffed>(suspect).is_some());
    cuffs
}

#[test]
fn cuffing_takes_a_moment_and_ties_their_hands_but_not_their_feet() {
    let mut app = setup();
    let officer = spawn_player_mob(app.world_mut(), OFFICER_AT);
    let suspect = spawn_player_mob(app.world_mut(), SUSPECT_AT);
    let cuffs = cuff(&mut app, officer, suspect);
    assert_eq!(app.world().get::<Hands>(officer).unwrap().active_item(), None);
    assert_eq!(
        app.world().get::<Equipment>(suspect).unwrap().get(Slot::Cuffs),
        Some(cuffs)
    );

    let item = spawn_object(app.world_mut(), &ObjectSpec::new("wrench"), SUSPECT_AT).unwrap();
    click(&mut app, suspect, item);
    assert_eq!(
        app.world().get::<Hands>(suspect).unwrap().active_item(),
        None,
        "no hands"
    );
}

#[test]
fn moving_away_stops_the_cuffing() {
    let mut app = setup();
    let officer = spawn_player_mob(app.world_mut(), OFFICER_AT);
    let suspect = spawn_player_mob(app.world_mut(), SUSPECT_AT);
    give(&mut app, officer, &ObjectSpec::new("handcuffs"));
    click(&mut app, officer, suspect);
    app.world_mut().get_mut::<GridPos>(suspect).unwrap().0 = TilePos::new(4, 2, 0);
    ticks(&mut app, (CUFF_DS * 2.0) as u32 + 2);
    assert!(app.world().get::<Cuffed>(suspect).is_none());
}

#[test]
fn someone_else_can_take_them_off_and_they_land_on_the_floor() {
    let mut app = setup();
    let officer = spawn_player_mob(app.world_mut(), OFFICER_AT);
    let suspect = spawn_player_mob(app.world_mut(), SUSPECT_AT);
    let cuffs = cuff(&mut app, officer, suspect);
    click(&mut app, officer, suspect);
    ticks(&mut app, (CUFF_DS * 2.0) as u32 + 2);
    assert!(app.world().get::<Cuffed>(suspect).is_none());
    assert_eq!(app.world().get::<GridPos>(cuffs).unwrap().0, SUSPECT_AT);
    assert_eq!(
        app.world().get::<Equipment>(suspect).unwrap().get(Slot::Cuffs),
        None
    );
}

#[test]
fn breaking_out_takes_long_and_needs_standing_still() {
    let mut app = setup();
    let officer = spawn_player_mob(app.world_mut(), OFFICER_AT);
    let suspect = spawn_player_mob(app.world_mut(), SUSPECT_AT);
    cuff(&mut app, officer, suspect);
    app.world_mut().entity_mut(suspect).insert(WantsResist(true));
    ticks(&mut app, 2);
    app.world_mut().get_mut::<GridPos>(suspect).unwrap().0 = TilePos::new(3, 1, 0);
    ticks(&mut app, 2);
    app.world_mut().entity_mut(suspect).insert(WantsResist(true));
    ticks(&mut app, (BREAKOUT_DS * 2.0) as u32 - 20);
    assert!(app.world().get::<Cuffed>(suspect).is_some(), "not yet");
    ticks(&mut app, 30);
    assert!(app.world().get::<Cuffed>(suspect).is_none());
}

fn turret(app: &mut App) -> Entity {
    let mut q = app.world_mut().query_filtered::<Entity, With<Turret>>();
    q.single(app.world()).unwrap()
}

#[test]
fn a_turret_rises_for_an_intruder_stuns_them_and_sinks_back() {
    let mut app = setup();
    let t = turret(&mut app);
    assert!(app.world().get::<Hidden>(t).is_some());
    let outside = spawn_player_mob(app.world_mut(), OFFICER_AT);
    ticks(&mut app, 40);
    assert!(app.world().get::<Hidden>(t).is_some(), "nobody in its area");

    let intruder = spawn_player_mob(app.world_mut(), IN_CORE);
    let mut rose = false;
    for _ in 0..60 {
        ticks(&mut app, 1);
        rose |= app.world().get::<Hidden>(t).is_none();
        if app.world().get::<Weakened>(intruder).is_some() {
            break;
        }
    }
    assert!(rose, "up");
    assert!(app.world().get::<Weakened>(intruder).is_some());
    assert_eq!(
        app.world().get::<Body>(intruder).unwrap().health(),
        100.0,
        "stun does no harm"
    );
    assert!(app.world().get::<Weakened>(outside).is_none());
    ticks(&mut app, 30);
    assert!(app.world().get::<Hidden>(t).is_some(), "nobody up to shoot");
}

#[test]
fn the_control_steps_through_kill_and_off_and_wants_access() {
    let mut app = setup();
    let t = turret(&mut app);
    let mut q = app.world_mut().query_filtered::<Entity, With<TurretControl>>();
    let control = q.single(app.world()).unwrap();
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(6, 1, 0));
    click(&mut app, mob, control);
    let last = app
        .world()
        .get::<Outbox>(mob)
        .unwrap()
        .0
        .last()
        .unwrap()
        .id
        .clone();
    assert_eq!(last, "turret-denied");
    assert!(!app.world().get::<Turret>(t).unwrap().lethal);

    let spec = ObjectSpec {
        access: vec!["ai_upload".to_owned()],
        ..ObjectSpec::new("id-card")
    };
    give(&mut app, mob, &spec);
    click(&mut app, mob, control);
    assert!(app.world().get::<Turret>(t).unwrap().lethal);
    click(&mut app, mob, control);
    assert!(!app.world().get::<Turret>(t).unwrap().enabled);
    click(&mut app, mob, control);
    let state = *app.world().get::<Turret>(t).unwrap();
    assert!(state.enabled && !state.lethal);
}
