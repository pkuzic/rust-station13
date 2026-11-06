//! Covering open floor: rods build a lattice over it, floor tiles on that make plating, and a
//! welding tool cuts the lattice back into a rod. Spec: docs/reference/floors.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::atmos::StationAtmos;
use rs13_classic::floors::Lattice;
use rs13_classic::hands::{Hands, Stack};
use rs13_classic::objects::{spawn_object, spawn_player_mob, spawn_stack, spawn_station};
use rs13_world::{Appearance, GridPos, MapLayer, ObjectSpec, TileMap, TilePos};

const LOWER: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
    },
    rows: [
        "WWWWW",
        "W...W",
        "WWWWW",
    ],
)"#;

const UPPER: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'o': (tile: (id: "open-space")),
    },
    rows: [
        "WWWWW",
        "W.o.W",
        "WWWWW",
    ],
)"#;

const HOLE: TilePos = TilePos::new(2, 1, 1);
const STAND: TilePos = TilePos::new(1, 1, 1);

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let decks = vec![
        TileMap::from_ron(LOWER, 0).unwrap(),
        TileMap::from_ron(UPPER, 1).unwrap(),
    ];
    let problems = spawn_station(app.world_mut(), decks);
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), STAND);
    tick(&mut app);
    (app, mob)
}

fn tick(app: &mut App) {
    app.world_mut().run_schedule(FixedUpdate);
}

fn act(app: &mut App, mob: Entity, action: Action) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
    tick(app);
}

fn active(app: &App, mob: Entity) -> Option<Entity> {
    app.world().get::<Hands>(mob).unwrap().active_item()
}

/// Put a pile of `amount` in the mob's active hand.
fn give_stack(app: &mut App, mob: Entity, proto: &str, amount: u32) -> Entity {
    let item = spawn_stack(app.world_mut(), proto, STAND, amount).unwrap();
    act(app, mob, Action::UseOn(item));
    assert_eq!(active(app, mob), Some(item));
    item
}

fn drop_held(app: &mut App, mob: Entity) {
    act(app, mob, Action::Drop);
    assert_eq!(active(app, mob), None);
}

fn tile(app: &mut App, pos: TilePos) -> String {
    let mut maps = app.world_mut().query::<&MapLayer>();
    maps.iter(app.world())
        .find(|m| m.0.z == pos.z)
        .unwrap()
        .0
        .tile(pos)
        .unwrap()
        .id
        .clone()
}

fn lattices(app: &mut App) -> Vec<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<Lattice>>();
    q.iter(app.world()).collect()
}

fn told(app: &App, mob: Entity, id: &str) -> bool {
    app.world()
        .get::<Outbox>(mob)
        .unwrap()
        .0
        .iter()
        .any(|m| m.id == id)
}

fn amount(app: &App, stack: Entity) -> Option<u32> {
    app.world().get::<Stack>(stack).map(|s| s.amount)
}

fn same_air(app: &App, a: TilePos, b: TilePos) -> bool {
    let atmos = &app.world().resource::<StationAtmos>().0;
    atmos.zone_at(a).is_some() && atmos.zone_at(a) == atmos.zone_at(b)
}

#[test]
fn rods_build_a_lattice_over_open_floor_and_tiles_floor_it() {
    let (mut app, mob) = setup();
    let below = TilePos::new(HOLE.x, HOLE.y, 0);
    assert!(same_air(&app, HOLE, below), "the hole shares its air");

    let rods = give_stack(&mut app, mob, "rods", 2);
    act(&mut app, mob, Action::UseOnTile(HOLE));
    assert!(told(&app, mob, "lattice-built"));
    assert_eq!(lattices(&mut app).len(), 1);
    assert_eq!(amount(&app, rods), Some(1));
    // A second lattice isn't built on the first.
    act(&mut app, mob, Action::UseOnTile(HOLE));
    assert_eq!(lattices(&mut app).len(), 1);
    assert_eq!(amount(&app, rods), Some(1));
    drop_held(&mut app, mob);

    let tiles = give_stack(&mut app, mob, "floor-tile", 2);
    // Clicking the lattice hands the tiles to the open floor under it.
    let lattice = lattices(&mut app)[0];
    act(&mut app, mob, Action::UseOn(lattice));
    tick(&mut app);
    assert_eq!(tile(&mut app, HOLE), "plating");
    assert!(lattices(&mut app).is_empty(), "the lattice goes into the floor");
    assert_eq!(amount(&app, tiles), Some(1));
    assert!(!same_air(&app, HOLE, below), "the decks' air is apart again");
}

#[test]
fn floor_tiles_on_bare_open_floor_need_support() {
    let (mut app, mob) = setup();
    let tiles = give_stack(&mut app, mob, "floor-tile", 2);
    act(&mut app, mob, Action::UseOnTile(HOLE));
    assert!(told(&app, mob, "lattice-needs-support"));
    assert_eq!(tile(&mut app, HOLE), "open-space");
    assert_eq!(amount(&app, tiles), Some(2));
}

#[test]
fn a_lit_welder_cuts_a_lattice_into_a_rod() {
    let (mut app, mob) = setup();
    give_stack(&mut app, mob, "rods", 1);
    act(&mut app, mob, Action::UseOnTile(HOLE));
    assert_eq!(lattices(&mut app).len(), 1);

    let welder = spawn_object(app.world_mut(), &ObjectSpec::new("welder"), STAND).unwrap();
    act(&mut app, mob, Action::UseOn(welder));
    assert_eq!(active(&app, mob), Some(welder));
    let lattice = lattices(&mut app)[0];
    // Unlit, it does nothing.
    act(&mut app, mob, Action::UseOn(lattice));
    assert_eq!(lattices(&mut app).len(), 1);

    act(&mut app, mob, Action::UseOn(welder));
    act(&mut app, mob, Action::UseOn(lattice));
    tick(&mut app);
    assert!(told(&app, mob, "lattice-sliced"));
    assert!(lattices(&mut app).is_empty());
    let mut q = app.world_mut().query::<(&GridPos, &Appearance)>();
    assert!(
        q.iter(app.world())
            .any(|(p, a)| p.0 == TilePos::new(HOLE.x, HOLE.y, 0) && a.sprite == "rods"),
        "a rod is left, and drops through the hole"
    );
}
