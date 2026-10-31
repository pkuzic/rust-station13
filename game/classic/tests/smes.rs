//! SMES units: map settings, charging through a terminal, and their window. Spec:
//! docs/reference/power.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::machines::MACHINE_TICKS;
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_classic::power::{POWER_DT, SMES_MAX_LEVEL, SMES_UNIT, Smes};
use rs13_classic::smes_panel::{Level, PendingSmesOps, SmesOp, SmesPanelOut};
use rs13_world::{GridPos, TileMap, TilePos};

/// A full SMES feeding a cable to a terminal, which charges a second, empty SMES east of
/// it.
fn map(input: u32) -> String {
    format!(
        r#"(
    legend: {{
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'A': (tile: (id: "floor"), objects: ["smes", (id: "cable", ends: [East])]),
        '-': (tile: (id: "floor"), objects: [(id: "cable", ends: [West, East])]),
        'T': (tile: (id: "floor"), objects: [(id: "terminal", dir: East), (id: "cable", ends: [West])]),
        'B': (tile: (id: "floor"), objects: [(id: "smes", vars: {{"charge": "0", "chargelevel": "{input}", "output": "0"}})]),
    }},
    rows: [
        "WWWWWWW",
        "WA-TB.W",
        "W.....W",
        "WWWWWWW",
    ],
)"#
    )
}

const SOURCE: TilePos = TilePos::new(1, 2, 0);
const STORE: TilePos = TilePos::new(4, 2, 0);

fn setup(input: u32) -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(&map(input), 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().run_schedule(FixedUpdate);
    app
}

fn cycles(app: &mut App, n: u64) {
    for _ in 0..n * MACHINE_TICKS {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn unit(app: &mut App, at: TilePos) -> (Entity, Smes) {
    let mut q = app.world_mut().query::<(Entity, &GridPos, &Smes)>();
    let (e, _, s) = q.iter(app.world()).find(|(_, p, _)| p.0 == at).unwrap();
    (e, *s)
}

#[test]
fn map_settings_are_lunas_units() {
    let mut app = setup(5000);
    let (_, store) = unit(&mut app, STORE);
    assert_eq!(store.charge, 0.0);
    assert_eq!(store.capacity, 5.0e6 * SMES_UNIT);
    assert_eq!((store.input, store.output), (5000.0, 0.0));
    assert!(store.charge_auto, "Luna's default");
    let (_, source) = unit(&mut app, SOURCE);
    assert_eq!(source.charge, source.capacity, "full by default");
}

#[test]
fn a_smes_charges_through_its_terminal_at_its_input_level() {
    let mut app = setup(5000);
    cycles(&mut app, 3);
    let (_, store) = unit(&mut app, STORE);
    assert!(store.charging);
    let per_cycle = 5000.0 * POWER_DT;
    assert!(
        (store.charge - 2.0 * per_cycle).abs() < per_cycle + 1.0,
        "about {per_cycle} J a cycle: {}",
        store.charge
    );
}

#[test]
fn with_no_input_level_it_does_not_charge() {
    let mut app = setup(0);
    cycles(&mut app, 3);
    let (_, store) = unit(&mut app, STORE);
    assert_eq!(store.charge, 0.0);
    assert!(!store.charging);
}

#[test]
fn the_window_sets_levels_and_switches() {
    let mut app = setup(0);
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(4, 1, 0));
    let (store, _) = unit(&mut app, STORE);
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(store));
    cycles(&mut app, 1);
    assert!(
        app.world().get::<SmesPanelOut>(mob).unwrap().0.is_some(),
        "the window opens"
    );

    let press = |app: &mut App, op| {
        app.world_mut()
            .get_mut::<PendingSmesOps>(mob)
            .unwrap()
            .0
            .push((store, op));
        app.world_mut().run_schedule(FixedUpdate);
    };
    press(&mut app, SmesOp::Input(Level::Max));
    press(&mut app, SmesOp::Output(Level::Change(1000.0)));
    press(&mut app, SmesOp::Output(Level::Change(-5000.0)));
    press(&mut app, SmesOp::ChargeMode);
    let (_, s) = unit(&mut app, STORE);
    assert_eq!(s.input, SMES_MAX_LEVEL);
    assert_eq!(s.output, 0.0, "never below nothing");
    assert!(!s.charge_auto);
    press(&mut app, SmesOp::Input(Level::Min));
    assert_eq!(unit(&mut app, STORE).1.input, 0.0);
}

#[test]
fn an_empty_smes_switches_its_output_off() {
    let mut app = setup(0);
    let (store, _) = unit(&mut app, STORE);
    {
        let mut s = app.world_mut().get_mut::<Smes>(store).unwrap();
        s.output = 30_000.0;
        s.online = true;
    }
    // It holds nothing, so it goes offline.
    cycles(&mut app, 2);
    assert!(!unit(&mut app, STORE).1.online);
}
