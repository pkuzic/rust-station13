//! Power cells and cell chargers. Spec: docs/reference/cells.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::cells::{CHARGE_PER_CYCLE, CHARGER_POWER, CellCharger, PowerCell};
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::machines::MACHINE_TICKS;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::power::{Apc, PowerUser};
use rs13_world::{Appearance, GridPos, ObjectSpec, TileMap, TilePos};

/// A powered room (SMES, cable, APC) with a cell charger.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'B': (tile: (id: "floor"), objects: ["smes", (id: "cable", ends: [East])]),
        'Q': (tile: (id: "floor"), objects: ["apc", (id: "cable", ends: [West])]),
        'C': (tile: (id: "floor"), objects: ["cell-charger"]),
    },
    area_legend: { 'a': "room" },
    area_rows: [
        "     ",
        " aaa ",
        " aaa ",
        "     ",
    ],
    rows: [
        "WWWWW",
        "WBQCW",
        "W...W",
        "WWWWW",
    ],
)"#;

const BESIDE: TilePos = TilePos::new(3, 2, 0);

fn setup() -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), BESIDE);
    ticks(&mut app, 2 * MACHINE_TICKS);
    let mut q = app.world_mut().query_filtered::<Entity, With<CellCharger>>();
    let charger = q.single(app.world()).unwrap();
    (app, mob, charger)
}

fn ticks(app: &mut App, n: u64) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn hold(app: &mut App, mob: Entity, proto: &str) -> Entity {
    let item = spawn_object(app.world_mut(), &ObjectSpec::new(proto), BESIDE).unwrap();
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    app.world_mut().get_mut::<Hands>(mob).unwrap().right = Some(item);
    item
}

fn click(app: &mut App, mob: Entity, target: Entity) -> Option<String> {
    let before = app.world().get::<Outbox>(mob).unwrap().0.len();
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    ticks(app, 1);
    let outbox = &app.world().get::<Outbox>(mob).unwrap().0;
    (outbox.len() > before).then(|| outbox[before].id.clone())
}

fn cell(app: &App, entity: Entity) -> PowerCell {
    *app.world().get::<PowerCell>(entity).unwrap()
}

fn look(app: &App, entity: Entity) -> String {
    app.world().get::<Appearance>(entity).unwrap().state.clone()
}

#[test]
fn cells_come_full_and_show_it() {
    let (mut app, mob, _) = setup();
    let small = hold(&mut app, mob, "power-cell");
    let big = hold(&mut app, mob, "high-power-cell");
    ticks(&mut app, 1);
    assert_eq!(cell(&app, small).capacity, 2.5e6, "Luna: 2 500 units");
    assert_eq!(cell(&app, big).capacity, 1.0e7, "Luna: 10 000 units");
    assert_eq!(cell(&app, small).charge, cell(&app, small).capacity);
    assert_eq!(look(&app, small), "+cell-o2");
    app.world_mut().get_mut::<PowerCell>(small).unwrap().charge = 1.0e6;
    ticks(&mut app, 1);
    assert_eq!(look(&app, small), "+cell-o1");
    app.world_mut().get_mut::<PowerCell>(small).unwrap().charge = 0.0;
    ticks(&mut app, 1);
    assert_eq!(look(&app, small), "");
}

#[test]
fn a_charger_charges_a_cell_and_hands_it_back() {
    let (mut app, mob, charger) = setup();
    let item = hold(&mut app, mob, "power-cell");
    app.world_mut().get_mut::<PowerCell>(item).unwrap().charge = 0.0;
    assert_eq!(
        click(&mut app, mob, charger).as_deref(),
        Some("charger-inserted-self")
    );
    assert_eq!(app.world().get::<Hands>(mob).unwrap().right, None);
    assert_eq!(
        app.world().get::<CellCharger>(charger).unwrap().charging,
        Some(item)
    );
    assert_eq!(look(&app, charger), "ccharger1+ccharger-o0");

    ticks(&mut app, 3 * MACHINE_TICKS);
    let charge = cell(&app, item).charge;
    assert!(
        (2.0 * CHARGE_PER_CYCLE..=4.0 * CHARGE_PER_CYCLE).contains(&charge),
        "175 units a cycle: {charge}"
    );
    assert_eq!(
        app.world().get::<PowerUser>(charger).unwrap().watts,
        CHARGER_POWER
    );

    // Full after about 15 cycles, and no more.
    ticks(&mut app, 20 * MACHINE_TICKS);
    assert_eq!(cell(&app, item).charge, cell(&app, item).capacity);
    assert_eq!(look(&app, charger), "ccharger1+ccharger-o4");

    assert_eq!(
        click(&mut app, mob, charger).as_deref(),
        Some("charger-removed-self")
    );
    assert_eq!(app.world().get::<Hands>(mob).unwrap().right, Some(item));
    assert_eq!(app.world().get::<HeldBy>(item), Some(&HeldBy(mob)));
    ticks(&mut app, 1);
    assert_eq!(look(&app, charger), "ccharger0");
    assert_eq!(app.world().get::<PowerUser>(charger).unwrap().watts, 0.0);
}

#[test]
fn one_cell_at_a_time_and_only_with_power() {
    let (mut app, mob, charger) = setup();
    hold(&mut app, mob, "power-cell");
    click(&mut app, mob, charger);
    hold(&mut app, mob, "power-cell");
    assert_eq!(click(&mut app, mob, charger).as_deref(), Some("charger-occupied"));

    // Take the first back out, cut the power: the charger won't take a cell.
    app.world_mut().get_mut::<Hands>(mob).unwrap().right = None;
    click(&mut app, mob, charger);
    let mut apcs = app.world_mut().query::<&mut Apc>();
    for mut apc in apcs.iter_mut(app.world_mut()) {
        apc.operating = false;
    }
    ticks(&mut app, 2 * MACHINE_TICKS);
    assert_eq!(click(&mut app, mob, charger).as_deref(), Some("charger-blinks"));
}

#[test]
fn a_wrench_frees_an_empty_charger_only() {
    let (mut app, mob, charger) = setup();
    hold(&mut app, mob, "power-cell");
    click(&mut app, mob, charger);
    hold(&mut app, mob, "wrench");
    assert_eq!(
        click(&mut app, mob, charger).as_deref(),
        Some("charger-remove-cell-first")
    );
    app.world_mut().get_mut::<CellCharger>(charger).unwrap().charging = None;
    assert_eq!(click(&mut app, mob, charger).as_deref(), Some("charger-detached"));
    assert!(!app.world().get::<CellCharger>(charger).unwrap().anchored);
    // A loose charger takes no cell.
    hold(&mut app, mob, "power-cell");
    assert_eq!(click(&mut app, mob, charger), None);
}
