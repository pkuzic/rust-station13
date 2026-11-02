//! Monkeys. Spec: docs/reference/monkeys.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::combat::CombatIntent;
use rs13_classic::ghosts::Departures;
use rs13_classic::health::{Body, Vitals};
use rs13_classic::machines::MACHINE_TICKS;
use rs13_classic::monkeys::Monkey;
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_world::{Appearance, DisplayName, GridPos, Intent, MobStat, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'm': (tile: (id: "floor"), objects: ["monkey"]),
    },
    rows: [
        "WWWWWWWW",
        "W......W",
        "W..m...W",
        "W......W",
        "WWWWWWWW",
    ],
)"#;

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mut q = app.world_mut().query_filtered::<Entity, With<Monkey>>();
    let monkey = q.single(app.world()).unwrap();
    app.world_mut().run_schedule(FixedUpdate);
    (app, monkey)
}

fn cycles(app: &mut App, n: u64) {
    for _ in 0..n * MACHINE_TICKS {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn at(app: &App, monkey: Entity) -> TilePos {
    app.world().get::<GridPos>(monkey).unwrap().0
}

#[test]
fn a_monkey_wanders_inside_the_room_one_step_at_a_time() {
    let (mut app, monkey) = setup();
    let start = at(&app, monkey);
    let mut seen = vec![start];
    let mut last = start;
    for _ in 0..200 {
        cycles(&mut app, 1);
        let now = at(&app, monkey);
        assert!(
            (now.x - last.x).abs() + (now.y - last.y).abs() <= 1,
            "one step: {last:?} to {now:?}"
        );
        assert!(now.x >= 1 && now.x <= 6 && now.y >= 1 && now.y <= 3, "{now:?}");
        last = now;
        seen.push(now);
    }
    seen.sort_by_key(|p| (p.x, p.y));
    seen.dedup();
    assert!(seen.len() > 3, "it gets about: {seen:?}");
}

#[test]
fn a_dead_monkey_lies_still_and_leaves_no_ghost() {
    let (mut app, monkey) = setup();
    app.world_mut().get_mut::<Body>(monkey).unwrap().oxy = 500.0;
    cycles(&mut app, 1);
    assert_eq!(app.world().get::<Vitals>(monkey).unwrap().stat, MobStat::Dead);
    assert_eq!(app.world().get::<Appearance>(monkey).unwrap().state, "dead");
    let still = at(&app, monkey);
    cycles(&mut app, 30);
    assert_eq!(at(&app, monkey), still);
    assert!(app.world().resource::<Departures>().0.is_empty());
}

#[test]
fn people_can_hit_a_monkey_and_it_doesnt_pile_up_messages() {
    let (mut app, monkey) = setup();
    assert_eq!(app.world().get::<Appearance>(monkey).unwrap().sprite, "monkey");
    assert!(
        app.world()
            .get::<DisplayName>(monkey)
            .unwrap()
            .0
            .starts_with("monkey")
    );
    let spot = at(&app, monkey);
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(spot.x + 1, spot.y, 0));
    app.world_mut().get_mut::<CombatIntent>(mob).unwrap().0 = Intent::Harm;
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(monkey));
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(FixedUpdate);
    assert!(
        !app.world().get::<Outbox>(mob).unwrap().0.is_empty(),
        "they're told how it went"
    );
    cycles(&mut app, 1);
    assert!(app.world().get::<Outbox>(monkey).unwrap().0.is_empty());
}
