//! Scenarios for hands, items and the airlock in the engineering chamber.
//! Specs: docs/reference/hands.md, docs/reference/airlock.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::access::IdCard;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::doors::{Airlock, DoorState};
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::movement::MoveIntent;
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_classic::{ClassicPlugin, maps};
use rs13_world::{Appearance, Dense, Dir, GridPos, Hand, HandsView, NetId, TileMap, TilePos};

/// The chamber with its airlock made public, for tests about door timing.
fn setup() -> App {
    let mut app = setup_as_mapped();
    let door = find(&mut app, "airlock");
    app.world_mut()
        .get_mut::<Airlock>(door)
        .unwrap()
        .req_access
        .clear();
    app
}

/// The chamber exactly as the map defines it.
fn setup_as_mapped() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let map = TileMap::from_ron(maps::ENGINEERING_CHAMBER, 0).unwrap();
    let problems = spawn_map(app.world_mut(), map);
    assert!(problems.is_empty(), "map problems: {problems:?}");
    app
}

fn tick(app: &mut App, n: usize) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

/// The only entity with this sprite.
fn find(app: &mut App, sprite: &str) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &Appearance)>();
    let found: Vec<Entity> = q
        .iter(app.world())
        .filter(|(_, a)| a.sprite == sprite)
        .map(|(e, _)| e)
        .collect();
    assert_eq!(found.len(), 1, "expected exactly one {sprite}");
    found[0]
}

fn pos_of(app: &App, e: Entity) -> Option<TilePos> {
    app.world().get::<GridPos>(e).map(|p| p.0)
}

fn act(app: &mut App, mob: Entity, action: Action) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
}

fn airlock(app: &App, e: Entity) -> Airlock {
    app.world().get::<Airlock>(e).unwrap().clone()
}

#[test]
fn pick_up_swap_hands_and_drop() {
    let mut app = setup();
    let wrench = find(&mut app, "wrench");
    let cutters = find(&mut app, "wirecutters");
    let wrench_pos = pos_of(&app, wrench).unwrap();
    let mob = spawn_player_mob(app.world_mut(), wrench_pos.step(Dir::East));

    act(&mut app, mob, Action::UseOn(wrench));
    tick(&mut app, 1);
    let hands = *app.world().get::<Hands>(mob).unwrap();
    assert_eq!(hands.right, Some(wrench));
    assert_eq!(pos_of(&app, wrench), None, "a held item is not on the floor");
    assert_eq!(app.world().get::<HeldBy>(wrench).unwrap().0, mob);
    let wrench_id = *app.world().get::<NetId>(wrench).unwrap();
    assert_eq!(app.world().get::<HandsView>(mob).unwrap().right, Some(wrench_id));

    // The wirecutters are out of reach: nothing happens.
    act(&mut app, mob, Action::SwapHands);
    act(&mut app, mob, Action::UseOn(cutters));
    tick(&mut app, 1);
    let hands = *app.world().get::<Hands>(mob).unwrap();
    assert_eq!(hands.active, Hand::Left);
    assert_eq!(hands.left, None);
    assert!(pos_of(&app, cutters).is_some());

    // Dropping with the empty left hand does nothing; with the right it drops the wrench.
    act(&mut app, mob, Action::Drop);
    act(&mut app, mob, Action::SwapHands);
    act(&mut app, mob, Action::Drop);
    tick(&mut app, 1);
    assert_eq!(app.world().get::<Hands>(mob).unwrap().right, None);
    assert_eq!(pos_of(&app, wrench), pos_of(&app, mob));
    assert!(app.world().get::<HeldBy>(wrench).is_none());
}

