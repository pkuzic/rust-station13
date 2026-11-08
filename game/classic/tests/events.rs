//! Random events. Spec: docs/reference/events.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::Outbox;
use rs13_classic::events::{self, EventKind, RandomEvents, Wormhole};
use rs13_classic::genetics::Radiation;
use rs13_classic::lights::{Light, LightStatus};
use rs13_classic::machines::{Canister, Switch};
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_classic::round::{Phase, Round};
use rs13_world::{GridPos, HumanLook, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'L': (tile: (id: "floor"), objects: [(id: "light", dir: North)]),
        'c': (tile: (id: "floor"), objects: ["canister"]),
        'p': (tile: (id: "floor"), objects: ["plasma-canister"]),
    },
    rows: [
        "WWWWWWWWWWWW",
        "WLLLLLLLLLLW",
        "W..........W",
        "W....cp....W",
        "W..........W",
        "WWWWWWWWWWWW",
    ],
)"#;

fn setup(playing: bool) -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    if playing {
        app.world_mut().insert_resource(Round {
            phase: Phase::Playing { mode: "extended" },
            forced_mode: None,
        });
    }
    tick(&mut app, 1);
    app
}

fn tick(app: &mut App, n: u32) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn wait_s(app: &mut App, s: f32) {
    tick(app, (s * rs13_core::TICK_RATE_HZ as f32) as u32 + 2);
}

fn person(app: &mut App, x: i32, y: i32) -> Entity {
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(x, y, 0));
    app.world_mut().entity_mut(mob).insert(HumanLook::default());
    mob
}

fn told(app: &App, mob: Entity, text: &str) -> bool {
    app.world()
        .get::<Outbox>(mob)
        .unwrap()
        .0
        .iter()
        .any(|m| m.id == "command-alert" && format!("{m:?}").contains(text))
}

fn active(app: &App) -> Option<EventKind> {
    app.world().resource::<RandomEvents>().active.map(|a| a.kind)
}

#[test]
fn an_event_comes_every_thousand_seconds_of_a_round_unless_turned_off() {
    let mut app = setup(false);
    assert_eq!(
        app.world().resource::<RandomEvents>().next_at,
        None,
        "not before a round"
    );
    let mut app2 = setup(true);
    let next = app2.world().resource::<RandomEvents>().next_at.unwrap();
    let now = app2
        .world()
        .resource::<rs13_classic::time::WorldTime>()
        .deciseconds();
    assert!((next - now - events::EVERY_DS).abs() < 2.0, "{next} {now}");
    // Due now.
    app2.world_mut().resource_mut::<RandomEvents>().next_at = Some(0.0);
    tick(&mut app2, 1);
    let started = app2.world().resource::<RandomEvents>();
    assert!(started.next_at.unwrap() > now + events::EVERY_DS - 10.0);
    // Some events are over at once (the portal storm, the epidemic), so whether one is still
    // running depends on the pick; a turned-off switch means none at all.
    tick(&mut app, 1);
    app.world_mut().insert_resource(Round {
        phase: Phase::Playing { mode: "extended" },
        forced_mode: None,
    });
    app.world_mut().resource_mut::<RandomEvents>().on = false;
    tick(&mut app, 1);
    app.world_mut().resource_mut::<RandomEvents>().next_at = Some(0.0);
    tick(&mut app, 1);
    assert_eq!(active(&app), None);
}

#[test]
fn an_electrical_storm_blows_lights_and_is_announced_both_ways() {
    let mut app = setup(true);
    let mob = person(&mut app, 2, 2);
    assert!(events::start(app.world_mut(), EventKind::ElectricalStorm));
    assert!(
        !events::start(app.world_mut(), EventKind::RadiationBelt),
        "one at a time"
    );
    tick(&mut app, 1);
    assert!(told(&app, mob, "event-storm-start"));
    let lifetime = app.world().resource::<RandomEvents>().active.unwrap().lifetime;
    assert!((30..=120).contains(&lifetime));
    wait_s(&mut app, 30.0);
    let broken = {
        let world = app.world_mut();
        let mut q = world.query::<&Light>();
        q.iter(world).filter(|l| l.status == LightStatus::Broken).count()
    };
    assert!(broken > 0, "a light a second, half the time, twice over");
    wait_s(&mut app, lifetime as f32 - 28.0);
    assert_eq!(active(&app), None);
    assert!(told(&app, mob, "event-storm-end"));
}

#[test]
fn a_radiation_belt_irradiates_everyone_for_ten_seconds() {
    let mut app = setup(true);
    let mob = person(&mut app, 2, 2);
    events::start(app.world_mut(), EventKind::RadiationBelt);
    wait_s(&mut app, 3.0);
    let rads = app.world().get::<Radiation>(mob).map_or(0.0, |r| r.0);
    assert!(rads >= 1.0, "{rads}");
    wait_s(&mut app, 10.0);
    assert_eq!(active(&app), None);
    assert!(told(&app, mob, "event-radiation-end"));
}

#[test]
fn a_gas_leak_opens_air_canisters_but_never_plasma() {
    let mut app = setup(true);
    events::start(app.world_mut(), EventKind::GasLeak);
    tick(&mut app, 1);
    let world = app.world_mut();
    let mut q = world.query::<(&rs13_world::Appearance, &Switch, &Canister)>();
    for (look, switch, canister) in q.iter(world) {
        if look.sprite == "canister" {
            assert!(switch.0, "the air canister leaks");
            assert!(canister.release_target > 101.0);
        } else {
            assert!(!switch.0, "{} stays shut", look.sprite);
        }
    }
}

#[test]
fn a_portal_storm_opens_wormholes_that_throw_people_across_the_ship() {
    let mut app = setup(true);
    events::start(app.world_mut(), EventKind::PortalStorm);
    tick(&mut app, 1);
    assert_eq!(
        active(&app),
        None,
        "over at once: the wormholes come by themselves"
    );
    // Open one now, on a known tile, going to a known place.
    let at = TilePos::new(8, 2, 0);
    app.world_mut().resource_mut::<RandomEvents>().wormholes = vec![(at, 0.0)];
    tick(&mut app, 1);
    let hole = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &GridPos, &mut Wormhole)>();
        let (hole, _, mut w) = q.iter_mut(world).find(|(_, p, _)| p.0 == at).unwrap();
        w.target = TilePos::new(2, 4, 0);
        hole
    };
    let mob = person(&mut app, 7, 2);
    app.world_mut().entity_mut(mob).insert(GridPos(at));
    tick(&mut app, 2);
    let now = app.world().get::<GridPos>(mob).unwrap().0;
    assert!((now.x - 2).abs() <= 1 && (now.y - 4).abs() <= 1, "{now:?}");
    wait_s(&mut app, events::WORMHOLE_DS / 10.0);
    assert!(app.world().get_entity(hole).is_err(), "gone after 30 s");
}
