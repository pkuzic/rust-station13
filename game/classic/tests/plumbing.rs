//! Sinks, showers, toilets and mirrors. Spec: docs/reference/plumbing.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::chemistry::Reagents;
use rs13_classic::environment::Lungs;
use rs13_classic::grabs::{self, Grab, Hold};
use rs13_classic::hands::{Hands, Item};
use rs13_classic::health::{Body, Zone};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::plumbing::{self, Mirror, Shower, WaterTemp};
use rs13_world::{Appearance, GridPos, HumanLook, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
    },
    rows: [
        "WWWWWWWW",
        "W......W",
        "W......W",
        "W......W",
        "WWWWWWWW",
    ],
)"#;

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

fn wait_ds(app: &mut App, ds: f32) {
    tick(app, (ds * rs13_core::TICK_RATE_HZ as f32 / 10.0) as u32 + 2);
}

fn at(x: i32, y: i32) -> TilePos {
    TilePos::new(x, y, 0)
}

fn person(app: &mut App, x: i32, y: i32) -> Entity {
    let mob = spawn_player_mob(app.world_mut(), at(x, y));
    app.world_mut().entity_mut(mob).insert(HumanLook::default());
    mob
}

fn thing(app: &mut App, proto: &str, x: i32, y: i32) -> Entity {
    spawn_object(app.world_mut(), &ObjectSpec::new(proto), at(x, y)).unwrap()
}

fn act(app: &mut App, mob: Entity, action: Action) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
    tick(app, 1);
}

fn hold(app: &mut App, mob: Entity, proto: &str) -> Entity {
    let pos = app.world().get::<GridPos>(mob).unwrap().0;
    let item = spawn_object(app.world_mut(), &ObjectSpec::new(proto), pos).unwrap();
    act(app, mob, Action::UseOn(item));
    assert_eq!(app.world().get::<Hands>(mob).unwrap().active_item(), Some(item));
    item
}

/// Brute and burn over the whole body.
fn damage(app: &App, mob: Entity) -> (f32, f32) {
    let body = app.world().get::<Body>(mob).unwrap();
    Zone::ALL.iter().fold((0.0, 0.0), |(b, f), z| {
        (b + body.organ(*z).brute, f + body.organ(*z).burn)
    })
}

fn told(app: &App, mob: Entity, id: &str) -> bool {
    app.world()
        .get::<Outbox>(mob)
        .unwrap()
        .0
        .iter()
        .any(|m| m.id == id)
}

#[test]
fn a_sink_fills_glassware_and_buckets_with_water() {
    let mut app = setup();
    let sink = thing(&mut app, "sink", 3, 2);
    let mob = person(&mut app, 2, 2);
    let beaker = hold(&mut app, mob, "beaker");
    act(&mut app, mob, Action::UseOn(sink));
    assert_eq!(app.world().get::<Reagents>(beaker).unwrap().get("water"), 10.0);
    assert!(told(&app, mob, "sink-fills-self"));
    act(&mut app, mob, Action::Drop);
    let bucket = hold(&mut app, mob, "bucket");
    act(&mut app, mob, Action::UseOn(sink));
    assert_eq!(app.world().get::<Reagents>(bucket).unwrap().get("water"), 70.0);
}

#[test]
fn washing_takes_four_seconds_standing_still_and_one_at_a_time() {
    let mut app = setup();
    let sink = thing(&mut app, "sink", 3, 2);
    let washer = person(&mut app, 2, 2);
    let other = person(&mut app, 4, 2);
    let watcher = person(&mut app, 2, 3);
    act(&mut app, washer, Action::UseOn(sink));
    assert!(told(&app, washer, "sink-start-hands"));
    act(&mut app, other, Action::UseOn(sink));
    assert!(told(&app, other, "sink-busy"));
    wait_ds(&mut app, plumbing::WASH_DS);
    assert!(told(&app, watcher, "sink-washed-hands"));
    assert!(told(&app, washer, "sink-washed-hands"));

    // Walking off: nothing.
    app.world_mut().get_mut::<Outbox>(other).unwrap().0.clear();
    act(&mut app, other, Action::UseOn(sink));
    assert!(told(&app, other, "sink-start-hands"));
    app.world_mut().entity_mut(other).insert(GridPos(at(5, 3)));
    wait_ds(&mut app, plumbing::WASH_DS);
    assert!(!told(&app, other, "sink-washed-hands"));
    // And the sink is free again.
    act(&mut app, washer, Action::UseOn(sink));
    assert!(told(&app, washer, "sink-start-hands"));
}

