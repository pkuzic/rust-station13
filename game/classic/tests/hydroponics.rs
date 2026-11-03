//! Plant pots. Spec: docs/reference/hydroponics.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::chemistry::Reagents;
use rs13_classic::hands::Hands;
use rs13_classic::hydroponics::{GROWTH_PER_CYCLE, PLANTS, POT_WATER, PlantPot};
use rs13_classic::machines::MACHINE_TICKS;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_world::{Appearance, GridPos, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: { 'W': (tile: (id: "wall", solid: true)), '.': (tile: (id: "floor")) },
    rows: ["WWWWW", "W...W", "WWWWW"],
)"#;
const HERE: TilePos = TilePos::new(1, 1, 0);
const THERE: TilePos = TilePos::new(2, 1, 0);

fn setup(seed: &str) -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), HERE);
    let pot = spawn_object(app.world_mut(), &ObjectSpec::new("plant-pot"), THERE).unwrap();
    let seed = spawn_object(app.world_mut(), &ObjectSpec::new(seed), HERE).unwrap();
    app.world_mut().run_schedule(FixedUpdate);
    click(&mut app, mob, seed);
    click(&mut app, mob, pot);
    (app, mob, pot)
}

fn click(app: &mut App, mob: Entity, target: Entity) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    app.world_mut().run_schedule(FixedUpdate);
}

fn ticks(app: &mut App, n: u64) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn state(app: &App, pot: Entity) -> rs13_classic::hydroponics::Growing {
    app.world()
        .get::<PlantPot>(pot)
        .unwrap()
        .growing
        .expect("growing")
}

fn set_growth(app: &mut App, pot: Entity, growth: i32) {
    let mut p = app.world_mut().get_mut::<PlantPot>(pot).unwrap();
    p.growing.as_mut().unwrap().growth = growth;
}

fn crops(app: &mut App, sprite: &str) -> usize {
    let mut q = app.world_mut().query::<(&Appearance, &GridPos)>();
    q.iter(app.world()).filter(|(a, _)| a.sprite == sprite).count()
}

fn told(app: &App, mob: Entity) -> String {
    app.world()
        .get::<Outbox>(mob)
        .unwrap()
        .0
        .last()
        .unwrap()
        .id
        .clone()
}

#[test]
fn every_plants_seed_and_crop_exist() {
    let (mut app, ..) = setup("seed-tomato");
    for plant in &PLANTS {
        spawn_object(app.world_mut(), &ObjectSpec::new(plant.seed), HERE).expect(plant.seed);
        spawn_object(app.world_mut(), &ObjectSpec::new(plant.crop), HERE).expect(plant.crop);
    }
}

#[test]
fn a_seed_grows_with_water_and_changes_its_look() {
    let (mut app, mob, pot) = setup("seed-tomato");
    assert!(
        app.world().get::<Hands>(mob).unwrap().active_item().is_none(),
        "planted"
    );
    assert_eq!(state(&app, pot).growth, 0);
    ticks(&mut app, 3 * MACHINE_TICKS);
    assert_eq!(state(&app, pot).growth, 3 * GROWTH_PER_CYCLE);
    assert!(
        app.world().get::<Reagents>(pot).unwrap().get("water") < POT_WATER,
        "it drinks"
    );
    assert_eq!(
        app.world().get::<Appearance>(pot).unwrap().state,
        "pot-empty+tomato-G1"
    );
    let grown = PLANTS.iter().find(|p| p.id == "tomato").unwrap().grow;
    set_growth(&mut app, pot, grown);
    ticks(&mut app, MACHINE_TICKS);
    assert_eq!(
        app.world().get::<Appearance>(pot).unwrap().state,
        "pot-empty+tomato-G2"
    );
}

#[test]
fn watering_a_pot_doesnt_stop_it_growing_and_a_dry_one_stops() {
    let (mut app, _, pot) = setup("seed-tomato");
    app.world_mut()
        .get_mut::<Reagents>(pot)
        .unwrap()
        .add("water", 100.0);
    ticks(&mut app, MACHINE_TICKS);
    let grown = state(&app, pot).growth;
    assert_eq!(grown, GROWTH_PER_CYCLE);
    app.world_mut()
        .get_mut::<Reagents>(pot)
        .unwrap()
        .remove("water", 1000.0);
    ticks(&mut app, 3 * MACHINE_TICKS);
    assert_eq!(state(&app, pot).growth, grown);
}

#[test]
fn a_thirsty_plant_dies_and_can_be_cleared_away() {
    let (mut app, mob, pot) = setup("seed-carrot");
    app.world_mut()
        .get_mut::<Reagents>(pot)
        .unwrap()
        .remove("water", 1000.0);
    ticks(&mut app, 400 * MACHINE_TICKS);
    assert!(state(&app, pot).dead);
    assert_eq!(state(&app, pot).growth, 0);
    click(&mut app, mob, pot);
    assert_eq!(told(&app, mob), "pot-cleared");
    assert!(app.world().get::<PlantPot>(pot).unwrap().growing.is_none());
}

#[test]
fn a_ripe_plant_is_harvested_until_it_has_no_more_to_give() {
    let (mut app, mob, pot) = setup("seed-lemon");
    let plant = PLANTS.iter().find(|p| p.id == "lemon").unwrap();
    click(&mut app, mob, pot);
    assert_eq!(told(&app, mob), "pot-not-ready");

    for round in 1..=plant.harvests {
        set_growth(&mut app, pot, plant.ripe);
        click(&mut app, mob, pot);
        assert_eq!(crops(&mut app, "lemon"), (plant.crop_size * round) as usize);
        assert_eq!(told(&app, mob), "pot-harvested");
    }
    assert!(
        app.world().get::<PlantPot>(pot).unwrap().growing.is_none(),
        "all used up"
    );
    assert_eq!(app.world().get::<Appearance>(pot).unwrap().state, "pot-empty");
}

#[test]
fn a_full_pot_refuses_another_seed_and_a_beaker_pours_in() {
    let (mut app, mob, pot) = setup("seed-apple");
    let second = spawn_object(app.world_mut(), &ObjectSpec::new("seed-potato"), HERE).unwrap();
    app.world_mut().run_schedule(FixedUpdate);
    click(&mut app, mob, second);
    click(&mut app, mob, pot);
    assert_eq!(told(&app, mob), "pot-occupied");
    app.world_mut().entity_mut(second).despawn();

    let mut hands = app.world_mut().get_mut::<Hands>(mob).unwrap();
    hands.left = None;
    hands.right = None;
    let beaker = spawn_object(app.world_mut(), &ObjectSpec::new("beaker"), HERE).unwrap();
    app.world_mut()
        .get_mut::<Reagents>(beaker)
        .unwrap()
        .add("water", 10.0);
    app.world_mut().run_schedule(FixedUpdate);
    click(&mut app, mob, beaker);
    click(&mut app, mob, pot);
    assert!(app.world().get::<Reagents>(pot).unwrap().get("water") > 205.0);
}
