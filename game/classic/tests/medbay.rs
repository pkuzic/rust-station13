//! Sleepers and cryo cells. Spec: docs/reference/medbay.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::chemistry::Reagents;
use rs13_classic::closets::{InCloset, PendingClosetExits};
use rs13_classic::health::{Body, Zone};
use rs13_classic::machines::{MACHINE_TICKS, PipeNets, side_bits};
use rs13_classic::medbay::{
    BodyScanner, CryoCell, PendingSleeperOps, ScanPanelOut, ScannerConsole, Sleeper, SleeperConsole,
    SleeperPanelOut,
};
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_classic::pulling::Pulling;
use rs13_simulation::atmos::Gas;
use rs13_world::{Appearance, Dir, Hidden, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        's': (tile: (id: "floor"), objects: ["sleeper"]),
        'k': (tile: (id: "floor"), objects: ["sleeper-console"]),
        'c': (tile: (id: "floor"), objects: ["cryo-cell"]),
        'b': (tile: (id: "floor"), objects: ["body-scanner"]),
        'o': (tile: (id: "floor"), objects: ["scanner-console"]),
        'p': (tile: (id: "floor"), objects: [(id: "pipe", dir: North)]),
    },
    rows: [
        "WWWWWWWW",
        "Wsk.c..W",
        "W...p..W",
        "Wbo....W",
        "WWWWWWWW",
    ],
)"#;
const NEAR_SLEEPER: TilePos = TilePos::new(2, 2, 0);
const NEAR_CELL: TilePos = TilePos::new(4, 2, 0);

fn setup(at: TilePos) -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let medic = spawn_player_mob(app.world_mut(), at);
    let patient = spawn_player_mob(app.world_mut(), TilePos::new(at.x + 1, at.y, 0));
    app.world_mut().run_schedule(FixedUpdate);
    (app, medic, patient)
}

fn ticks(app: &mut App, n: u64) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn click(app: &mut App, mob: Entity, target: Entity) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    ticks(app, 1);
}

fn find<T: Component>(app: &mut App) -> Entity {
    let mut q = app.world_mut().query_filtered::<Entity, With<T>>();
    q.single(app.world()).unwrap()
}

fn lay_in_sleeper(app: &mut App, medic: Entity, patient: Entity) -> Entity {
    let sleeper = find::<Sleeper>(app);
    app.world_mut().entity_mut(medic).insert(Pulling(patient));
    click(app, medic, sleeper);
    assert_eq!(
        app.world().get::<Sleeper>(sleeper).unwrap().occupant,
        Some(patient)
    );
    assert_eq!(app.world().get::<Appearance>(sleeper).unwrap().state, "sleeper");
    assert!(app.world().get::<Hidden>(patient).is_some());
    sleeper
}

#[test]
fn a_sleeper_keeps_someone_stable_and_lets_them_walk_out() {
    let (mut app, medic, patient) = setup(NEAR_SLEEPER);
    let sleeper = lay_in_sleeper(&mut app, medic, patient);
    app.world_mut().get_mut::<Body>(patient).unwrap().oxy = 5.0;
    ticks(&mut app, MACHINE_TICKS);
    assert_eq!(app.world().get::<Body>(patient).unwrap().oxy, 0.0);
    assert!(app.world().get::<Reagents>(patient).unwrap().get("inaprovaline") > 4.0);

    app.world_mut()
        .resource_mut::<PendingClosetExits>()
        .0
        .push((patient, sleeper));
    ticks(&mut app, 2);
    assert!(app.world().get::<Sleeper>(sleeper).unwrap().occupant.is_none());
    assert!(app.world().get::<InCloset>(patient).is_none());
    assert!(app.world().get::<Hidden>(patient).is_none());
}

