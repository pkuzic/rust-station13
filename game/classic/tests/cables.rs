//! Laying and cutting power cable, and cable coils. Spec: docs/reference/cables.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::hands::{Hands, Stack};
use rs13_classic::movement::Mob;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::power::{Apc, Cable, Powernets, Smes};
use rs13_world::{Dir, GridPos, ObjectSpec, TileMap, TilePos};

/// A SMES and an APC joined by a run of cable over bare plating, with floor tiles on the
/// east side.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        ',': (tile: (id: "plating")),
        'B': (tile: (id: "plating"), objects: ["smes", (id: "cable", ends: [East])]),
        '-': (tile: (id: "plating"), objects: [(id: "cable", ends: [West, East])]),
        'Q': (tile: (id: "plating"), objects: ["apc", (id: "cable", ends: [West])]),
    },
    area_legend: { 'r': "room" },
    area_rows: [
        "       ",
        " rrrrr ",
        " rrrrr ",
        " rrrrr ",
        "       ",
    ],
    rows: [
        "WWWWWWW",
        "WB--Q.W",
        "W,,,,.W",
        "W,,,,.W",
        "WWWWWWW",
    ],
)"#;

const GAP: TilePos = TilePos::new(2, 3, 0);
const BELOW_GAP: TilePos = TilePos::new(2, 2, 0);
const EAST_OF_GAP: TilePos = TilePos::new(3, 3, 0);
const FLOOR: TilePos = TilePos::new(5, 2, 0);

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), BELOW_GAP);
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

fn go(app: &mut App, mob: Entity, to: TilePos) {
    app.world_mut().get_mut::<GridPos>(mob).unwrap().0 = to;
}

/// Put a new item in the mob's active hand.
fn give(app: &mut App, mob: Entity, proto: &str) -> Entity {
    let at = app.world().get::<GridPos>(mob).unwrap().0;
    let item = spawn_object(app.world_mut(), &ObjectSpec::new(proto), at).unwrap();
    act(app, mob, Action::UseOn(item));
    assert_eq!(active(app, mob), Some(item));
    item
}

fn active(app: &App, mob: Entity) -> Option<Entity> {
    app.world().get::<Hands>(mob).unwrap().active_item()
}

fn told(app: &App, mob: Entity) -> Vec<String> {
    app.world()
        .get::<Outbox>(mob)
        .unwrap()
        .0
        .iter()
        .map(|m| m.id.clone())
        .collect()
}

fn cables_at(app: &mut App, pos: TilePos) -> Vec<(Entity, String)> {
    let mut q = app.world_mut().query::<(Entity, &GridPos, &Cable)>();
    q.iter(app.world())
        .filter(|(_, p, _)| p.0 == pos)
        .map(|(e, _, c)| (e, c.state()))
        .collect()
}

fn coils_at(app: &mut App, pos: TilePos) -> Vec<u32> {
    let mut q = app.world_mut().query::<(&GridPos, &Stack)>();
    q.iter(app.world())
        .filter(|(p, _)| p.0 == pos)
        .map(|(_, s)| s.amount)
        .collect()
}

fn amount(app: &App, coil: Entity) -> Option<u32> {
    app.world().get::<Stack>(coil).map(|s| s.amount)
}

/// Whether the SMES and the APC are on one network.
fn connected(app: &mut App) -> bool {
    let smes = app
        .world_mut()
        .query_filtered::<Entity, With<Smes>>()
        .single(app.world())
        .unwrap();
    let apc = app
        .world_mut()
        .query_filtered::<Entity, With<Apc>>()
        .single(app.world())
        .unwrap();
    let nets = app.world().resource::<Powernets>();
    nets.net_of.contains_key(&smes) && nets.net_of.get(&smes) == nets.net_of.get(&apc)
}

#[test]
fn cut_and_mend_a_cable_run() {
    let (mut app, mob) = setup();
    assert!(connected(&mut app), "the run joins the SMES to the APC");

    // Wirecutters take the middle piece out: it drops as a piece of two lengths.
    give(&mut app, mob, "wirecutters");
    let (cable, state) = cables_at(&mut app, GAP).remove(0);
    assert_eq!(state, "4-8");
    act(&mut app, mob, Action::UseOn(cable));
    assert!(cables_at(&mut app, GAP).is_empty());
    assert_eq!(coils_at(&mut app, GAP), [2]);
    assert!(
        told(&app, mob).contains(&"cable-cut".to_owned()),
        "everyone around sees it"
    );
    tick(&mut app);
    assert!(!connected(&mut app), "the APC is cut off");
    act(&mut app, mob, Action::Drop);

    // Standing in the gap facing west, a coil lays a knot pointing west...
    let coil = give(&mut app, mob, "cable-coil");
    go(&mut app, mob, GAP);
    app.world_mut().get_mut::<Mob>(mob).unwrap().facing = Some(Dir::West);
    act(&mut app, mob, Action::UseOnTile(GAP));
    let (knot, state) = cables_at(&mut app, GAP).remove(0);
    assert_eq!(state, "0-8");
    assert_eq!(amount(&app, coil), Some(29));
    tick(&mut app);
    assert!(
        !connected(&mut app),
        "nothing points back at the knot from the east yet"
    );

    // ...and from the east, clicking the knot turns it into a straight run.
    go(&mut app, mob, EAST_OF_GAP);
    act(&mut app, mob, Action::UseOn(knot));
    assert_eq!(cables_at(&mut app, GAP), [(knot, "4-8".to_owned())]);
    assert_eq!(amount(&app, coil), Some(28));
    tick(&mut app);
    assert!(connected(&mut app));
}

