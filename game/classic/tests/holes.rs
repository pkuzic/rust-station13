//! Hearing through holes in the floor. Spec: docs/reference/multiz.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::Outbox;
use rs13_classic::objects::{spawn_player_mob, spawn_station};
use rs13_classic::sound::{Earshot, SoundEvents};
use rs13_classic::speech::PendingSpeech;
use rs13_world::{TileMap, TilePos};

/// Downstairs: a long room with a wall sticking up from the south at x = 7.
const LOWER: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
    },
    rows: [
        "WWWWWWWWWWWWWWWWWWWW",
        "W..................W",
        "W......W...........W",
        "W......W...........W",
        "WWWWWWWWWWWWWWWWWWWW",
    ],
)"#;

/// Upstairs: the same room, with a hole at (5, 2).
const UPPER: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'o': (tile: (id: "open-space")),
    },
    rows: [
        "WWWWWWWWWWWWWWWWWWWW",
        "W..................W",
        "W....o.............W",
        "W..................W",
        "WWWWWWWWWWWWWWWWWWWW",
    ],
)"#;

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let decks = vec![
        TileMap::from_ron(LOWER, 0).unwrap(),
        TileMap::from_ron(UPPER, 1).unwrap(),
    ];
    let problems = spawn_station(app.world_mut(), decks);
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().run_schedule(FixedUpdate);
    app
}

fn mob(app: &mut App, x: i32, y: i32, z: i32) -> Entity {
    spawn_player_mob(app.world_mut(), TilePos::new(x, y, z))
}

fn say(app: &mut App, who: Entity) {
    app.world_mut()
        .get_mut::<PendingSpeech>(who)
        .unwrap()
        .0
        .push("hello".to_owned());
    app.world_mut().run_schedule(FixedUpdate);
}

fn heard(app: &App, who: Entity) -> bool {
    app.world()
        .get::<Outbox>(who)
        .unwrap()
        .0
        .iter()
        .any(|m| m.id == "say")
}

fn sound_at(app: &mut App, at: TilePos) {
    app.world_mut()
        .resource_mut::<SoundEvents>()
        .play("airlock", at, 100.0);
    app.world_mut().run_schedule(FixedUpdate);
}

#[test]
fn speech_goes_down_a_hole_and_up_it() {
    let mut app = setup();
    let above = mob(&mut app, 5, 2, 1);
    let below = mob(&mut app, 5, 2, 0);
    say(&mut app, above);
    assert!(heard(&app, below), "down");

    let mut app = setup();
    let above = mob(&mut app, 5, 2, 1);
    let below = mob(&mut app, 5, 2, 0);
    say(&mut app, below);
    assert!(heard(&app, above), "up");
}

#[test]
fn speech_carries_from_a_little_way_off_the_hole() {
    let mut app = setup();
    // Two along upstairs, one through, one along downstairs: 4 of 7.
    let above = mob(&mut app, 3, 2, 1);
    let below = mob(&mut app, 4, 2, 0);
    say(&mut app, above);
    assert!(heard(&app, below));
}

#[test]
fn speech_doesnt_carry_past_its_range() {
    let mut app = setup();
    // Four along upstairs, one through, three along downstairs: 8 of 7, though each is near enough the hole.
    let above = mob(&mut app, 9, 1, 1);
    let below = mob(&mut app, 2, 2, 0);
    say(&mut app, above);
    assert!(!heard(&app, below));
}

#[test]
fn a_wall_downstairs_stops_speech_from_the_hole() {
    let mut app = setup();
    // Under the hole is (5, 2); (9, 2) is behind the wall at x = 7 but in range.
    let above = mob(&mut app, 5, 2, 1);
    let below = mob(&mut app, 9, 2, 0);
    say(&mut app, above);
    assert!(!heard(&app, below));
}

#[test]
fn a_floor_with_no_hole_stops_speech_and_sound() {
    let mut app = setup();
    let above = mob(&mut app, 15, 2, 1);
    let below = mob(&mut app, 15, 2, 0);
    say(&mut app, above);
    assert!(!heard(&app, below));
    sound_at(&mut app, TilePos::new(15, 2, 1));
    assert!(app.world().get::<Earshot>(below).unwrap().0.is_empty());
}

#[test]
fn a_sound_below_is_heard_through_the_hole_and_seems_to_come_from_it() {
    let mut app = setup();
    let above = mob(&mut app, 3, 2, 1);
    sound_at(&mut app, TilePos::new(4, 1, 0));
    let got = app
        .world()
        .get::<Earshot>(above)
        .unwrap()
        .0
        .last()
        .cloned()
        .expect("heard");
    assert_eq!((got.at.x, got.at.y, got.at.z), (5, 2, 1), "from the hole");
    assert!(got.volume < 100.0, "quieter for the way there");
}

#[test]
fn a_sound_further_from_the_hole_is_quieter() {
    let volume = |x: i32| {
        let mut app = setup();
        let above = mob(&mut app, 5, 2, 1);
        sound_at(&mut app, TilePos::new(x, 2, 0));
        app.world()
            .get::<Earshot>(above)
            .unwrap()
            .0
            .last()
            .map(|h| h.volume)
    };
    let near = volume(5).expect("right under");
    let far = volume(3).expect("a bit away");
    assert!(far < near, "{far} < {near}");
}