#[test]
fn only_one_mob_gets_an_item_grabbed_in_the_same_tick() {
    let mut app = setup();
    let wrench = find(&mut app, "wrench");
    let p = pos_of(&app, wrench).unwrap();
    let a = spawn_player_mob(app.world_mut(), p.step(Dir::East));
    let b = spawn_player_mob(app.world_mut(), p.step(Dir::West));
    act(&mut app, a, Action::UseOn(wrench));
    act(&mut app, b, Action::UseOn(wrench));
    tick(&mut app, 1);
    let held = [a, b].map(|m| app.world().get::<Hands>(m).unwrap().right);
    assert_eq!(held.iter().filter(|h| **h == Some(wrench)).count(), 1);
}

#[test]
fn airlock_opens_in_6ds_and_autocloses_after_150ds() {
    let mut app = setup();
    let door = find(&mut app, "airlock");
    let door_pos = pos_of(&app, door).unwrap();
    let mob = spawn_player_mob(app.world_mut(), door_pos.step(Dir::West));

    act(&mut app, mob, Action::UseOn(door));
    // Tick 1 starts opening at 0.5 ds; it completes at 6.5 ds, i.e. tick 13.
    tick(&mut app, 12);
    assert!(matches!(airlock(&app, door).state, DoorState::Opening { .. }));
    assert!(
        app.world().get::<Dense>(door).is_some(),
        "still dense while opening"
    );
    tick(&mut app, 1);
    assert!(matches!(airlock(&app, door).state, DoorState::Open { .. }));
    assert!(app.world().get::<Dense>(door).is_none());
    assert_eq!(app.world().get::<Appearance>(door).unwrap().state, "open");

    // 150 ds (300 ticks) later it starts closing, and is closed 6 ds (12 ticks) after that.
    tick(&mut app, 299);
    assert!(matches!(airlock(&app, door).state, DoorState::Open { .. }));
    tick(&mut app, 1);
    assert!(matches!(airlock(&app, door).state, DoorState::Closing { .. }));
    assert!(
        app.world().get::<Dense>(door).is_some(),
        "dense as soon as it starts closing"
    );
    tick(&mut app, 12);
    assert_eq!(airlock(&app, door).state, DoorState::Closed);
}

#[test]
fn walking_into_the_airlock_opens_it_and_lets_the_player_through() {
    let mut app = setup();
    let door = find(&mut app, "airlock");
    let door_pos = pos_of(&app, door).unwrap();
    let mob = spawn_player_mob(app.world_mut(), door_pos.step(Dir::West).step(Dir::West));
    app.world_mut().get_mut::<MoveIntent>(mob).unwrap().dir = Some(Dir::East);

    tick(&mut app, 100);
    let end = pos_of(&app, mob).unwrap();
    assert!(end.x > door_pos.x, "mob should be in the closet, is at {end:?}");
}

#[test]
fn closing_airlock_pushes_the_mob_out_of_the_way() {
    let mut app = setup();
    let door = find(&mut app, "airlock");
    let door_pos = pos_of(&app, door).unwrap();
    let mob = spawn_player_mob(app.world_mut(), door_pos.step(Dir::West));

    act(&mut app, mob, Action::UseOn(door));
    tick(&mut app, 13);
    // Step into the open doorway, then close the door on ourselves.
    app.world_mut().get_mut::<GridPos>(mob).unwrap().0 = door_pos;
    act(&mut app, mob, Action::UseOn(door));
    tick(&mut app, 1);
    assert!(matches!(airlock(&app, door).state, DoorState::Closing { .. }));
    assert_eq!(pos_of(&app, mob), Some(door_pos));

    // Pushed after 4 ds: north and south are walls, east is the first free tile.
    tick(&mut app, 8);
    assert_eq!(pos_of(&app, mob), Some(door_pos.step(Dir::East)));
    let outbox = &app.world().get::<Outbox>(mob).unwrap().0;
    assert_eq!(outbox.last().map(|m| m.id.as_str()), Some("airlock-pushed"));
}

