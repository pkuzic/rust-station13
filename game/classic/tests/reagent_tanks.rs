//! Tanks to fill a beaker from, and vendors that want access. Specs: docs/reference/chemistry.md,
//! docs/reference/vending.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::access::IdCard;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::chemistry::Reagents;
use rs13_classic::extinguishers::WaterTank;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::vending::{PendingVendOps, Vending};
use rs13_classic::welding::FuelTank;
use rs13_world::{Appearance, GridPos, ObjectSpec, TileMap, TilePos};

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
const NEXT: TilePos = TilePos::new(2, 1, 0);

fn setup(next_to: &str, holding: &str) -> (App, Entity, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), HERE);
    let thing = spawn_object(app.world_mut(), &ObjectSpec::new(next_to), NEXT).unwrap();
    let held = spawn_object(app.world_mut(), &ObjectSpec::new(holding), HERE).unwrap();
    app.world_mut().run_schedule(FixedUpdate);
    click(&mut app, mob, held);
    (app, mob, thing, held)
}

fn click(app: &mut App, mob: Entity, target: Entity) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    app.world_mut().run_schedule(FixedUpdate);
}

fn last_message(app: &App, mob: Entity) -> String {
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
fn each_tank_fills_a_beaker_by_its_own_amount() {
    for (proto, reagent, amount) in [
        ("pepper-tank", "condensedcapsaicin", 50.0),
        ("water-cooler", "water", 5.0),
        ("beer-keg", "beer", 10.0),
    ] {
        let (mut app, mob, tank, beaker) = setup(proto, "beaker");
        let before = app.world().get::<Reagents>(tank).unwrap().total();
        click(&mut app, mob, tank);
        assert_eq!(
            app.world().get::<Reagents>(beaker).unwrap().get(reagent),
            amount,
            "{proto}"
        );
        assert_eq!(
            app.world().get::<Reagents>(tank).unwrap().total(),
            before - amount,
            "{proto}"
        );
        assert_eq!(last_message(&app, mob), "tank-filled", "{proto}");
    }
}

#[test]
fn a_full_beaker_stops_taking_and_says_so() {
    let (mut app, mob, tank, beaker) = setup("pepper-tank", "beaker");
    click(&mut app, mob, tank);
    let before = app.world().get::<Reagents>(tank).unwrap().total();
    click(&mut app, mob, tank);
    assert_eq!(app.world().get::<Reagents>(tank).unwrap().total(), before);
    assert_eq!(app.world().get::<Reagents>(beaker).unwrap().total(), 50.0);
    assert_eq!(last_message(&app, mob), "tank-glass-full");
}

#[test]
fn water_and_fuel_tanks_fill_from_their_own_counts() {
    let (mut app, mob, tank, beaker) = setup("water-tank", "beaker");
    let before = app.world().get::<WaterTank>(tank).unwrap().water;
    click(&mut app, mob, tank);
    assert_eq!(app.world().get::<WaterTank>(tank).unwrap().water, before - 10);
    assert_eq!(app.world().get::<Reagents>(beaker).unwrap().get("water"), 10.0);

    let (mut app, mob, tank, beaker) = setup("fuel-tank", "beaker");
    let before = app.world().get::<FuelTank>(tank).unwrap().fuel;
    click(&mut app, mob, tank);
    assert_eq!(app.world().get::<FuelTank>(tank).unwrap().fuel, before - 10);
    assert_eq!(app.world().get::<Reagents>(beaker).unwrap().get("fuel"), 10.0);
}

fn press_first_line(app: &mut App, mob: Entity, machine: Entity) -> u16 {
    app.world_mut()
        .get_mut::<PendingVendOps>(mob)
        .unwrap()
        .0
        .push((machine, 0));
    app.world_mut().run_schedule(FixedUpdate);
    app.world().get::<Vending>(machine).unwrap().stock[0]
}

#[test]
fn a_vendor_that_wants_access_refuses_without_it() {
    let (mut app, mob, machine, card) = setup("vending-medical", "id-card");
    assert_eq!(press_first_line(&mut app, mob, machine), 4);
    assert_eq!(last_message(&app, mob), "vending-denied");

    app.world_mut().entity_mut(card).insert(IdCard {
        access: vec!["medical".to_owned()],
    });
    assert_eq!(press_first_line(&mut app, mob, machine), 3);
}

#[test]
fn a_wall_vendor_wants_nothing_and_stands_in_nobodys_way() {
    let (mut app, mob, machine, _) = setup("vending-wallmed", "beaker");
    assert_eq!(press_first_line(&mut app, mob, machine), 2);
    assert!(app.world().get::<rs13_world::Dense>(machine).is_none());
    let mut q = app.world_mut().query::<(&Appearance, &GridPos)>();
    assert!(q.iter(app.world()).any(|(a, _)| a.sprite == "vending-wallmed"));
}
