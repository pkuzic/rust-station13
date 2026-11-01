//! Wall-mounted sparkers. Spec: docs/reference/igniters.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::atmos::StationAtmos;
use rs13_classic::fires::Fire;
use rs13_classic::igniters::{IgnitionSwitch, Sparker};
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_simulation::atmos::gas::T20C;
use rs13_simulation::atmos::{Gas, GasMix};
use rs13_world::{Appearance, LightSource, TileMap, TilePos};

/// A closed 3 × 3 burn chamber with a sparker in the middle, and a room west of it where
/// the switch is.
const CHAMBER_MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'x': (tile: (id: "floor"), objects: [(id: "ignition-switch", vars: {"id": "burn"})]),
        'k': (tile: (id: "floor"), objects: [(id: "sparker", vars: {"id": "burn"})]),
    },
    rows: [
        "WWWWWWWW",
        "W.xW...W",
        "W..W.k.W",
        "W..W...W",
        "WWWWWWWW",
    ],
)"#;

const CHAMBER: TilePos = TilePos::new(5, 2, 0);
const BY_SWITCH: TilePos = TilePos::new(1, 3, 0);

/// A sparker beside a screwdriver lying where the player stands.
const BENCH_MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'v': (tile: (id: "floor"), objects: ["screwdriver"]),
        'k': (tile: (id: "floor"), objects: [(id: "sparker", vars: {"id": "burn"})]),
    },
    rows: [
        "WWWWW",
        "WvkWW",
        "WWWWW",
    ],
)"#;

fn app_with(map: &str, player: TilePos) -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(map, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    spawn_player_mob(app.world_mut(), player);
    seconds(&mut app, 2);
    app
}

fn seconds(app: &mut App, s: u64) {
    for _ in 0..s * 20 {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn one<C: Component>(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<C>>()
        .single(app.world())
        .unwrap()
}

fn find(app: &mut App, sprite: &str) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &Appearance)>();
    q.iter(app.world()).find(|(_, a)| a.sprite == sprite).unwrap().0
}

fn fires(app: &mut App) -> usize {
    app.world_mut().query::<&Fire>().iter(app.world()).count()
}

fn look(app: &mut App) -> String {
    let e = one::<Sparker>(app);
    app.world().get::<Appearance>(e).unwrap().state.clone()
}

/// The player clicks `target`, and what they are told.
fn click(app: &mut App, target: Entity) -> Vec<String> {
    let mob = one::<PendingActions>(app);
    let before = app.world().get::<Outbox>(mob).unwrap().0.len();
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    app.world_mut().run_schedule(FixedUpdate);
    app.world().get::<Outbox>(mob).unwrap().0[before..]
        .iter()
        .map(|m| m.id.clone())
        .collect()
}

#[test]
fn one_press_throws_one_spark_and_the_chamber_burns() {
    let mut app = app_with(CHAMBER_MAP, BY_SWITCH);
    let mut gas = GasMix::default();
    gas.temperature = T20C;
    gas.set(Gas::Plasma, 40.0);
    app.world_mut()
        .resource_mut::<StationAtmos>()
        .0
        .add_at(CHAMBER, &gas);
    seconds(&mut app, 10);
    assert_eq!(fires(&mut app), 0);
    let sparker = one::<Sparker>(&mut app);
    assert!(
        app.world().get::<LightSource>(sparker).is_some(),
        "a powered sparker has a pilot light"
    );

    let switch = one::<IgnitionSwitch>(&mut app);
    click(&mut app, switch);
    assert_eq!(look(&mut app), "migniter-spark");
    let most = (0..60)
        .map(|_| {
            seconds(&mut app, 1);
            fires(&mut app)
        })
        .max();
    assert_eq!(most, Some(9), "fire on every tile of the chamber");
    assert_eq!(look(&mut app), "migniter", "the spark is over");
}

#[test]
fn a_sparker_waits_five_seconds_between_sparks() {
    let mut app = app_with(CHAMBER_MAP, BY_SWITCH);
    let switch = one::<IgnitionSwitch>(&mut app);
    click(&mut app, switch);
    let sparker = one::<Sparker>(&mut app);
    let first = app.world().get::<Sparker>(sparker).unwrap().last_spark;
    assert!(first.is_some());
    // The switch is back up after five seconds and the sparker ready again just after.
    seconds(&mut app, 5);
    click(&mut app, switch);
    let second = app.world().get::<Sparker>(sparker).unwrap().last_spark;
    assert!(second > first, "{first:?} then {second:?}");
}

#[test]
fn a_screwdriver_disconnects_a_sparker_and_puts_its_light_out() {
    let mut app = app_with(BENCH_MAP, TilePos::new(1, 1, 0));
    let screwdriver = find(&mut app, "screwdriver");
    click(&mut app, screwdriver);
    let sparker = one::<Sparker>(&mut app);
    assert_eq!(click(&mut app, sparker), ["sparker-disabled-self"]);
    app.world_mut().run_schedule(FixedUpdate);
    assert!(app.world().get::<Sparker>(sparker).unwrap().disabled);
    assert_eq!(look(&mut app), "migniter-d");
    assert!(app.world().get::<LightSource>(sparker).is_none());

    assert_eq!(click(&mut app, sparker), ["sparker-reconnected-self"]);
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(look(&mut app), "migniter");
    assert!(app.world().get::<LightSource>(sparker).is_some());
}
