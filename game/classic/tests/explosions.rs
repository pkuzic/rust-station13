//! Explosions. Spec: docs/reference/explosions.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::atmos::StationAtmos;
use rs13_classic::equipment::TankGas;
use rs13_classic::explosions::{explode, spawn_charge};
use rs13_classic::health::{Body, Vitals};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_simulation::atmos::Gas;
use rs13_world::{GridPos, MapLayer, MobStat, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        'R': (tile: (id: "r-wall", solid: true)),
        '.': (tile: (id: "floor")),
    },
    rows: [
        "WWWWWWWWWWWWWWWWWWWWWWWWW",
        "W.........W.............W",
        "W.........W.............W",
        "W.........W.............W",
        "W.........W.............W",
        "WWWWWWWWWWWWWWWWWWWWWWWWW",
    ],
)"#;
const MIDDLE: TilePos = TilePos::new(5, 2, 0);

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().run_schedule(FixedUpdate);
    app
}

fn seconds(app: &mut App, n: u32) {
    for _ in 0..n * 20 {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn tile(app: &mut App, at: TilePos) -> String {
    let mut maps = app.world_mut().query::<&MapLayer>();
    maps.single(app.world()).unwrap().0.tile(at).unwrap().id.clone()
}

#[test]
fn a_blast_opens_the_middle_to_space_and_air_rushes_out() {
    let mut app = setup();
    let air = |app: &App, at| app.world().resource::<StationAtmos>().0.pressure_at(at);
    assert!(air(&app, MIDDLE) > 50.0);
    explode(app.world_mut(), MIDDLE, 3, 5, 7, 9);
    seconds(&mut app, 2);
    assert_eq!(tile(&mut app, MIDDLE), "space");
    seconds(&mut app, 10);
    assert!(air(&app, TilePos::new(9, 4, 0)) < 50.0, "the room is venting");
}

#[test]
fn a_wall_shelters_what_is_behind_it() {
    let mut app = setup();
    let near = spawn_player_mob(app.world_mut(), TilePos::new(9, 2, 0));
    let behind = spawn_player_mob(app.world_mut(), TilePos::new(11, 2, 0));
    explode(app.world_mut(), TilePos::new(7, 2, 0), 2, 4, 5, 6);
    seconds(&mut app, 2);
    let hurt = |app: &App, mob| 100.0 - app.world().get::<Body>(mob).unwrap().health();
    assert!(hurt(&app, near) > 0.0, "in the open");
    assert!(
        hurt(&app, behind) < hurt(&app, near),
        "behind the wall: {} {}",
        hurt(&app, behind),
        hurt(&app, near)
    );
}

#[test]
fn walls_near_it_come_down_to_plating_with_their_metal() {
    let mut app = setup();
    explode(app.world_mut(), TilePos::new(9, 2, 0), 0, 3, 3, 3);
    seconds(&mut app, 2);
    assert_ne!(tile(&mut app, TilePos::new(10, 2, 0)), "wall");
}

#[test]
fn people_die_at_the_middle_and_suffer_further_out() {
    let mut app = setup();
    let middle = spawn_player_mob(app.world_mut(), MIDDLE);
    let further = spawn_player_mob(app.world_mut(), TilePos::new(1, 2, 0));
    explode(app.world_mut(), MIDDLE, 3, 4, 5, 6);
    seconds(&mut app, 2);
    assert_eq!(app.world().get::<Vitals>(middle).unwrap().stat, MobStat::Dead);
    let health = app.world().get::<Body>(further).unwrap().health();
    assert!(health < 100.0 && health > -100.0, "{health}");
}

#[test]
fn items_go_and_what_was_in_a_box_is_left_behind() {
    let mut app = setup();
    let item = spawn_object(app.world_mut(), &ObjectSpec::new("crowbar"), MIDDLE).unwrap();
    let toolbox = spawn_object(
        app.world_mut(),
        &ObjectSpec::new("toolbox-mechanical"),
        TilePos::new(5, 3, 0),
    )
    .unwrap();
    let inside: Vec<Entity> = {
        let mut q = app
            .world_mut()
            .query::<(Entity, &rs13_classic::storage::InStorage)>();
        q.iter(app.world())
            .filter(|(_, s)| s.0 == toolbox)
            .map(|(e, _)| e)
            .collect()
    };
    assert!(!inside.is_empty());
    explode(app.world_mut(), MIDDLE, 3, 4, 5, 6);
    seconds(&mut app, 2);
    assert!(app.world().get_entity(item).is_err());
    assert!(app.world().get_entity(toolbox).is_err());
    let survived = inside
        .iter()
        .filter(|e| app.world().get_entity(**e).is_ok())
        .count();
    for held in inside.iter().filter(|e| app.world().get_entity(**e).is_ok()) {
        assert!(app.world().get::<GridPos>(*held).is_some(), "on the floor");
    }
    let _ = survived;
}

#[test]
fn a_tank_under_too_much_pressure_blows_up() {
    let mut app = setup();
    let tank = spawn_object(app.world_mut(), &ObjectSpec::new("plasma-tank"), MIDDLE).unwrap();
    let mut gas = app.world_mut().get_mut::<TankGas>(tank).unwrap();
    gas.mix.set(Gas::Plasma, 400.0);
    gas.mix.temperature = 300.0;
    seconds(&mut app, 3);
    assert!(app.world().get_entity(tank).is_err(), "gone");
    assert_eq!(tile(&mut app, MIDDLE), "space");
}

#[test]
fn a_tank_a_little_over_leaks_after_a_while() {
    let mut app = setup();
    let tank = spawn_object(app.world_mut(), &ObjectSpec::new("plasma-tank"), MIDDLE).unwrap();
    let mut gas = app.world_mut().get_mut::<TankGas>(tank).unwrap();
    let moles = gas.mix.total_moles();
    let scale = 35.0 * 101.325 / gas.pressure();
    gas.mix = gas.mix.scaled(scale);
    let before = moles * scale;
    seconds(&mut app, 8);
    let after = app.world().get::<TankGas>(tank).unwrap().mix.total_moles();
    assert!(after < before, "{after} {before}");
}

#[test]
fn a_detonator_opens_then_sets_it_off() {
    let mut app = setup();
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(14, 2, 0));
    let (charge, detonator) = spawn_charge(app.world_mut(), "c4-small", MIDDLE).unwrap();
    app.world_mut()
        .entity_mut(detonator)
        .remove::<GridPos>()
        .insert(rs13_classic::hands::HeldBy(mob));
    {
        let mut hands = app
            .world_mut()
            .get_mut::<rs13_classic::hands::Hands>(mob)
            .unwrap();
        let active = hands.active;
        *hands.slot_mut(active) = Some(detonator);
    }
    let press = |app: &mut App| {
        app.world_mut()
            .get_mut::<PendingActions>(mob)
            .unwrap()
            .0
            .push(Action::UseOn(detonator));
        app.world_mut().run_schedule(FixedUpdate);
    };
    press(&mut app);
    seconds(&mut app, 7);
    assert!(
        app.world().get_entity(charge).is_ok(),
        "opening it does nothing yet"
    );
    press(&mut app);
    seconds(&mut app, 3);
    assert!(app.world().get_entity(charge).is_ok(), "on a five second fuse");
    seconds(&mut app, 4);
    assert!(app.world().get_entity(charge).is_err());
}
