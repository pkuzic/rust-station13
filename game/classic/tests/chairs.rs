//! Chairs. Spec: docs/reference/chairs.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::chairs::{Buckled, Chair};
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::movement::MoveIntent;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_world::{Appearance, Dir, GridPos, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'c': (tile: (id: "floor"), objects: [(id: "chair", dir: East)]),
    },
    rows: [
        "WWWWW",
        "W.c.W",
        "W...W",
        "WWWWW",
    ],
)"#;

const CHAIR_AT: TilePos = TilePos::new(2, 2, 0);

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().run_schedule(FixedUpdate);
    let mut q = app.world_mut().query_filtered::<Entity, With<Chair>>();
    let chair = q.single(app.world()).unwrap();
    (app, chair)
}

fn ticks(app: &mut App, n: u64) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn click(app: &mut App, mob: Entity, target: Entity) -> Vec<String> {
    let before = app.world().get::<Outbox>(mob).unwrap().0.len();
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    ticks(app, 1);
    app.world().get::<Outbox>(mob).unwrap().0[before..]
        .iter()
        .map(|m| m.id.clone())
        .collect()
}

fn chair(app: &App, entity: Entity) -> Chair {
    *app.world().get::<Chair>(entity).unwrap()
}

#[test]
fn sitting_on_a_chair_buckles_you_in_and_holds_you_there() {
    let (mut app, chair_id) = setup();
    let mob = spawn_player_mob(app.world_mut(), CHAIR_AT);
    ticks(&mut app, 1);
    assert_eq!(app.world().get::<Appearance>(chair_id).unwrap().state, "east");

    let said = click(&mut app, mob, chair_id);
    assert!(said.contains(&"chair-buckled-self".to_owned()), "{said:?}");
    assert_eq!(chair(&app, chair_id).buckled, Some(mob));
    assert_eq!(app.world().get::<Buckled>(mob), Some(&Buckled(chair_id)));

    // Can't walk off while buckled.
    app.world_mut().get_mut::<MoveIntent>(mob).unwrap().dir = Some(Dir::South);
    ticks(&mut app, 10);
    assert_eq!(app.world().get::<GridPos>(mob).unwrap().0, CHAIR_AT);

    // Click again to get out, then walk.
    app.world_mut().get_mut::<MoveIntent>(mob).unwrap().dir = None;
    let said = click(&mut app, mob, chair_id);
    assert!(said.contains(&"chair-unbuckled-self".to_owned()), "{said:?}");
    assert!(app.world().get::<Buckled>(mob).is_none());
    app.world_mut().get_mut::<MoveIntent>(mob).unwrap().dir = Some(Dir::South);
    ticks(&mut app, 2);
    assert_eq!(
        app.world().get::<GridPos>(mob).unwrap().0,
        CHAIR_AT.step(Dir::South)
    );
}

#[test]
fn someone_beside_it_can_unbuckle_you_or_turn_an_empty_chair() {
    let (mut app, chair_id) = setup();
    let sitter = spawn_player_mob(app.world_mut(), CHAIR_AT);
    let helper = spawn_player_mob(app.world_mut(), TilePos::new(1, 2, 0));
    ticks(&mut app, 1);
    click(&mut app, sitter, chair_id);
    let seen = click(&mut app, helper, chair_id);
    assert!(seen.contains(&"chair-unbuckled-by".to_owned()), "{seen:?}");
    assert_eq!(chair(&app, chair_id).buckled, None);
    assert!(app.world().get::<Buckled>(sitter).is_none());

    // Empty now: a click from beside it turns it a quarter, anticlockwise (BYOND `turn`).
    click(&mut app, helper, chair_id);
    assert_eq!(chair(&app, chair_id).dir, Dir::North);
    ticks(&mut app, 1);
    assert_eq!(app.world().get::<Appearance>(chair_id).unwrap().state, "north");
}

#[test]
fn a_wrench_takes_it_apart_into_a_metal_sheet() {
    let (mut app, chair_id) = setup();
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(1, 2, 0));
    let wrench = spawn_object(app.world_mut(), &ObjectSpec::new("wrench"), CHAIR_AT).unwrap();
    app.world_mut()
        .entity_mut(wrench)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    app.world_mut().get_mut::<Hands>(mob).unwrap().right = Some(wrench);
    click(&mut app, mob, chair_id);
    ticks(&mut app, 1);
    assert!(app.world().get_entity(chair_id).is_err());
    let mut q = app.world_mut().query::<(&GridPos, &Appearance)>();
    assert!(
        q.iter(app.world())
            .any(|(p, a)| p.0 == CHAIR_AT && a.sprite == "metal-sheet")
    );
}
