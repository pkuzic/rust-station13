//! Cloning. Spec: docs/reference/cloning.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::cloning::{Arrivals, ClonePod, CloneRecords, DnaScanner, HEAL_LEVEL};
use rs13_classic::closets::InCloset;
use rs13_classic::ghosts::LeftBody;
use rs13_classic::health::Body;
use rs13_classic::machines::MACHINE_TICKS;
use rs13_classic::monkeys::Monkey;
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_classic::pulling::Pulling;
use rs13_world::{DisplayName, GridPos, Hidden, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        's': (tile: (id: "floor"), objects: ["dna-scanner"]),
        'c': (tile: (id: "floor"), objects: ["cloning-console"]),
        'p': (tile: (id: "floor"), objects: ["clone-pod"]),
        'm': (tile: (id: "floor"), objects: ["monkey"]),
    },
    rows: [
        "WWWWWWWW",
        "Wscp...W",
        "W......W",
        "WWWWWWWW",
    ],
)"#;
const OPERATOR_AT: TilePos = TilePos::new(2, 1, 0);
const VICTIM_AT: TilePos = TilePos::new(1, 1, 0);

fn setup() -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let operator = spawn_player_mob(app.world_mut(), OPERATOR_AT);
    let victim = spawn_player_mob(app.world_mut(), VICTIM_AT);
    app.world_mut().get_mut::<DisplayName>(victim).unwrap().0 = "Ann Hart".into();
    ticks(&mut app, 1);
    (app, operator, victim)
}

fn ticks(app: &mut App, n: u64) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn find<T: Component>(app: &mut App) -> Entity {
    let mut q = app.world_mut().query_filtered::<Entity, With<T>>();
    q.single(app.world()).unwrap()
}

fn console(app: &mut App) -> Entity {
    find::<rs13_classic::cloning::CloningConsole>(app)
}

fn click(app: &mut App, mob: Entity, target: Entity) -> String {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    ticks(app, 1);
    let outbox = app.world().get::<Outbox>(mob).unwrap();
    outbox.0.last().map(|m| m.id.clone()).unwrap_or_default()
}

fn lay_in_scanner(app: &mut App, operator: Entity, person: Entity) {
    app.world_mut().entity_mut(operator).insert(Pulling(person));
    let scanner = find::<DnaScanner>(app);
    click(app, operator, scanner);
    assert_eq!(
        app.world().get::<DnaScanner>(scanner).unwrap().occupant,
        Some(person)
    );
    assert!(app.world().get::<Hidden>(person).is_some());
    assert!(app.world().get::<Pulling>(operator).is_none());
}

fn kill(app: &mut App, person: Entity) -> Entity {
    app.world_mut().get_mut::<Body>(person).unwrap().oxy = 500.0;
    ticks(app, 2);
    app.world().get::<LeftBody>(person).unwrap().0
}

#[test]
fn nobody_on_record_means_nothing_to_clone() {
    let (mut app, operator, victim) = setup();
    kill(&mut app, victim);
    let c = console(&mut app);
    assert_eq!(click(&mut app, operator, c), "cloning-nobody");
}

#[test]
fn someone_scanned_then_killed_grows_back_and_comes_out_healthy() {
    let (mut app, operator, victim) = setup();
    let c = console(&mut app);
    lay_in_scanner(&mut app, operator, victim);
    assert_eq!(click(&mut app, operator, c), "cloning-scanned");
    assert_eq!(click(&mut app, operator, c), "cloning-already-scanned");
    assert_eq!(app.world().resource::<CloneRecords>().0.len(), 1);

    // Out of the scanner, and dead. Clicking with nobody pulled lets them out.
    let scanner = find::<DnaScanner>(&mut app);
    click(&mut app, operator, scanner);
    assert!(app.world().get::<InCloset>(victim).is_none());
    let ghost = kill(&mut app, victim);

    assert_eq!(click(&mut app, operator, c), "cloning-started");
    assert!(app.world().resource::<CloneRecords>().0.is_empty());
    let pod = find::<ClonePod>(&mut app);
    let clone = app
        .world()
        .get::<ClonePod>(pod)
        .unwrap()
        .occupant
        .expect("growing");
    assert_eq!(app.world().resource::<Arrivals>().0, vec![(ghost, clone)]);
    assert_eq!(app.world().get::<DisplayName>(clone).unwrap().0, "Ann Hart");
    assert!(app.world().get::<Hidden>(clone).is_some());
    let start = app.world().get::<Body>(clone).unwrap().health();
    assert!(start < 0.0, "badly hurt: {start}");

    // One at a time.
    let other = spawn_player_mob(app.world_mut(), TilePos::new(1, 2, 0));
    lay_in_scanner(&mut app, operator, other);
    click(&mut app, operator, c);
    click(&mut app, operator, scanner);
    kill(&mut app, other);
    assert_eq!(click(&mut app, operator, c), "cloning-busy");

    ticks(&mut app, 100 * MACHINE_TICKS);
    assert!(
        app.world().get::<ClonePod>(pod).unwrap().occupant.is_none(),
        "it's done"
    );
    assert!(app.world().get::<Hidden>(clone).is_none());
    assert_eq!(
        app.world().get::<GridPos>(clone).unwrap().0,
        app.world().get::<GridPos>(pod).unwrap().0
    );
    let health = app.world().get::<Body>(clone).unwrap().health();
    assert!(health >= HEAL_LEVEL, "{health}");
}

#[test]
fn a_cremated_body_can_still_come_back_if_it_was_scanned() {
    let (mut app, operator, victim) = setup();
    let c = console(&mut app);
    lay_in_scanner(&mut app, operator, victim);
    click(&mut app, operator, c);
    let scanner = find::<DnaScanner>(&mut app);
    click(&mut app, operator, scanner);
    kill(&mut app, victim);
    app.world_mut().entity_mut(victim).despawn();
    assert_eq!(click(&mut app, operator, c), "cloning-started");
}

#[test]
fn monkeys_cant_be_scanned() {
    let (mut app, operator, _) = setup();
    let monkey = rs13_classic::objects::spawn_object(
        app.world_mut(),
        &rs13_world::ObjectSpec::new("monkey"),
        TilePos::new(1, 2, 0),
    )
    .unwrap();
    assert!(app.world().get::<Monkey>(monkey).is_some());
    lay_in_scanner(&mut app, operator, monkey);
    let c = console(&mut app);
    assert_eq!(click(&mut app, operator, c), "cloning-no-genes");
}
