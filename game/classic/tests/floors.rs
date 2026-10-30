//! Prying up floor tiles and putting them back, and what they hide.
//! Spec: docs/reference/floors.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::hands::{Hands, Stack};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob, spawn_stack};
use rs13_classic::power::Cable;
use rs13_world::{Appearance, GridPos, Hidden, MapLayer, ObjectSpec, TileMap, TilePos};

/// Floor tiles over a cable and a pipe, and a pipe laid over the floor tiles.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        ',': (tile: (id: "plating")),
        '-': (tile: (id: "floor"), objects: [(id: "cable", ends: [West, East])]),
        'p': (tile: (id: "floor"), objects: [(id: "pipe", dir: North)]),
        'o': (tile: (id: "floor"), objects: [(id: "pipe", dir: North, above_floor: true)]),
    },
    rows: [
        "WWWWW",
        "W-poW",
        "W..,W",
        "WWWWW",
    ],
)"#;

const CABLE: TilePos = TilePos::new(1, 2, 0);
const PIPE: TilePos = TilePos::new(2, 2, 0);
const PIPE_OVER: TilePos = TilePos::new(3, 2, 0);
const STAND: TilePos = TilePos::new(2, 1, 0);

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
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

fn tile(app: &mut App, pos: TilePos) -> String {
    let mut maps = app.world_mut().query::<&MapLayer>();
    maps.single(app.world()).unwrap().0.tile(pos).unwrap().id.clone()
}

/// The thing with this sprite on `pos`, and whether it is hidden.
fn thing_at(app: &mut App, pos: TilePos, sprite: &str) -> (Entity, bool) {
    let mut q = app
        .world_mut()
        .query::<(Entity, &GridPos, &Appearance, Has<Hidden>)>();
    q.iter(app.world())
        .find(|(_, p, a, _)| p.0 == pos && a.sprite == sprite)
        .map(|(e, _, _, h)| (e, h))
        .unwrap()
}

fn tiles_at(app: &mut App, pos: TilePos) -> Vec<u32> {
    let mut q = app.world_mut().query::<(&GridPos, &Appearance, &Stack)>();
    q.iter(app.world())
        .filter(|(p, a, _)| p.0 == pos && a.sprite == "floor-tile")
        .map(|(_, _, s)| s.amount)
        .collect()
}

#[test]
fn floor_tiles_hide_what_is_laid_under_them() {
    let (mut app, _) = setup();
    assert!(thing_at(&mut app, CABLE, "cable").1);
    assert!(thing_at(&mut app, PIPE, "pipe").1);
    assert!(
        !thing_at(&mut app, PIPE_OVER, "pipe").1,
        "a pipe laid over the floor tiles shows"
    );
}

#[test]
fn a_crowbar_pries_up_floor_tiles_and_they_go_back() {
    let (mut app, mob) = setup();
    give(&mut app, mob, "crowbar");
    act(&mut app, mob, Action::UseOnTile(CABLE));
    assert_eq!(tile(&mut app, CABLE), "plating");
    assert_eq!(tiles_at(&mut app, CABLE), [1], "one floor tile drops");
    tick(&mut app);
    assert!(!thing_at(&mut app, CABLE, "cable").1, "the cable shows now");

    // Plating stays plating; out of reach does nothing.
    act(&mut app, mob, Action::UseOnTile(CABLE));
    assert_eq!(tiles_at(&mut app, CABLE), [1]);
    act(&mut app, mob, Action::UseOnTile(TilePos::new(3, 3, 0)));
    act(&mut app, mob, Action::Drop);

    // The floor tile goes back, used up, and the cable is hidden again.
    let (pile, _) = thing_at(&mut app, CABLE, "floor-tile");
    act(&mut app, mob, Action::UseOn(pile));
    assert_eq!(active(&app, mob), Some(pile));
    act(&mut app, mob, Action::UseOnTile(CABLE));
    assert_eq!(tile(&mut app, CABLE), "floor");
    assert_eq!(active(&app, mob), None);
    assert!(app.world().get_entity(pile).is_err());
    tick(&mut app);
    assert!(thing_at(&mut app, CABLE, "cable").1);
}

#[test]
fn hidden_things_are_out_of_reach() {
    let (mut app, mob) = setup();
    give(&mut app, mob, "wirecutters");
    let (cable, _) = thing_at(&mut app, CABLE, "cable");
    app.world_mut().get_mut::<Outbox>(mob).unwrap().0.clear();
    act(&mut app, mob, Action::UseOn(cable));
    act(&mut app, mob, Action::Examine(cable));
    assert!(app.world().get::<Cable>(cable).is_some(), "not cut");
    assert!(app.world().get::<Outbox>(mob).unwrap().0.is_empty());

    // A coil can't reach it either: no cable under floor tiles.
    act(&mut app, mob, Action::Drop);
    give(&mut app, mob, "cable-coil");
    act(&mut app, mob, Action::UseOnTile(PIPE));
    assert_eq!(
        app.world()
            .get::<Outbox>(mob)
            .unwrap()
            .0
            .last()
            .map(|m| m.id.as_str()),
        Some("cable-floor-intact")
    );
}

#[test]
fn floor_tiles_pile_up() {
    let (mut app, mob) = setup();
    let mine = spawn_stack(app.world_mut(), "floor-tile", STAND, 10).unwrap();
    let other = spawn_stack(app.world_mut(), "floor-tile", STAND, 55).unwrap();
    act(&mut app, mob, Action::Examine(other));
    assert_eq!(
        app.world()
            .get::<Outbox>(mob)
            .unwrap()
            .0
            .last()
            .map(|m| m.id.as_str()),
        Some("examine-floor-tiles")
    );
    act(&mut app, mob, Action::UseOn(mine));
    act(&mut app, mob, Action::UseOn(other));
    let amount = |app: &App, e| app.world().get::<Stack>(e).map(|s| s.amount);
    assert_eq!((amount(&app, mine), amount(&app, other)), (Some(60), Some(5)));

    // From the other hand, one at a time.
    act(&mut app, mob, Action::SwapHands);
    act(&mut app, mob, Action::UseOn(other));
    act(&mut app, mob, Action::SwapHands);
    app.world_mut().get_mut::<Stack>(mine).unwrap().amount = 50;
    act(&mut app, mob, Action::UseOn(other));
    assert_eq!((amount(&app, mine), amount(&app, other)), (Some(51), Some(4)));
}