#[test]
fn a_shower_runs_mists_up_and_its_valve_turns_hot_and_cold() {
    let mut app = setup();
    let shower = thing(&mut app, "shower", 3, 2);
    let mob = person(&mut app, 2, 2);
    let count = |app: &mut App, sprite: &str| {
        let world = app.world_mut();
        let mut q = world.query::<&Appearance>();
        q.iter(world).filter(|a| a.sprite == sprite).count()
    };
    act(&mut app, mob, Action::UseOn(shower));
    tick(&mut app, 1);
    assert!(app.world().get::<Shower>(shower).unwrap().on);
    assert_eq!(count(&mut app, plumbing::SHOWER_WATER), 1);
    assert_eq!(count(&mut app, plumbing::MIST), 0, "not yet");
    wait_ds(&mut app, plumbing::MIST_AFTER_DS);
    assert_eq!(count(&mut app, plumbing::MIST), 1);

    // A wrench turns it to freezing: no mist.
    let wrench = hold(&mut app, mob, "wrench");
    act(&mut app, mob, Action::UseOn(shower));
    assert!(told(&app, mob, "shower-valve-start"));
    wait_ds(&mut app, plumbing::VALVE_DS);
    assert_eq!(
        app.world().get::<Shower>(shower).unwrap().temp,
        WaterTemp::Freezing
    );
    assert!(told(&app, mob, "shower-adjusted-self"));
    assert_eq!(count(&mut app, plumbing::MIST), 0);

    // Stepping under freezing water chills at once.
    let before = app.world().get::<Lungs>(mob).unwrap().body_temperature;
    app.world_mut().entity_mut(mob).insert(GridPos(at(3, 2)));
    tick(&mut app, 1);
    let after = app.world().get::<Lungs>(mob).unwrap().body_temperature;
    assert!(after <= before - 79.0, "{before} -> {after}");
    assert!(told(&app, mob, "shower-freezing"));

    // Then boiling: it burns.
    app.world_mut().entity_mut(mob).insert(GridPos(at(2, 2)));
    act(&mut app, mob, Action::UseOn(shower));
    wait_ds(&mut app, plumbing::VALVE_DS);
    assert_eq!(
        app.world().get::<Shower>(shower).unwrap().temp,
        WaterTemp::Boiling
    );
    app.world_mut().entity_mut(mob).insert(GridPos(at(3, 2)));
    tick(&mut app, 1);
    assert!(damage(&app, mob).1 > 0.0);
    assert!(told(&app, mob, "shower-searing"));
    let _ = wrench;

    // Off: the water stops at once, the mist lingers.
    app.world_mut().entity_mut(mob).insert(GridPos(at(2, 2)));
    act(&mut app, mob, Action::Drop);
    act(&mut app, mob, Action::UseOn(shower));
    tick(&mut app, 1);
    assert!(!app.world().get::<Shower>(shower).unwrap().on);
    assert_eq!(count(&mut app, plumbing::SHOWER_WATER), 0);
}

#[test]
fn an_air_analyzer_reads_the_water_temperature() {
    let mut app = setup();
    let shower = thing(&mut app, "shower", 3, 2);
    let mob = person(&mut app, 2, 2);
    hold(&mut app, mob, "atmos-analyzer");
    act(&mut app, mob, Action::UseOn(shower));
    assert!(told(&app, mob, "shower-reads"));
}

#[test]
fn a_mirror_shatters_at_twice_the_force_in_percent() {
    let mut app = setup();
    let mirror = thing(&mut app, "mirror", 3, 2);
    let mob = person(&mut app, 2, 2);
    let watcher = person(&mut app, 2, 3);
    let beaker = hold(&mut app, mob, "beaker");
    for _ in 0..10 {
        act(&mut app, mob, Action::UseOn(mirror));
    }
    assert!(
        !app.world().get::<Mirror>(mirror).unwrap().shattered,
        "force 0 never"
    );
    assert!(told(&app, watcher, "mirror-hit"));
    app.world_mut().get_mut::<Item>(beaker).unwrap().force = 50.0;
    act(&mut app, mob, Action::UseOn(mirror));
    assert!(app.world().get::<Mirror>(mirror).unwrap().shattered);
    assert!(told(&app, watcher, "mirror-smashed"));
    assert_eq!(app.world().get::<Appearance>(mirror).unwrap().state, "broken");
}

#[test]
fn a_tight_grab_slams_someone_on_a_toilet_into_it() {
    let mut app = setup();
    let toilet = thing(&mut app, "toilet", 3, 2);
    let bully = person(&mut app, 2, 2);
    let victim = person(&mut app, 3, 2);
    grabs::start(app.world_mut(), bully, victim);
    tick(&mut app, 1);
    let grab = app.world().get::<Hands>(bully).unwrap().active_item().unwrap();
    act(&mut app, bully, Action::UseOn(toilet));
    assert!(told(&app, bully, "toilet-tighter-grip"));
    let before = damage(&app, victim).0;
    app.world_mut().get_mut::<Grab>(grab).unwrap().hold = Hold::Aggressive;
    act(&mut app, bully, Action::UseOn(toilet));
    assert!(told(&app, bully, "toilet-slam-self"));
    let after = damage(&app, victim).0;
    assert!(
        (after - before - plumbing::SLAM_BRUTE).abs() < 0.01,
        "{before} -> {after}"
    );
}
