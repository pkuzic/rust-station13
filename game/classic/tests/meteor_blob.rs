//! Meteors and the blob. Spec: docs/reference/meteor-blob.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::blob::{BlobCell, BlobState, attacked, blasted, damage, over, setup_round, spawn_blob};
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::meteor::{Meteor, spawn_meteor};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::round::{Phase, Round};
use rs13_world::{Appearance, GridPos, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        ' ': (tile: (id: "space")),
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
    },
    rows: [
        "            ",
        " WWWWWWWWWW ",
        " W........W ",
        " W........W ",
        " W........W ",
        " W........W ",
        " WWWWWWWWWW ",
        "            ",
    ],
)"#;

fn setup(mode: &'static str) -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().insert_resource(Round {
        phase: Phase::Playing { mode },
        forced_mode: None,
    });
    app
}

fn tick(app: &mut App, n: u32) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn cells(app: &mut App) -> usize {
    let world = app.world_mut();
    world.query::<&BlobCell>().iter(world).count()
}

fn meteors(app: &mut App) -> usize {
    let world = app.world_mut();
    world.query::<&Meteor>().iter(world).count()
}

#[test]
fn a_meteor_starts_in_space_and_is_gone_once_its_hit_something() {
    let mut app = setup("extended");
    let meteor = spawn_meteor(app.world_mut()).unwrap();
    let at = app.world().get::<GridPos>(meteor).unwrap().0;
    assert!(at.x == 0 || at.y == 0 || at.x == 11 || at.y == 7, "{at:?}");
    tick(&mut app, 300);
    assert!(app.world().get_entity(meteor).is_err());
}

#[test]
fn a_meteor_round_sends_waves_and_others_dont() {
    let mut app = setup("extended");
    tick(&mut app, 60);
    assert_eq!(meteors(&mut app), 0);
    let mut app = setup("meteor");
    let mut most = 0;
    for _ in 0..4000 {
        tick(&mut app, 1);
        most = most.max(meteors(&mut app));
    }
    assert!(most > 0, "some meteors came");
}

#[test]
fn a_meteor_breaks_the_wall_it_hits() {
    let mut app = setup("extended");
    let solid = |app: &mut App| {
        let world = app.world_mut();
        let mut maps = world.query::<&rs13_world::MapLayer>();
        let map = maps.iter(world).next().unwrap();
        (0..12)
            .flat_map(|x| (0..8).map(move |y| TilePos::new(x, y, 0)))
            .filter(|at| map.0.tile(*at).is_some_and(|t| t.solid))
            .count()
    };
    let before = solid(&mut app);
    for _ in 0..30 {
        spawn_meteor(app.world_mut());
        tick(&mut app, 100);
    }
    assert!(solid(&mut app) < before);
}

#[test]
fn the_blob_grows_while_it_has_health() {
    let mut app = setup("blob");
    assert!(setup_round(app.world_mut()));
    assert_eq!(cells(&mut app), 1);
    tick(&mut app, 600);
    assert!(cells(&mut app) > 1, "{}", cells(&mut app));
}

#[test]
fn hurting_a_cell_changes_its_look_and_ends_it() {
    let mut app = setup("blob");
    let cell = spawn_blob(app.world_mut(), TilePos::new(3, 3, 0), 30.0);
    assert_eq!(app.world().get::<Appearance>(cell).unwrap().state, "a");
    damage(app.world_mut(), cell, 15.0);
    assert_eq!(app.world().get::<Appearance>(cell).unwrap().state, "b");
    damage(app.world_mut(), cell, 10.0);
    assert_eq!(app.world().get::<Appearance>(cell).unwrap().state, "c");
    damage(app.world_mut(), cell, 5.0);
    assert!(app.world().get_entity(cell).is_err());
}

#[test]
fn a_devastating_blast_destroys_a_cell_and_lesser_ones_hurt_it() {
    let mut app = setup("blob");
    let cell = spawn_blob(app.world_mut(), TilePos::new(3, 3, 0), 30.0);
    blasted(app.world_mut(), cell, 3);
    let left = app.world().get::<BlobCell>(cell).unwrap().health;
    assert!((5.0..=15.0).contains(&left), "{left}");
    blasted(app.world_mut(), cell, 1);
    assert!(app.world().get_entity(cell).is_err());
}

#[test]
fn a_weapon_does_a_quarter_of_its_force_and_a_lit_welder_fifteen() {
    let mut app = setup("blob");
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(3, 3, 0));
    let cell = spawn_blob(app.world_mut(), TilePos::new(4, 3, 0), 30.0);
    let hold = |app: &mut App, id: &str| {
        let item = spawn_object(app.world_mut(), &ObjectSpec::new(id), TilePos::new(3, 3, 0)).unwrap();
        app.world_mut()
            .entity_mut(item)
            .remove::<GridPos>()
            .insert(HeldBy(mob));
        let mut hands = app.world_mut().get_mut::<Hands>(mob).unwrap();
        let hand = hands.active;
        *hands.slot_mut(hand) = Some(item);
        item
    };
    hold(&mut app, "toolbox-mechanical");
    attacked(app.world_mut(), mob, cell);
    assert_eq!(app.world().get::<BlobCell>(cell).unwrap().health, 27.5);
    let welder = hold(&mut app, "welder");
    app.world_mut()
        .get_mut::<rs13_classic::welding::Welder>(welder)
        .unwrap()
        .lit = true;
    attacked(app.world_mut(), mob, cell);
    assert_eq!(app.world().get::<BlobCell>(cell).unwrap().health, 12.5);
}

#[test]
fn the_staff_win_with_no_cells_left() {
    let mut app = setup("blob");
    assert!(setup_round(app.world_mut()));
    let world = app.world_mut();
    let all: Vec<Entity> = world
        .query_filtered::<Entity, With<BlobCell>>()
        .iter(world)
        .collect();
    for cell in all {
        world.entity_mut(cell).despawn();
    }
    tick(&mut app, 2);
    assert!(over(app.world()));
    assert_eq!(app.world().resource::<BlobState>().staff_won, Some(true));
}
