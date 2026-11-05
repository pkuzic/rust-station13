//! Fire extinguishers, extinguisher cabinets, water tanks and fire closets. Spec:
//! docs/reference/extinguishers.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::atmos::StationAtmos;
use rs13_classic::closets::Closet;
use rs13_classic::extinguishers::{Cabinet, Extinguisher, InCabinet, MAX_WATER, WaterTank};
use rs13_classic::hands::Hands;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_simulation::atmos::gas::T20C;
use rs13_simulation::atmos::{Gas, GasMix};
use rs13_world::{Appearance, GridPos, ObjectSpec, TileMap, TilePos};

/// A closed room 12 tiles long with a water tank and a cabinet at its west end.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        't': (tile: (id: "floor"), objects: ["water-tank"]),
        'c': (tile: (id: "floor"), objects: ["extinguisher-cabinet"]),
    },
    rows: [
        "WWWWWWWWWWWWWW",
        "Wc...........W",
        "W............W",
        "Wt...........W",
        "WWWWWWWWWWWWWW",
    ],
)"#;

const PLAYER: TilePos = TilePos::new(2, 2, 0);
const FIRE: TilePos = TilePos::new(4, 2, 0);

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().run_schedule(FixedUpdate);
    app
}

fn seconds(app: &mut App, s: f64) {
    for _ in 0..(s * 20.0) as u64 {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn act(app: &mut App, mob: Entity, action: Action) -> Vec<String> {
    let before = app.world().get::<Outbox>(mob).unwrap().0.len();
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
    app.world_mut().run_schedule(FixedUpdate);
    app.world().get::<Outbox>(mob).unwrap().0[before..]
        .iter()
        .map(|m| m.id.clone())
        .collect()
}

fn one<C: Component>(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<C>>()
        .single(app.world())
        .unwrap()
}

/// A player holding a fire extinguisher, safety off.
fn armed(app: &mut App) -> (Entity, Entity) {
    let mob = spawn_player_mob(app.world_mut(), PLAYER);
    let ext = spawn_object(app.world_mut(), &ObjectSpec::new("fire-extinguisher"), PLAYER).unwrap();
    act(app, mob, Action::UseOn(ext));
    assert_eq!(app.world().get::<Hands>(mob).unwrap().active_item(), Some(ext));
    assert_eq!(act(app, mob, Action::UseOn(ext)), ["extinguisher-safety-off"]);
    assert_eq!(app.world().get::<Appearance>(ext).unwrap().state, "on");
    (mob, ext)
}

fn burning(app: &App, at: TilePos) -> bool {
    app.world().resource::<StationAtmos>().0.burning_at(at)
}

#[test]
fn the_safety_keeps_it_from_spraying() {
    let mut app = setup();
    let (mob, ext) = armed(&mut app);
    assert_eq!(act(&mut app, mob, Action::UseOn(ext)), ["extinguisher-safety-on"]);
    act(&mut app, mob, Action::UseOnTile(FIRE));
    assert_eq!(app.world().get::<Extinguisher>(ext).unwrap().water, MAX_WATER);
}

/// The room with plasma through it, lit by a flame on the fire's tile, two tiles east of
/// the player.
fn on_fire() -> App {
    let mut app = setup();
    let mut gas = GasMix::default();
    gas.temperature = T20C;
    gas.set(Gas::Plasma, 60.0);
    let atmos = &mut app.world_mut().resource_mut::<StationAtmos>().0;
    atmos.add_at(FIRE, &gas);
    assert!(atmos.expose(FIRE, 3500.0) > 0.0);
    app
}

fn burning_tiles(app: &App) -> usize {
    app.world().resource::<StationAtmos>().0.burning_tiles().len()
}

#[test]
fn left_alone_the_fire_spreads_through_the_room() {
    let mut app = on_fire();
    seconds(&mut app, 10.0);
    assert!(burning_tiles(&app) > 20, "{}", burning_tiles(&app));
}

#[test]
fn a_spray_puts_out_a_fire_as_it_starts_and_a_water_tank_refills_it() {
    let mut app = on_fire();
    let (mob, ext) = armed(&mut app);
    act(&mut app, mob, Action::UseOnTile(FIRE));
    seconds(&mut app, 10.0);
    assert_eq!(burning_tiles(&app), 0, "out for good");
    assert!(!burning(&app, FIRE));
    let left = app.world().get::<Extinguisher>(ext).unwrap().water;
    assert_eq!(left, MAX_WATER - 5, "five units a spray");

    // Two seconds between sprays.
    act(&mut app, mob, Action::UseOnTile(FIRE));
    act(&mut app, mob, Action::UseOnTile(FIRE));
    let left = app.world().get::<Extinguisher>(ext).unwrap().water;
    assert_eq!(left, MAX_WATER - 10);

    // The puffs are gone seven seconds after the last spray.
    seconds(&mut app, 7.0);
    let mut puffs = app.world_mut().query::<&Appearance>();
    let left_puffs = puffs
        .iter(app.world())
        .filter(|a| a.sprite == "water-puff")
        .count();
    assert_eq!(left_puffs, 0);

    // The water tank is at the room's west end, a step away.
    let tank = one::<WaterTank>(&mut app);
    assert_eq!(act(&mut app, mob, Action::UseOn(tank)), ["extinguisher-refilled"]);
    assert_eq!(app.world().get::<Extinguisher>(ext).unwrap().water, MAX_WATER);
    assert_eq!(app.world().get::<WaterTank>(tank).unwrap().water, 1000 - 10);
}

#[test]
fn an_empty_extinguisher_says_so() {
    let mut app = setup();
    let (mob, ext) = armed(&mut app);
    app.world_mut().get_mut::<Extinguisher>(ext).unwrap().water = 0;
    assert_eq!(
        act(&mut app, mob, Action::UseOnTile(FIRE)),
        ["extinguisher-empty"]
    );
}

#[test]
fn the_cabinet_gives_up_its_extinguisher_and_takes_it_back() {
    let mut app = setup();
    let cabinet = one::<Cabinet>(&mut app);
    let state = |app: &App| app.world().get::<Appearance>(cabinet).unwrap().state.clone();
    assert_eq!(state(&app), "extinguisher_closed");
    let ext = app.world().get::<Cabinet>(cabinet).unwrap().held.unwrap();
    assert!(app.world().get::<InCabinet>(ext).is_some());
    assert!(app.world().get::<GridPos>(ext).is_none());

    let mob = spawn_player_mob(app.world_mut(), PLAYER);
    assert_eq!(act(&mut app, mob, Action::UseOn(cabinet)), ["extinguisher-taken"]);
    assert_eq!(app.world().get::<Hands>(mob).unwrap().active_item(), Some(ext));
    assert_eq!(state(&app), "extinguisher_empty");

    // An empty hand closes and opens the empty cabinet.
    act(&mut app, mob, Action::SwapHands);
    act(&mut app, mob, Action::UseOn(cabinet));
    assert_eq!(state(&app), "extinguisher_closed");
    act(&mut app, mob, Action::UseOn(cabinet));
    assert_eq!(state(&app), "extinguisher_empty");
    act(&mut app, mob, Action::SwapHands);

    assert_eq!(
        act(&mut app, mob, Action::UseOn(cabinet)),
        ["extinguisher-placed"]
    );
    assert_eq!(state(&app), "extinguisher_full");
    assert_eq!(app.world().get::<Hands>(mob).unwrap().active_item(), None);
    assert_eq!(app.world().get::<Cabinet>(cabinet).unwrap().held, Some(ext));
}

#[test]
fn a_fire_closet_holds_extinguishers_and_emergency_toolboxes() {
    let mut app = setup();
    let closet = spawn_object(app.world_mut(), &ObjectSpec::new("closet-fire"), PLAYER).unwrap();
    let contents = app.world().get::<Closet>(closet).unwrap().contents.clone();
    let mut names: Vec<String> = contents
        .iter()
        .map(|e| app.world().get::<Appearance>(*e).unwrap().sprite.clone())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "fire-extinguisher",
            "fire-extinguisher",
            "toolbox-emergency",
            "toolbox-emergency"
        ]
    );
}
