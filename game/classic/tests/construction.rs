//! Building by hand. Spec: docs/reference/construction.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::atmos::StationAtmos;
use rs13_classic::construction::{BuildPanelOut, Girder, METAL, PendingBuildOps, RECIPES, RODS};
use rs13_classic::hands::{Hands, HeldBy, Stack};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob, spawn_stack};
use rs13_classic::welding::Welder;
use rs13_classic::windows::WindowPane;
use rs13_simulation::atmos::Cell;
use rs13_world::{Appearance, GridPos, MapLayer, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: { 'W': (tile: (id: "wall", solid: true)), '.': (tile: (id: "floor")) },
    rows: ["WWWWWWW", "W.....W", "W.....W", "WWWWWWW"],
)"#;
const AT: TilePos = TilePos::new(2, 1, 0);
const WALL: TilePos = TilePos::new(2, 0, 0);

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), AT);
    app.world_mut().run_schedule(FixedUpdate);
    (app, mob)
}

fn act(app: &mut App, mob: Entity, action: Action) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
    app.world_mut().run_schedule(FixedUpdate);
}

fn seconds(app: &mut App, n: f32) {
    for _ in 0..(n * 20.0) as u32 {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn hold(app: &mut App, mob: Entity, item: Entity) {
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    let mut hands = app.world_mut().get_mut::<Hands>(mob).unwrap();
    let active = hands.active;
    *hands.slot_mut(active) = Some(item);
}

fn hold_stack(app: &mut App, mob: Entity, proto: &str, amount: u32) -> Entity {
    let pile = spawn_stack(app.world_mut(), proto, AT, amount).unwrap();
    hold(app, mob, pile);
    pile
}

fn hold_new(app: &mut App, mob: Entity, proto: &str) -> Entity {
    let item = spawn_object(app.world_mut(), &ObjectSpec::new(proto), AT).unwrap();
    hold(app, mob, item);
    item
}

fn tile(app: &mut App, at: TilePos) -> (String, bool) {
    let mut maps = app.world_mut().query::<&MapLayer>();
    let map = maps.single(app.world()).unwrap();
    let t = map.0.tile(at).unwrap();
    (t.id.clone(), t.solid)
}

fn line(proto: &str) -> usize {
    RECIPES.iter().position(|(p, ..)| *p == proto).unwrap()
}

fn things_here(app: &mut App, proto: &str, at: TilePos) -> Vec<Entity> {
    let mut q = app.world_mut().query::<(Entity, &GridPos, &Appearance)>();
    q.iter(app.world())
        .filter(|(_, p, a)| p.0 == at && a.sprite == proto)
        .map(|(e, ..)| e)
        .collect()
}

#[test]
fn every_recipe_makes_something_that_exists() {
    let (mut app, _) = setup();
    for (proto, ..) in RECIPES {
        assert!(
            spawn_object(app.world_mut(), &ObjectSpec::new(proto), AT).is_ok(),
            "{proto}"
        );
    }
}

#[test]
fn metal_opens_a_list_and_makes_things() {
    let (mut app, mob) = setup();
    let pile = hold_stack(&mut app, mob, METAL, 10);
    act(&mut app, mob, Action::UseOn(pile));
    app.world_mut().run_schedule(FixedUpdate);
    let view = app.world().get::<BuildPanelOut>(mob).unwrap().0.clone().unwrap();
    assert_eq!(view.amount, 10);

    app.world_mut()
        .get_mut::<PendingBuildOps>(mob)
        .unwrap()
        .0
        .push((pile, line("floor-tile")));
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(app.world().get::<Stack>(pile).unwrap().amount, 9);
    let tiles = things_here(&mut app, "floor-tile", AT);
    assert_eq!(app.world().get::<Stack>(tiles[0]).unwrap().amount, 4);
}

#[test]
fn a_wall_goes_up_on_a_girder_and_comes_down_with_a_welder() {
    let (mut app, mob) = setup();
    let pile = hold_stack(&mut app, mob, METAL, 10);
    app.world_mut()
        .get_mut::<PendingBuildOps>(mob)
        .unwrap()
        .0
        .push((pile, line("girder")));
    seconds(&mut app, 1.0);
    assert!(
        things_here(&mut app, "girder", AT).is_empty(),
        "that takes a while"
    );
    seconds(&mut app, 5.0);
    let girder = things_here(&mut app, "girder", AT)[0];
    assert_eq!(app.world().get::<Stack>(pile).unwrap().amount, 8);

    // Moved off it, as you have to be to build the wall there.
    app.world_mut().get_mut::<GridPos>(mob).unwrap().0 = TilePos::new(3, 1, 0);
    act(&mut app, mob, Action::UseOn(girder));
    seconds(&mut app, 5.0);
    assert_eq!(tile(&mut app, AT), ("wall".to_owned(), true));
    assert!(app.world().get_entity(girder).is_err());
    assert_eq!(app.world().resource::<StationAtmos>().0.cell(AT), Cell::Solid);
    assert_eq!(app.world().get::<Stack>(pile).unwrap().amount, 6);

    let welder = hold_new(&mut app, mob, "welder");
    app.world_mut().get_mut::<Welder>(welder).unwrap().lit = true;
    app.world_mut().get_mut::<GridPos>(mob).unwrap().0 = TilePos::new(3, 1, 0);
    act(&mut app, mob, Action::UseOnTile(AT));
    seconds(&mut app, 11.0);
    assert_eq!(tile(&mut app, AT).0, "plating");
    assert_eq!(things_here(&mut app, "girder", AT).len(), 1);
    assert_eq!(app.world().resource::<StationAtmos>().0.cell(AT), Cell::Open);
    assert_eq!(things_here(&mut app, METAL, AT).len(), 1);
}

#[test]
fn walking_off_stops_the_work() {
    let (mut app, mob) = setup();
    let welder = hold_new(&mut app, mob, "welder");
    app.world_mut().get_mut::<Welder>(welder).unwrap().lit = true;
    act(&mut app, mob, Action::UseOnTile(WALL));
    app.world_mut().get_mut::<GridPos>(mob).unwrap().0 = TilePos::new(3, 1, 0);
    seconds(&mut app, 11.0);
    assert_eq!(tile(&mut app, WALL).0, "wall");
}

#[test]
fn rods_make_a_grille_that_lets_air_through_and_wirecutters_undo_it() {
    let (mut app, mob) = setup();
    let rods = hold_stack(&mut app, mob, RODS, 4);
    act(&mut app, mob, Action::UseOn(rods));
    assert_eq!(tile(&mut app, AT), ("grille".to_owned(), true));
    assert_eq!(app.world().resource::<StationAtmos>().0.cell(AT), Cell::Open);
    assert_eq!(app.world().get::<Stack>(rods).unwrap().amount, 2);

    app.world_mut().get_mut::<GridPos>(mob).unwrap().0 = TilePos::new(3, 1, 0);
    hold_new(&mut app, mob, "wirecutters");
    act(&mut app, mob, Action::UseOnTile(AT));
    assert_eq!(tile(&mut app, AT).0, "plating");
    assert_eq!(things_here(&mut app, RODS, AT).len(), 1);
}

#[test]
fn glass_makes_a_pane_on_a_free_side_and_a_wrench_takes_it_back() {
    let (mut app, mob) = setup();
    let glass = hold_stack(&mut app, mob, "glass-sheet", 5);
    for _ in 0..5 {
        act(&mut app, mob, Action::UseOn(glass));
    }
    let mut panes = app.world_mut().query::<(Entity, &WindowPane)>();
    let all: Vec<_> = panes.iter(app.world()).map(|(e, w)| (e, w.dir)).collect();
    assert_eq!(all.len(), 4, "one to each side");
    let dirs: std::collections::HashSet<_> = all.iter().map(|(_, d)| format!("{d:?}")).collect();
    assert_eq!(dirs.len(), 4);
    assert_eq!(
        app.world().get::<Stack>(glass).unwrap().amount,
        1,
        "the fifth wasn't used"
    );

    hold_new(&mut app, mob, "wrench");
    act(&mut app, mob, Action::UseOn(all[0].0));
    assert!(app.world().get_entity(all[0].0).is_err());
}

#[test]
fn a_reinforced_wall_takes_plasteel_twice() {
    let (mut app, mob) = setup();
    let there = TilePos::new(3, 1, 0);
    let girder = spawn_object(app.world_mut(), &ObjectSpec::new("girder"), there).unwrap();
    assert!(!app.world().get::<Girder>(girder).unwrap().reinforced);
    let plasteel = hold_stack(&mut app, mob, "plasteel-sheet", 2);
    act(&mut app, mob, Action::UseOn(girder));
    seconds(&mut app, 7.0);
    assert!(app.world().get_entity(girder).is_err());
    let reinforced = things_here(&mut app, "girder-reinforced", there)[0];
    act(&mut app, mob, Action::UseOn(reinforced));
    seconds(&mut app, 6.0);
    assert_eq!(tile(&mut app, there), ("r-wall".to_owned(), true));
    assert!(app.world().get_entity(plasteel).is_err(), "used up");
}