#[test]
fn a_cable_that_points_at_you_carries_on_under_your_feet() {
    let (mut app, mob) = setup();
    // A knot on the plating below the run, pointing up at the cable above it...
    let coil = give(&mut app, mob, "cable-coil");
    go(&mut app, mob, TilePos::new(2, 1, 0));
    act(&mut app, mob, Action::UseOnTile(BELOW_GAP));
    let (knot, state) = cables_at(&mut app, BELOW_GAP).remove(0);
    assert_eq!(state, "0-2", "a knot pointing at the mob");

    // Clicked from the tile it points at: the new piece goes on the mob's tile.
    act(&mut app, mob, Action::UseOn(knot));
    assert_eq!(cables_at(&mut app, TilePos::new(2, 1, 0))[0].1, "0-1");
    assert_eq!(amount(&app, coil), Some(28));

    // The same end twice is refused.
    act(&mut app, mob, Action::UseOn(knot));
    assert_eq!(told(&app, mob).last().map(String::as_str), Some("cable-exists"));
    assert_eq!(amount(&app, coil), Some(28));
}

#[test]
fn not_through_floor_tiles_nor_out_of_reach() {
    let (mut app, mob) = setup();
    let coil = give(&mut app, mob, "cable-coil");
    go(&mut app, mob, FLOOR);
    act(&mut app, mob, Action::UseOnTile(FLOOR));
    assert_eq!(
        told(&app, mob).last().map(String::as_str),
        Some("cable-floor-intact")
    );
    act(&mut app, mob, Action::UseOnTile(TilePos::new(1, 1, 0)));
    assert_eq!(told(&app, mob).last().map(String::as_str), Some("cable-too-far"));
    assert!(cables_at(&mut app, FLOOR).is_empty());
    assert_eq!(amount(&app, coil), Some(30));
}

#[test]
fn coils_split_and_join() {
    let (mut app, mob) = setup();
    let coil = spawn_object(app.world_mut(), &ObjectSpec::new("cable-coil"), BELOW_GAP).unwrap();
    act(&mut app, mob, Action::Examine(coil));
    assert_eq!(told(&app, mob).last().map(String::as_str), Some("examine-coil"));

    // Wirecutters snip a piece of one length onto the floor.
    give(&mut app, mob, "wirecutters");
    act(&mut app, mob, Action::UseOn(coil));
    assert_eq!(amount(&app, coil), Some(29));
    let mut lengths = coils_at(&mut app, BELOW_GAP);
    lengths.sort();
    assert_eq!(lengths, [1, 29]);
    act(&mut app, mob, Action::Drop);

    // Picked up, the piece takes the rest of the coil back into itself.
    let mut pieces = app.world_mut().query::<(Entity, &Stack)>();
    let piece = pieces
        .iter(app.world())
        .find(|(_, s)| s.amount == 1)
        .map(|(e, _)| e)
        .unwrap();
    act(&mut app, mob, Action::UseOn(piece));
    act(&mut app, mob, Action::UseOn(coil));
    assert_eq!(amount(&app, piece), Some(30));
    assert!(app.world().get_entity(coil).is_err(), "the other coil is used up");
    assert_eq!(told(&app, mob).last().map(String::as_str), Some("coil-joined"));

    // A full coil takes no more; a part-used one takes what fits.
    let other = spawn_object(app.world_mut(), &ObjectSpec::new("cable-coil"), BELOW_GAP).unwrap();
    act(&mut app, mob, Action::UseOn(other));
    assert_eq!(told(&app, mob).last().map(String::as_str), Some("coil-full"));
    app.world_mut().get_mut::<Stack>(piece).unwrap().amount = 20;
    app.world_mut().get_mut::<Stack>(other).unwrap().amount = 15;
    act(&mut app, mob, Action::UseOn(other));
    assert_eq!((amount(&app, piece), amount(&app, other)), (Some(30), Some(5)));
    assert_eq!(
        told(&app, mob).last().map(String::as_str),
        Some("coil-transferred")
    );
}

#[test]
fn the_last_length_leaves_the_hand_empty() {
    let (mut app, mob) = setup();
    let coil = give(&mut app, mob, "cable-coil");
    app.world_mut().get_mut::<Stack>(coil).unwrap().amount = 1;
    act(&mut app, mob, Action::UseOnTile(TilePos::new(1, 2, 0)));
    assert_eq!(cables_at(&mut app, TilePos::new(1, 2, 0))[0].1, "0-4");
    assert_eq!(active(&app, mob), None);
    assert!(app.world().get_entity(coil).is_err());
}
