//! P.A.C.M.A.N. portable generators. Spec: docs/reference/generators.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::generators::{CYCLES_PER_SHEET, PACMAN_MAX_SHEETS, Pacman};
use rs13_classic::hands::{Hands, HeldBy, Stack};
use rs13_classic::machines::MACHINE_TICKS;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::power::{Apc, PowerSupply};
use rs13_world::{Appearance, GridPos, ObjectSpec, TileMap, TilePos};

/// A room with an APC wired only to a knot under a generator: no SMES, so the generator
/// is the only source.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'G': (tile: (id: "floor"), objects: ["pacman", (id: "cable", ends: [East])]),
        'Q': (tile: (id: "floor"), objects: ["apc", (id: "cable", ends: [West])]),
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
        "WGQ.W",
        "W...W",
        "WWWWW",
    ],
)"#;

const BESIDE: TilePos = TilePos::new(2, 2, 0);

fn setup() -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), BESIDE);
    ticks(&mut app, 1);
    let mut q = app.world_mut().query_filtered::<Entity, With<Pacman>>();
    let generator = q.single(app.world()).unwrap();
    (app, mob, generator)
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

fn empty_hands(app: &mut App, mob: Entity) {
    let mut hands = app.world_mut().get_mut::<Hands>(mob).unwrap();
    hands.left = None;
    hands.right = None;
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

fn generator(app: &App, entity: Entity) -> Pacman {
    *app.world().get::<Pacman>(entity).unwrap()
}

fn cell(app: &mut App) -> f64 {
    let mut q = app.world_mut().query::<&Apc>();
    q.single(app.world()).unwrap().cell.unwrap().0
}

/// Fuel it, wrench it down and switch it on.
fn start(app: &mut App, mob: Entity, generator: Entity) {
    hold(app, mob, "plasma-sheet");
    assert_eq!(click(app, mob, generator).as_deref(), Some("pacman-fuel-added"));
    hold(app, mob, "wrench");
    assert_eq!(click(app, mob, generator).as_deref(), Some("pacman-secured"));
    empty_hands(app, mob);
    assert_eq!(click(app, mob, generator).as_deref(), Some("pacman-on"));
}

#[test]
fn it_starts_empty_and_unsecured_and_wont_run_like_that() {
    let (mut app, mob, generator_id) = setup();
    assert_eq!(
        generator(&app, generator_id),
        Pacman {
            sheets: 0,
            burning: 0,
            active: false,
            anchored: false
        }
    );
    // An unsecured generator has no panel to work.
    assert_eq!(click(&mut app, mob, generator_id), None);
    hold(&mut app, mob, "wrench");
    click(&mut app, mob, generator_id);
    empty_hands(&mut app, mob);
    assert_eq!(
        click(&mut app, mob, generator_id).as_deref(),
        Some("pacman-no-fuel")
    );
}

#[test]
fn plasma_sheets_fill_the_hopper_up_to_fifty() {
    let (mut app, mob, generator_id) = setup();
    let sheets = hold(&mut app, mob, "plasma-sheet");
    app.world_mut().get_mut::<Stack>(sheets).unwrap().amount = 30;
    click(&mut app, mob, generator_id);
    assert_eq!(generator(&app, generator_id).sheets, 30);
    assert!(app.world().get_entity(sheets).is_err(), "the whole stack went in");
    assert_eq!(app.world().get::<Hands>(mob).unwrap().right, None);

    let more = hold(&mut app, mob, "plasma-sheet");
    click(&mut app, mob, generator_id);
    assert_eq!(generator(&app, generator_id).sheets, PACMAN_MAX_SHEETS);
    assert_eq!(
        app.world().get::<Stack>(more).unwrap().amount,
        30,
        "20 of 50 went in"
    );
    assert_eq!(click(&mut app, mob, generator_id).as_deref(), Some("pacman-full"));
}

#[test]
fn a_running_generator_powers_the_network_and_burns_a_sheet_every_hundred_cycles() {
    let (mut app, mob, generator_id) = setup();
    // Without a source the APC's cell can't charge (and nothing here draws power).
    ticks(&mut app, 2 * MACHINE_TICKS);
    let before = cell(&mut app);
    ticks(&mut app, 4 * MACHINE_TICKS);
    assert_eq!(cell(&mut app), before, "nothing feeds the APC yet");

    start(&mut app, mob, generator_id);
    ticks(&mut app, 2 * MACHINE_TICKS);
    assert_eq!(
        app.world().get::<Appearance>(generator_id).unwrap().state,
        "portgen1"
    );
    assert_eq!(
        app.world().get::<PowerSupply>(generator_id).unwrap().watts,
        5_000.0
    );
    let before = cell(&mut app);
    ticks(&mut app, 4 * MACHINE_TICKS);
    assert!(
        cell(&mut app) > before,
        "the APC trickle-charges from the generator"
    );

    let g = generator(&app, generator_id);
    assert_eq!(g.sheets, 49, "one sheet is burning");
    let cycles = u64::from(g.burning) + 1;
    ticks(&mut app, cycles * MACHINE_TICKS);
    let g = generator(&app, generator_id);
    assert_eq!(
        (g.sheets, g.burning),
        (48, CYCLES_PER_SHEET - 1),
        "the next sheet"
    );
}

#[test]
fn it_stops_when_switched_off_or_out_of_fuel_and_only_unwrenches_when_off() {
    let (mut app, mob, generator_id) = setup();
    start(&mut app, mob, generator_id);
    ticks(&mut app, MACHINE_TICKS);

    hold(&mut app, mob, "wrench");
    assert_eq!(click(&mut app, mob, generator_id), None, "not while it runs");
    assert!(generator(&app, generator_id).anchored);

    empty_hands(&mut app, mob);
    assert_eq!(click(&mut app, mob, generator_id).as_deref(), Some("pacman-off"));
    ticks(&mut app, MACHINE_TICKS);
    assert_eq!(app.world().get::<PowerSupply>(generator_id).unwrap().watts, 0.0);
    assert_eq!(
        app.world().get::<Appearance>(generator_id).unwrap().state,
        "portgen0"
    );

    // Out of fuel, it switches itself off.
    app.world_mut().get_mut::<Pacman>(generator_id).unwrap().sheets = 0;
    app.world_mut().get_mut::<Pacman>(generator_id).unwrap().burning = 1;
    click(&mut app, mob, generator_id);
    ticks(&mut app, 2 * MACHINE_TICKS);
    assert!(!generator(&app, generator_id).active);
}