#[test]
fn bolted_or_unpowered_airlock_refuses() {
    let mut app = setup();
    let door = find(&mut app, "airlock");
    let door_pos = pos_of(&app, door).unwrap();
    let mob = spawn_player_mob(app.world_mut(), door_pos.step(Dir::West));

    app.world_mut().get_mut::<Airlock>(door).unwrap().bolted = true;
    act(&mut app, mob, Action::UseOn(door));
    tick(&mut app, 20);
    assert_eq!(airlock(&app, door).state, DoorState::Closed);

    let mut a = app.world_mut().get_mut::<Airlock>(door).unwrap();
    a.bolted = false;
    a.powered = false;
    act(&mut app, mob, Action::UseOn(door));
    tick(&mut app, 20);
    assert_eq!(airlock(&app, door).state, DoorState::Closed);
}

#[test]
fn examine_reports_what_it_is() {
    let mut app = setup();
    let crowbar = find(&mut app, "crowbar");
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(3, 3, 0));
    act(&mut app, mob, Action::Examine(crowbar));
    tick(&mut app, 1);
    let outbox = &app.world().get::<Outbox>(mob).unwrap().0;
    assert_eq!(outbox.len(), 1);
    assert_eq!(outbox[0].id, "examine");
}

/// The ID card granting exactly `access`.
fn card_with(app: &mut App, access: &[&str]) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &IdCard)>();
    q.iter(app.world())
        .find(|(_, c)| c.access.iter().map(String::as_str).eq(access.iter().copied()))
        .unwrap()
        .0
}

fn hold(app: &mut App, mob: Entity, item: Entity) {
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    app.world_mut().get_mut::<Hands>(mob).unwrap().right = Some(item);
}

#[test]
fn engineering_door_needs_an_id_card_in_the_active_hand() {
    let mut app = setup_as_mapped();
    let door = find(&mut app, "airlock");
    let door_pos = pos_of(&app, door).unwrap();
    let mob = spawn_player_mob(app.world_mut(), door_pos.step(Dir::West));

    // Empty-handed: denied, and the door shows it.
    act(&mut app, mob, Action::UseOn(door));
    tick(&mut app, 1);
    assert_eq!(airlock(&app, door).state, DoorState::Closed);
    assert_eq!(app.world().get::<Appearance>(door).unwrap().state, "deny");
    tick(&mut app, 6);
    assert_eq!(app.world().get::<Appearance>(door).unwrap().state, "closed");

    // The assistant card lacks engine_equip.
    let assistant = card_with(&mut app, &["maint_tunnels"]);
    hold(&mut app, mob, assistant);
    act(&mut app, mob, Action::UseOn(door));
    tick(&mut app, 1);
    assert_eq!(airlock(&app, door).state, DoorState::Closed);

    // The engineer card has it: any one matching access is enough.
    let engineer = card_with(
        &mut app,
        &[
            "engine",
            "engine_equip",
            "tech_storage",
            "maint_tunnels",
            "external_airlocks",
        ],
    );
    hold(&mut app, mob, engineer);
    act(&mut app, mob, Action::UseOn(door));
    tick(&mut app, 1);
    assert!(matches!(airlock(&app, door).state, DoorState::Opening { .. }));
}

#[test]
fn bumping_a_restricted_door_needs_access_too() {
    let mut app = setup_as_mapped();
    let door = find(&mut app, "airlock");
    let door_pos = pos_of(&app, door).unwrap();
    let mob = spawn_player_mob(app.world_mut(), door_pos.step(Dir::West));
    app.world_mut().get_mut::<MoveIntent>(mob).unwrap().dir = Some(Dir::East);

    tick(&mut app, 40);
    assert_eq!(
        pos_of(&app, mob),
        Some(door_pos.step(Dir::West)),
        "no card, no entry"
    );

    let engineer = card_with(
        &mut app,
        &[
            "engine",
            "engine_equip",
            "tech_storage",
            "maint_tunnels",
            "external_airlocks",
        ],
    );
    hold(&mut app, mob, engineer);
    tick(&mut app, 60);
    assert!(
        pos_of(&app, mob).unwrap().x > door_pos.x,
        "with the card, through the door"
    );
}
