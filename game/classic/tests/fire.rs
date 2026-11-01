//! Plasma fires. Spec: docs/reference/fire.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::atmos::StationAtmos;
use rs13_classic::fires::Fire;
use rs13_classic::objects::{spawn_map, spawn_object};
use rs13_classic::welding::Welder;
use rs13_simulation::atmos::gas::{T0C, T20C};
use rs13_simulation::atmos::{Gas, GasMix};
use rs13_world::{AtmosView, ObjectSpec, TileMap, TilePos};

/// A closed room of 3 × 3 tiles.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
    },
    rows: [
        "WWWWW",
        "W...W",
        "W...W",
        "W...W",
        "WWWWW",
    ],
)"#;

const MIDDLE: TilePos = TilePos::new(2, 2, 0);

fn setup(plasma: f64) -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().run_schedule(FixedUpdate);
    let mut gas = GasMix::default();
    gas.temperature = T20C;
    gas.set(Gas::Plasma, plasma);
    app.world_mut()
        .resource_mut::<StationAtmos>()
        .0
        .add_at(MIDDLE, &gas);
    app
}

fn seconds(app: &mut App, s: u64) {
    for _ in 0..s * 20 {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn room(app: &App) -> GasMix {
    let atmos = &app.world().resource::<StationAtmos>().0;
    atmos.zones()[atmos.zone_at(MIDDLE).unwrap()].mix
}

fn light_a_welder(app: &mut App) {
    let welder = spawn_object(app.world_mut(), &ObjectSpec::new("welder"), MIDDLE).unwrap();
    app.world_mut().get_mut::<Welder>(welder).unwrap().lit = true;
}

fn fires(app: &mut App) -> usize {
    let mut q = app.world_mut().query_filtered::<(), With<Fire>>();
    q.iter(app.world()).count()
}

#[test]
fn plasma_and_air_at_room_temperature_do_not_burn_by_themselves() {
    let mut app = setup(40.0);
    seconds(&mut app, 10);
    assert_eq!(room(&app).get(Gas::CarbonDioxide), 0.0);
    assert_eq!(fires(&mut app), 0);
}

#[test]
fn a_welding_flame_sets_plasma_alight_and_the_room_burns() {
    let mut app = setup(40.0);
    let oxygen = room(&app).get(Gas::Oxygen);
    light_a_welder(&mut app);
    seconds(&mut app, 60);
    let air = room(&app);
    assert!(
        air.temperature > 250.0 + T0C,
        "the fire heats the room: {} K",
        air.temperature
    );
    assert!(air.get(Gas::Plasma) < 40.0, "plasma burns");
    assert!(air.get(Gas::Oxygen) < oxygen, "oxygen is used");
    assert!(air.get(Gas::CarbonDioxide) > 0.0, "carbon dioxide is made");
    assert_eq!(fires(&mut app), 9, "fire on every tile of the room");
}

#[test]
fn without_plasma_a_flame_does_nothing() {
    let mut app = setup(0.0);
    light_a_welder(&mut app);
    seconds(&mut app, 20);
    let air = room(&app);
    assert!((air.temperature - T20C).abs() < 1.0, "{} K", air.temperature);
    assert_eq!(fires(&mut app), 0);
}

#[test]
fn the_air_picture_shows_plasma_on_every_tile_of_its_room() {
    // 40 mol over 9 tiles: about 4.4 mol a tile, plenty to see.
    let mut app = setup(40.0);
    seconds(&mut app, 1);
    let mut q = app.world_mut().query::<&AtmosView>();
    let view = q.single(app.world()).unwrap();
    let width = 5;
    let at = |x: usize, y: usize| view.plasma_dmol[y * width + x];
    assert!((40..=50).contains(&at(2, 2)), "{} tenths of a mole", at(2, 2));
    assert_eq!(at(1, 1), at(3, 3), "the room's air is one");
    assert_eq!(at(0, 0), 0, "none in the wall");
    assert!(view.n2o_dmol.iter().all(|&n| n == 0));
}
