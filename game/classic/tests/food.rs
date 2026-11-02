//! Snacks, drinks and hunger. Spec: docs/reference/food.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::environment::LIFE_TICKS;
use rs13_classic::food::{Drink, HUNGER_PER_CYCLE, Nutrition, STARTING_NUTRITION, Snack};
use rs13_classic::hands::Hands;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_world::{Appearance, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
    },
    rows: [
        "WWWWW",
        "W...W",
        "W...W",
        "WWWWW",
    ],
)"#;

const HERE: TilePos = TilePos::new(1, 1, 0);

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().run_schedule(FixedUpdate);
    app
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

/// A mob holding a fresh `proto`, with its nutrition set.
fn holding(app: &mut App, proto: &str, nutrition: f32) -> (Entity, Entity) {
    let mob = spawn_player_mob(app.world_mut(), HERE);
    app.world_mut().get_mut::<Nutrition>(mob).unwrap().level = nutrition;
    let item = spawn_object(app.world_mut(), &ObjectSpec::new(proto), HERE).unwrap();
    act(app, mob, Action::UseOn(item));
    assert_eq!(app.world().get::<Hands>(mob).unwrap().active_item(), Some(item));
    (mob, item)
}

fn nutrition(app: &App, mob: Entity) -> f32 {
    app.world().get::<Nutrition>(mob).unwrap().level
}

#[test]
fn a_candy_is_eaten_in_six_bites_and_leaves_its_wrapper_in_hand() {
    let mut app = setup();
    let (mob, candy) = holding(&mut app, "candy", 300.0);
    assert_eq!(app.world().get::<Snack>(candy).unwrap().bites_left, 6);
    let said = act(&mut app, mob, Action::UseOn(mob));
    assert_eq!(said, ["food-eat-begin"], "a fullness of 150");
    let after = nutrition(&app, mob);
    assert!(after > 300.0, "fed: {after}");
    for _ in 0..5 {
        act(&mut app, mob, Action::UseOn(mob));
    }
    app.world_mut().run_schedule(FixedUpdate);
    assert!(app.world().get_entity(candy).is_err(), "eaten up");
    let held = app
        .world()
        .get::<Hands>(mob)
        .unwrap()
        .active_item()
        .expect("its wrapper");
    assert_eq!(app.world().get::<Appearance>(held).unwrap().sprite, "trash-candy");
    // Luna's candy: 9 nutriment and 3 sugar.
    let total = nutrition(&app, mob) - 300.0;
    assert!((total - 142.5).abs() < 5.0, "{total}");
}

#[test]
fn how_full_you_are_decides_the_message_and_too_full_eats_nothing() {
    let mut app = setup();
    let (mob, _) = holding(&mut app, "chips", 100.0);
    assert_eq!(act(&mut app, mob, Action::UseOn(mob)), ["food-eat-gobble"]);
    let (mob, _) = holding(&mut app, "chips", 800.0);
    assert_eq!(act(&mut app, mob, Action::UseOn(mob)), ["food-eat"]);
    let (mob, _) = holding(&mut app, "chips", STARTING_NUTRITION);
    assert_eq!(act(&mut app, mob, Action::UseOn(mob)), ["food-eat-unwilling"]);
    let (mob, chips) = holding(&mut app, "chips", 1800.0);
    assert_eq!(act(&mut app, mob, Action::UseOn(mob)), ["food-eat-full"]);
    assert_eq!(
        app.world().get::<Snack>(chips).unwrap().bites_left,
        11,
        "nothing eaten"
    );
}

#[test]
fn a_can_of_cola_goes_in_gulps_and_stays_when_empty() {
    let mut app = setup();
    let (mob, cola) = holding(&mut app, "cola", 500.0);
    let mut gulps = 0;
    while app.world().get::<Drink>(cola).unwrap().units > 0.0 {
        assert_eq!(act(&mut app, mob, Action::UseOn(mob)), ["drink-gulp"]);
        gulps += 1;
        assert!(gulps < 10);
    }
    // 30 units: a gulp of 6, four of 5, and the last 4.
    assert_eq!(gulps, 6);
    assert_eq!(act(&mut app, mob, Action::UseOn(mob)), ["drink-empty"]);
    assert_eq!(app.world().get::<Hands>(mob).unwrap().active_item(), Some(cola));
    assert!(nutrition(&app, mob) > 500.0);
}

#[test]
fn hunger_creeps_up_and_at_nothing_the_stomach_rumbles() {
    let mut app = setup();
    let mob = spawn_player_mob(app.world_mut(), HERE);
    let start = nutrition(&app, mob);
    for _ in 0..10 * LIFE_TICKS {
        app.world_mut().run_schedule(FixedUpdate);
    }
    let lost = start - nutrition(&app, mob);
    assert!(
        (lost - 10.0 * HUNGER_PER_CYCLE).abs() <= HUNGER_PER_CYCLE,
        "{lost}"
    );

    app.world_mut().get_mut::<Nutrition>(mob).unwrap().level = 1.0;
    for _ in 0..2 * LIFE_TICKS {
        app.world_mut().run_schedule(FixedUpdate);
    }
    let pangs: Vec<String> = app
        .world()
        .get::<Outbox>(mob)
        .unwrap()
        .0
        .iter()
        .map(|m| m.id.clone())
        .filter(|id| id.starts_with("hunger-"))
        .collect();
    assert_eq!(pangs.len(), 1, "one pang, not one a cycle: {pangs:?}");
}