#[test]
fn the_console_shows_and_injects_up_to_its_limit() {
    let (mut app, medic, patient) = setup(NEAR_SLEEPER);
    lay_in_sleeper(&mut app, medic, patient);
    let console = find::<SleeperConsole>(&mut app);
    click(&mut app, medic, console);
    ticks(&mut app, 1);
    let view = app
        .world()
        .get::<SleeperPanelOut>(medic)
        .unwrap()
        .0
        .clone()
        .expect("open");
    assert!(view.occupant.is_some());
    let line = view.reagents.iter().position(|(r, _)| r == "dexalin").unwrap();
    for _ in 0..4 {
        app.world_mut()
            .get_mut::<PendingSleeperOps>(medic)
            .unwrap()
            .0
            .push((console, line));
        ticks(&mut app, 1);
    }
    let units = app.world().get::<Reagents>(patient).unwrap().get("dexalin");
    assert!(units > 50.0 && units <= 60.0, "{units}");
}

#[test]
fn someone_in_crit_only_gets_inaprovaline() {
    let (mut app, medic, patient) = setup(NEAR_SLEEPER);
    lay_in_sleeper(&mut app, medic, patient);
    let console = find::<SleeperConsole>(&mut app);
    app.world_mut().get_mut::<Body>(patient).unwrap().tox = 150.0;
    click(&mut app, medic, console);
    ticks(&mut app, 1);
    let view = app
        .world()
        .get::<SleeperPanelOut>(medic)
        .unwrap()
        .0
        .clone()
        .expect("open");
    let line = view.reagents.iter().position(|(r, _)| r == "dexalin").unwrap();
    app.world_mut()
        .get_mut::<PendingSleeperOps>(medic)
        .unwrap()
        .0
        .push((console, line));
    ticks(&mut app, 1);
    assert_eq!(app.world().get::<Reagents>(patient).unwrap().get("dexalin"), 0.0);
    let last = app
        .world()
        .get::<Outbox>(medic)
        .unwrap()
        .0
        .last()
        .unwrap()
        .id
        .clone();
    assert_eq!(last, "sleeper-unstable");
}

fn fill_pipe(app: &mut App, temperature: f64) {
    let at = TilePos::new(4, 2, 0);
    let mut nets = app.world_mut().resource_mut::<PipeNets>();
    let net = nets.net_toward(at, side_bits(Dir::North)).expect("a pipe");
    let mix = &mut nets.nets[net].mix;
    mix.set(Gas::Oxygen, 50.0);
    mix.temperature = temperature;
}

fn lay_in_cell(app: &mut App, medic: Entity, patient: Entity) -> Entity {
    let cell = find::<CryoCell>(app);
    app.world_mut().entity_mut(medic).insert(Pulling(patient));
    click(app, medic, cell);
    assert_eq!(app.world().get::<CryoCell>(cell).unwrap().occupant, Some(patient));
    click(app, medic, cell);
    assert!(app.world().get::<CryoCell>(cell).unwrap().on);
    assert_eq!(app.world().get::<Appearance>(cell).unwrap().state, "celltop_1");
    cell
}

/// Oxygen damage left after 3 machine ticks in a cell with gas this cold. Breathing clears some of
/// it anyway, so what counts is the difference.
fn oxy_after(temperature: f64) -> f32 {
    let (mut app, medic, patient) = setup(NEAR_CELL);
    ticks(&mut app, MACHINE_TICKS);
    lay_in_cell(&mut app, medic, patient);
    app.world_mut().get_mut::<Body>(patient).unwrap().oxy = 30.0;
    fill_pipe(&mut app, temperature);
    ticks(&mut app, 3 * MACHINE_TICKS);
    app.world().get::<Body>(patient).unwrap().oxy
}

#[test]
fn cold_gas_clears_oxygen_damage_and_warm_gas_doesnt() {
    assert!(oxy_after(260.0) < oxy_after(300.0) - 2.0);
}

