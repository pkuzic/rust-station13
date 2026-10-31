//! The engine core: supermatter, lasers and the laser computer. Spec:
//! docs/reference/engine.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::atmos::StationAtmos;
use rs13_classic::health::{Body, Vitals};
use rs13_classic::machines::MACHINE_TICKS;
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_classic::supermatter::{
    Beam, EngineLaser, LASER_POWER, LaserComputer, LaserOp, LaserPanelOut, PendingLaserOps,
};
use rs13_simulation::atmos::Gas;
use rs13_simulation::atmos::gas::{ONE_ATMOSPHERE, R, T20C};
use rs13_world::{GridPos, MobStat, TileMap, TilePos};

/// A room with the crystal in the middle, a laser west of it firing east, and the laser
/// computer in the far corner.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'L': (tile: (id: "floor"), objects: [(id: "engine-laser", dir: East, vars: {"id": "west"})]),
        'S': (tile: (id: "floor"), objects: ["supermatter"]),
        'C': (tile: (id: "floor"), objects: [(id: "laser-computer", vars: {"id": "list(\"west\")"})]),
    },
    rows: [
        "WWWWWWWWWWWW",
        "W..........W",
        "WL...S.....W",
        "W..........W",
        "W.........CW",
        "WWWWWWWWWWWW",
    ],
)"#;

const LASER_AT: TilePos = TilePos::new(1, 3, 0);
const CRYSTAL: TilePos = TilePos::new(5, 3, 0);
const OPERATOR: TilePos = TilePos::new(9, 1, 0);

fn setup() -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let operator = spawn_player_mob(app.world_mut(), OPERATOR);
    app.world_mut().run_schedule(FixedUpdate);
    let mut q = app.world_mut().query_filtered::<Entity, With<LaserComputer>>();
    let computer = q.single(app.world()).unwrap();
    (app, operator, computer)
}

fn cycles(app: &mut App, n: u64) {
    for _ in 0..n * MACHINE_TICKS {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn op(app: &mut App, mob: Entity, computer: Entity, op: LaserOp) {
    app.world_mut()
        .get_mut::<PendingLaserOps>(mob)
        .unwrap()
        .0
        .push((computer, op));
    app.world_mut().run_schedule(FixedUpdate);
}

fn laser(app: &mut App) -> EngineLaser {
    let mut q = app.world_mut().query::<&EngineLaser>();
    q.single(app.world()).unwrap().clone()
}

fn beam_tiles(app: &mut App) -> Vec<TilePos> {
    let mut q = app.world_mut().query_filtered::<&GridPos, With<Beam>>();
    let mut tiles: Vec<TilePos> = q.iter(app.world()).map(|p| p.0).collect();
    tiles.sort();
    tiles
}

fn fill_with_oxygen(app: &mut App) {
    let mut atmos = app.world_mut().resource_mut::<StationAtmos>();
    let _ = atmos.0.remove_at(CRYSTAL, 1e9);
    let mut oxygen = rs13_simulation::atmos::GasMix::default();
    oxygen.temperature = T20C;
    // One atmosphere of pure oxygen across the room's 50 tiles.
    oxygen.set(Gas::Oxygen, ONE_ATMOSPHERE * 2500.0 * 50.0 / (R * T20C));
    atmos.0.add_at(CRYSTAL, &oxygen);
}

#[test]
fn the_computer_opens_and_works_the_lasers() {
    let (mut app, operator, computer) = setup();
    assert_eq!(laser(&mut app).power, LASER_POWER);
    assert!(!laser(&mut app).on);

    app.world_mut()
        .get_mut::<PendingActions>(operator)
        .unwrap()
        .0
        .push(Action::UseOn(computer));
    cycles(&mut app, 1);
    let view = app
        .world()
        .get::<LaserPanelOut>(operator)
        .unwrap()
        .0
        .clone()
        .expect("the window opens");
    assert_eq!((view.lasers, view.power, view.online), (1, LASER_POWER, false));

    op(&mut app, operator, computer, LaserOp::Power(1000));
    op(&mut app, operator, computer, LaserOp::Power(1000));
    op(&mut app, operator, computer, LaserOp::Power(1000));
    assert_eq!(laser(&mut app).power, 3000, "kept within 1–3000");
    op(&mut app, operator, computer, LaserOp::Power(-5000));
    assert_eq!(laser(&mut app).power, 1);

    op(&mut app, operator, computer, LaserOp::Toggle);
    assert!(laser(&mut app).on);
}

#[test]
fn a_firing_laser_draws_a_beam_up_to_the_crystal() {
    let (mut app, operator, computer) = setup();
    assert!(beam_tiles(&mut app).is_empty());
    op(&mut app, operator, computer, LaserOp::Toggle);
    cycles(&mut app, 1);
    let expected: Vec<TilePos> = (2..5).map(|x| TilePos::new(x, LASER_AT.y, 0)).collect();
    assert_eq!(beam_tiles(&mut app), expected, "it stops at the dense crystal");
    op(&mut app, operator, computer, LaserOp::Toggle);
    cycles(&mut app, 1);
    assert!(beam_tiles(&mut app).is_empty(), "switched off, the beam goes");
}

#[test]
fn a_beam_stops_at_a_person_and_burns_them() {
    let (mut app, operator, computer) = setup();
    // Far enough from the crystal not to be killed by it.
    let victim = spawn_player_mob(app.world_mut(), TilePos::new(2, 3, 0));
    op(&mut app, operator, computer, LaserOp::Toggle);
    cycles(&mut app, 2);
    assert!(
        beam_tiles(&mut app).is_empty(),
        "nothing between the laser and them"
    );
    let burn: f32 = rs13_classic::health::Zone::ALL
        .iter()
        .map(|z| app.world().get::<Body>(victim).unwrap().organ(*z).burn)
        .sum();
    assert!(burn > 0.0);
}

#[test]
fn lasers_in_oxygen_make_the_crystal_heat_the_room_and_give_off_oxygen() {
    let (mut app, operator, computer) = setup();
    fill_with_oxygen(&mut app);
    let air = |app: &App| app.world().resource::<StationAtmos>().0.air_at(CRYSTAL);
    let (t0, o0) = (air(&app).temperature, air(&app).get(Gas::Oxygen));
    // At full power: below that, the oxygen given off each cycle rounds down to nothing.
    op(&mut app, operator, computer, LaserOp::Power(2500));
    op(&mut app, operator, computer, LaserOp::Toggle);
    cycles(&mut app, 10);
    let (t1, o1) = (air(&app).temperature, air(&app).get(Gas::Oxygen));
    // The heat spreads over the whole room (Luna: "1 °C / #tiles in core" per 4 °C).
    assert!(t1 > t0 + 0.1, "{t0} → {t1}");
    assert!(o1 > o0, "{o0} → {o1}");
}

#[test]
fn nothing_lives_within_three_tiles_of_the_crystal() {
    let (mut app, _, _) = setup();
    let near = spawn_player_mob(app.world_mut(), TilePos::new(7, 3, 0));
    let far = spawn_player_mob(app.world_mut(), TilePos::new(9, 3, 0));
    cycles(&mut app, 2);
    assert_eq!(app.world().get::<Vitals>(near).unwrap().stat, MobStat::Dead);
    assert_ne!(app.world().get::<Vitals>(far).unwrap().stat, MobStat::Dead);
}
