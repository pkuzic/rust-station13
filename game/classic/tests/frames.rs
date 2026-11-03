//! Computer frames. Spec: docs/reference/construction.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::frames::{BOARDS, ComputerFrame};
use rs13_classic::hands::{Hands, HeldBy, Stack};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob, spawn_stack};
use rs13_world::{Appearance, GridPos, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: { 'W': (tile: (id: "wall", solid: true)), '.': (tile: (id: "floor")) },
    rows: ["WWWWWW", "W....W", "WWWWWW"],
)"#;
const NEAR: TilePos = TilePos::new(1, 1, 0);
const AT: TilePos = TilePos::new(2, 1, 0);

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), NEAR);
    app.world_mut().run_schedule(FixedUpdate);
    (app, mob)
}

fn hold(app: &mut App, mob: Entity, proto: &str) -> Entity {
    let item = spawn_stack(app.world_mut(), proto, NEAR, 50).unwrap();
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    let mut hands = app.world_mut().get_mut::<Hands>(mob).unwrap();
    let active = hands.active;
    if let Some(old) = *hands.slot_mut(active) {
        app.world_mut().entity_mut(old).despawn();
    }
    let mut hands = app.world_mut().get_mut::<Hands>(mob).unwrap();
    let active = hands.active;
    *hands.slot_mut(active) = Some(item);
    item
}

fn click(app: &mut App, mob: Entity, target: Entity) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    app.world_mut().run_schedule(FixedUpdate);
}

fn seconds(app: &mut App, n: f32) {
    for _ in 0..(n * 20.0) as u32 {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn state(app: &App, frame: Entity) -> u8 {
    app.world().get::<ComputerFrame>(frame).unwrap().state
}

fn found(app: &mut App, proto: &str) -> Vec<Entity> {
    let mut q = app.world_mut().query::<(Entity, &GridPos, &Appearance)>();
    q.iter(app.world())
        .filter(|(_, p, a)| p.0 == AT && a.sprite == proto)
        .map(|(e, ..)| e)
        .collect()
}

fn built_to_glass(app: &mut App, mob: Entity) -> Entity {
    let frame = spawn_object(app.world_mut(), &ObjectSpec::new("computer-frame"), AT).unwrap();
    hold(app, mob, "wrench");
    click(app, mob, frame);
    seconds(app, 2.5);
    assert_eq!(state(app, frame), 1, "fastened");
    hold(app, mob, "board-arcade");
    click(app, mob, frame);
    assert!(app.world().get::<ComputerFrame>(frame).unwrap().board.is_some());
    assert!(app.world().get::<Hands>(mob).unwrap().active_item().is_none());
    hold(app, mob, "screwdriver");
    click(app, mob, frame);
    assert_eq!(state(app, frame), 2);
    let coil = hold(app, mob, "cable-coil");
    let before = app.world().get::<Stack>(coil).unwrap().amount;
    click(app, mob, frame);
    assert_eq!(state(app, frame), 3);
    assert_eq!(app.world().get::<Stack>(coil).unwrap().amount, before - 5);
    let glass = hold(app, mob, "glass-sheet");
    let before = app.world().get::<Stack>(glass).unwrap().amount;
    click(app, mob, frame);
    assert_eq!(state(app, frame), 4);
    assert_eq!(app.world().get::<Stack>(glass).unwrap().amount, before - 2);
    frame
}

#[test]
fn every_board_is_for_a_computer_that_exists() {
    let (mut app, _) = setup();
    for (board, computer) in BOARDS {
        assert!(
            spawn_object(app.world_mut(), &ObjectSpec::new(board), AT).is_ok(),
            "{board}"
        );
        assert!(
            spawn_object(app.world_mut(), &ObjectSpec::new(computer), AT).is_ok(),
            "{computer}"
        );
    }
}

#[test]
fn a_frame_goes_together_in_order_and_makes_the_computer() {
    let (mut app, mob) = setup();
    let frame = built_to_glass(&mut app, mob);
    hold(&mut app, mob, "screwdriver");
    click(&mut app, mob, frame);
    assert!(app.world().get_entity(frame).is_err());
    assert_eq!(found(&mut app, "arcade").len(), 1);
}

#[test]
fn each_step_needs_the_one_before() {
    let (mut app, mob) = setup();
    let frame = spawn_object(app.world_mut(), &ObjectSpec::new("computer-frame"), AT).unwrap();
    hold(&mut app, mob, "cable-coil");
    click(&mut app, mob, frame);
    assert_eq!(state(&app, frame), 0, "no cables on a loose frame");
    hold(&mut app, mob, "board-arcade");
    click(&mut app, mob, frame);
    assert!(
        app.world().get::<ComputerFrame>(frame).unwrap().board.is_none(),
        "not till it's fastened"
    );
}

#[test]
fn a_computer_comes_back_to_its_frame_with_its_board() {
    let (mut app, mob) = setup();
    let arcade = spawn_object(app.world_mut(), &ObjectSpec::new("arcade"), AT).unwrap();
    hold(&mut app, mob, "screwdriver");
    click(&mut app, mob, arcade);
    seconds(&mut app, 2.5);
    assert!(app.world().get_entity(arcade).is_err());
    let frame = found(&mut app, "computer-frame")[0];
    assert_eq!(state(&app, frame), 4);

    // And all the way down.
    hold(&mut app, mob, "crowbar");
    click(&mut app, mob, frame);
    assert_eq!(state(&app, frame), 3);
    assert_eq!(found(&mut app, "glass-sheet").len(), 1);
    hold(&mut app, mob, "wirecutters");
    click(&mut app, mob, frame);
    assert_eq!(state(&app, frame), 2);
    hold(&mut app, mob, "screwdriver");
    click(&mut app, mob, frame);
    assert_eq!(state(&app, frame), 1);
    hold(&mut app, mob, "crowbar");
    click(&mut app, mob, frame);
    assert_eq!(found(&mut app, "board-arcade").len(), 1, "the board comes out");
    hold(&mut app, mob, "wrench");
    click(&mut app, mob, frame);
    seconds(&mut app, 2.5);
    assert_eq!(state(&app, frame), 0);
}