#[test]
fn cryoxadone_only_works_in_very_cold_gas() {
    let (mut app, medic, patient) = setup(NEAR_CELL);
    ticks(&mut app, MACHINE_TICKS);
    lay_in_cell(&mut app, medic, patient);
    app.world_mut()
        .get_mut::<Body>(patient)
        .unwrap()
        .take_damage(Zone::Chest, 20.0, 0.0);
    app.world_mut()
        .get_mut::<Reagents>(patient)
        .unwrap()
        .add("cryoxadone", 1000.0);
    fill_pipe(&mut app, 300.0);
    ticks(&mut app, 3 * MACHINE_TICKS);
    assert_eq!(
        app.world().get::<Body>(patient).unwrap().organ(Zone::Chest).brute,
        20.0,
        "too warm"
    );

    fill_pipe(&mut app, 150.0);
    ticks(&mut app, 4 * MACHINE_TICKS);
    assert!(app.world().get::<Body>(patient).unwrap().organ(Zone::Chest).brute < 20.0);
}

#[test]
fn a_cell_with_no_gas_does_nothing_and_it_takes_a_beaker() {
    let (mut app, medic, patient) = setup(NEAR_CELL);
    ticks(&mut app, MACHINE_TICKS);
    let cell = lay_in_cell(&mut app, medic, patient);
    app.world_mut()
        .get_mut::<Body>(patient)
        .unwrap()
        .take_damage(Zone::Chest, 20.0, 0.0);
    app.world_mut()
        .get_mut::<Reagents>(patient)
        .unwrap()
        .add("cryoxadone", 1000.0);
    ticks(&mut app, 2 * MACHINE_TICKS);
    assert_eq!(
        app.world().get::<Body>(patient).unwrap().organ(Zone::Chest).brute,
        20.0
    );

    let beaker = rs13_classic::objects::spawn_object(
        app.world_mut(),
        &rs13_world::ObjectSpec::new("beaker"),
        NEAR_CELL,
    )
    .unwrap();
    app.world_mut()
        .get_mut::<Reagents>(beaker)
        .unwrap()
        .add("kelotane", 10.0);
    click(&mut app, medic, beaker);
    click(&mut app, medic, cell);
    assert_eq!(app.world().get::<CryoCell>(cell).unwrap().beaker, Some(beaker));
    fill_pipe(&mut app, 300.0);
    ticks(&mut app, 2 * MACHINE_TICKS);
    assert!(
        app.world().get::<Reagents>(beaker).unwrap().get("kelotane") < 10.0,
        "a dose went in"
    );
}

#[test]
fn a_body_scanner_shows_whats_hurt_and_lets_them_out() {
    let (mut app, medic, patient) = setup(NEAR_SLEEPER);
    let scanner = find::<BodyScanner>(&mut app);
    let console = find::<ScannerConsole>(&mut app);
    click(&mut app, medic, console);
    ticks(&mut app, 1);
    let view = app
        .world()
        .get::<ScanPanelOut>(medic)
        .unwrap()
        .0
        .clone()
        .expect("open");
    assert!(view.occupant.is_none());

    app.world_mut().entity_mut(medic).insert(Pulling(patient));
    click(&mut app, medic, scanner);
    assert_eq!(
        app.world().get::<BodyScanner>(scanner).unwrap().occupant,
        Some(patient)
    );
    assert_eq!(
        app.world().get::<Appearance>(scanner).unwrap().state,
        "body_scanner_1"
    );
    app.world_mut()
        .get_mut::<Body>(patient)
        .unwrap()
        .take_damage(Zone::Chest, 12.0, 7.0);
    ticks(&mut app, 2);
    let view = app
        .world()
        .get::<ScanPanelOut>(medic)
        .unwrap()
        .0
        .clone()
        .expect("open");
    assert!(view.occupant.is_some());
    assert_eq!(view.parts, vec![(Zone::Chest.noun_id().to_owned(), 12, 7)]);

    click(&mut app, medic, scanner);
    assert!(
        app.world()
            .get::<BodyScanner>(scanner)
            .unwrap()
            .occupant
            .is_none()
    );
    assert!(app.world().get::<Hidden>(patient).is_none());
    assert_eq!(
        app.world().get::<Appearance>(scanner).unwrap().state,
        "body_scanner_0"
    );
}
