//! Welding tools: lighting them, fuel, fuel tanks and welding airlocks shut.
//! Spec: docs/reference/welding.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::doors::{Airlock, DoorState};
use rs13_classic::hands::Hands;
use rs13_classic::health::{Body, Zone};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::welding::{FuelTank, Welder};
use rs13_world::{Appearance, ObjectSpec, TileMap, TilePos};

/// A fuel tank and an airlock next to a bit of floor.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'F': (tile: (id: "floor"), objects: ["fuel-tank"]),
        'D': (tile: (id: "floor"), objects: ["airlock"]),
    },
    rows: [
        "WWWWW",
        "WF.DW",
        "W...W",
        "WWWWW",
    ],
)"#;

const STAND: TilePos = TilePos::new(2, 2, 0);

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), STAND);
    tick(&mut app);
    (app, mob)
}

fn tick(app: &mut App) {
    app.world_mut().run_schedule(FixedUpdate);
}

fn act(app: &mut App, mob: Entity, action: Action) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
    tick(app);
}

fn give(app: &mut App, mob: Entity, proto: &str) -> Entity {
    let item = spawn_object(app.world_mut(), &ObjectSpec::new(proto), STAND).unwrap();
    act(app, mob, Action::UseOn(item));
    assert_eq!(app.world().get::<Hands>(mob).unwrap().active_item(), Some(item));
    item
}

fn last_told(app: &App, mob: Entity) -> Option<String> {
    app.world()
        .get::<Outbox>(mob)
        .unwrap()
        .0
        .last()
        .map(|m| m.id.clone())
}

fn welder(app: &App, item: Entity) -> Welder {
    *app.world().get::<Welder>(item).unwrap()
}

fn find(app: &mut App, sprite: &str) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &Appearance)>();
    q.iter(app.world())
        .find(|(_, a)| a.sprite == sprite)
        .map(|(e, _)| e)
        .unwrap()
}

#[test]
fn light_it_in_hand_and_put_it_out() {
    let (mut app, mob) = setup();
    let tool = give(&mut app, mob, "welder");
    act(&mut app, mob, Action::Examine(tool));
    assert_eq!(last_told(&app, mob).as_deref(), Some("examine-welder"));

    act(&mut app, mob, Action::UseOn(tool));
    assert!(welder(&app, tool).lit);
    assert_eq!(last_told(&app, mob).as_deref(), Some("welder-on"));
    tick(&mut app);
    assert_eq!(app.world().get::<Appearance>(tool).unwrap().state, "lit");

    act(&mut app, mob, Action::UseOn(tool));
    assert!(!welder(&app, tool).lit);
    assert_eq!(last_told(&app, mob).as_deref(), Some("welder-off"));

    // Empty, it won't light.
    app.world_mut().get_mut::<Welder>(tool).unwrap().fuel = 0;
    act(&mut app, mob, Action::UseOn(tool));
    assert!(!welder(&app, tool).lit);
    assert_eq!(last_told(&app, mob).as_deref(), Some("welder-no-fuel"));
}

#[test]
fn weld_an_airlock_shut_and_open_again() {
    let (mut app, mob) = setup();
    let door = find(&mut app, "airlock");
    let tool = give(&mut app, mob, "welder");
    act(&mut app, mob, Action::UseOn(tool));

    // Two units for the weld, one more for using a lit tool on anything.
    act(&mut app, mob, Action::UseOn(door));
    assert!(app.world().get::<Airlock>(door).unwrap().welded);
    assert_eq!(welder(&app, tool).fuel, 27);
    tick(&mut app);
    assert_eq!(
        app.world().get::<Appearance>(door).unwrap().state,
        "closed+welded"
    );

    // Welded, it opens for nothing.
    {
        let mut airlock = app.world_mut().get_mut::<Airlock>(door).unwrap();
        assert!(!airlock.toggle(0.0));
        airlock.powered = false;
        assert!(!airlock.force(0.0));
        airlock.powered = true;
    }
    assert_eq!(app.world().get::<Airlock>(door).unwrap().state, DoorState::Closed);

    act(&mut app, mob, Action::UseOn(door));
    assert!(!app.world().get::<Airlock>(door).unwrap().welded);
    assert_eq!(welder(&app, tool).fuel, 24);
}

#[test]
fn running_out_of_fuel() {
    let (mut app, mob) = setup();
    let door = find(&mut app, "airlock");
    let tool = give(&mut app, mob, "welder");
    act(&mut app, mob, Action::UseOn(tool));

    // Two units are not enough for a door, and the one used up leaves one.
    app.world_mut().get_mut::<Welder>(tool).unwrap().fuel = 2;
    act(&mut app, mob, Action::UseOn(door));
    assert!(!app.world().get::<Airlock>(door).unwrap().welded);
    assert_eq!(last_told(&app, mob).as_deref(), Some("welder-short"));
    assert_eq!(welder(&app, tool).fuel, 1);

    // The last unit puts it out.
    act(&mut app, mob, Action::UseOnTile(STAND));
    assert_eq!(welder(&app, tool), Welder { fuel: 0, lit: false });
    assert_eq!(last_told(&app, mob).as_deref(), Some("welder-no-fuel"));
}

#[test]
fn refill_at_a_fuel_tank() {
    let (mut app, mob) = setup();
    let tank = find(&mut app, "fuel-tank");
    let tool = give(&mut app, mob, "welder");
    app.world_mut().get_mut::<Welder>(tool).unwrap().fuel = 5;
    act(&mut app, mob, Action::UseOn(tank));
    assert_eq!(welder(&app, tool).fuel, 30);
    assert_eq!(app.world().get::<FuelTank>(tank).unwrap().fuel, 975);
    assert_eq!(last_told(&app, mob).as_deref(), Some("welder-refueled"));

    // Lit, it takes nothing from the tank.
    act(&mut app, mob, Action::UseOn(tool));
    act(&mut app, mob, Action::UseOn(tank));
    assert_eq!(welder(&app, tool).fuel, 29);
    assert_eq!(app.world().get::<FuelTank>(tank).unwrap().fuel, 975);
    act(&mut app, mob, Action::Examine(tank));
    assert_eq!(last_told(&app, mob).as_deref(), Some("examine-fuel-tank"));
}

#[test]
fn a_lit_welder_burns() {
    let (mut app, mob) = setup();
    let victim = spawn_player_mob(app.world_mut(), TilePos::new(2, 1, 0));
    let tool = give(&mut app, mob, "welder");
    act(&mut app, mob, Action::UseOn(tool));
    act(&mut app, mob, Action::UseOn(victim));
    let body = app.world().get::<Body>(victim).unwrap();
    let burn: f32 = Zone::ALL.iter().map(|z| body.organ(*z).burn).sum();
    let brute: f32 = Zone::ALL.iter().map(|z| body.organ(*z).brute).sum();
    assert_eq!((brute, burn), (0.0, 15.0));
    assert_eq!(welder(&app, tool).fuel, 29);
}
