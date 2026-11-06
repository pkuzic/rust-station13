//! The AutoTraitor round (Luna's Extend-A-Traitormongous). Spec: docs/reference/autotraitor.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::Outbox;
use rs13_classic::autotraitor::{
    AutoTraitor, late_join, later_odds, run_autotraitor, setup_round, start_odds,
};
use rs13_classic::jobs::HasJob;
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_classic::traitor::{Traitor, WantsTraitor};
use rs13_world::{DisplayName, TileMap, TilePos};

const MAP: &str = r#"(
    legend: { 'W': (tile: (id: "wall", solid: true)), '.': (tile: (id: "floor")) },
    rows: ["WWWWWWW", "W.....W", "W.....W", "WWWWWWW"],
)"#;

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().run_schedule(FixedUpdate);
    app
}

fn crew(app: &mut App, n: usize, wants: bool) -> Vec<Entity> {
    (0..n)
        .map(|i| {
            let mob = spawn_player_mob(app.world_mut(), TilePos::new(1 + (i % 5) as i32, 1, 0));
            let mut e = app.world_mut().entity_mut(mob);
            e.insert((HasJob("clown".to_owned()), DisplayName(format!("Crew {i}"))));
            if wants {
                e.insert(WantsTraitor);
            }
            mob
        })
        .collect()
}

fn traitors(app: &mut App) -> usize {
    let world = app.world_mut();
    world.query::<&Traitor>().iter(world).count()
}

fn heard(app: &App, mob: Entity, id: &str) -> bool {
    app.world()
        .get::<Outbox>(mob)
        .unwrap()
        .0
        .iter()
        .any(|m| m.id == id)
}

#[test]
fn the_odds_are_a_tenth_and_one_and_more_while_short() {
    assert_eq!(start_odds(5), (1, 50.0));
    assert_eq!(start_odds(10), (2, 0.0));
    assert_eq!(start_odds(15), (2, 50.0));
    assert_eq!(later_odds(15, 0), (2, 75.0));
    assert_eq!(later_odds(15, 1), (2, 25.0));
    assert_eq!(later_odds(25, 1), (3, 75.0));
}

#[test]
fn at_the_start_there_are_a_tenth_of_them_and_one() {
    let mut app = setup();
    crew(&mut app, 10, true);
    // Ten players: at most two, and no chance of the second.
    setup_round(app.world_mut());
    assert_eq!(traitors(&mut app), 1);
    assert!(app.world().get_resource::<AutoTraitor>().is_some());
}

#[test]
fn later_on_one_who_wants_it_is_made_a_traitor_but_never_more_than_the_most() {
    let mut app = setup();
    // Nineteen players: two at most, and 95% each look while there are none.
    let mobs = crew(&mut app, 19, true);
    app.world_mut().insert_resource(AutoTraitor { check_at: 0.0 });
    for _ in 0..100 {
        app.world_mut().resource_mut::<AutoTraitor>().check_at = 0.0;
        run_autotraitor(app.world_mut());
        assert!(traitors(&mut app) <= 2);
    }
    assert_eq!(traitors(&mut app), 2);
    let made: Vec<Entity> = mobs
        .into_iter()
        .filter(|m| app.world().get::<Traitor>(*m).is_some())
        .collect();
    for mob in made {
        assert!(heard(&app, mob, "traitor-debt"));
        assert!(heard(&app, mob, "traitor-now"));
        assert!(!app.world().get::<Traitor>(mob).unwrap().objectives.is_empty());
    }
}

#[test]
fn no_one_who_doesnt_want_it_is_made_one_later() {
    let mut app = setup();
    crew(&mut app, 19, false);
    app.world_mut().insert_resource(AutoTraitor { check_at: 0.0 });
    for _ in 0..50 {
        app.world_mut().resource_mut::<AutoTraitor>().check_at = 0.0;
        run_autotraitor(app.world_mut());
    }
    assert_eq!(traitors(&mut app), 0);
}

#[test]
fn someone_joining_late_may_be_made_one_only_in_this_round() {
    let mut app = setup();
    crew(&mut app, 18, false);
    let late = crew(&mut app, 1, true)[0];
    late_join(app.world_mut(), late);
    assert!(app.world().get::<Traitor>(late).is_none(), "not in another round");

    app.world_mut()
        .insert_resource(AutoTraitor { check_at: f32::MAX });
    let mut made = false;
    for _ in 0..30 {
        late_join(app.world_mut(), late);
        if app.world().get::<Traitor>(late).is_some() {
            made = true;
            break;
        }
    }
    assert!(made, "95% each time");
    assert!(heard(&app, late, "traitor-you"));
}
