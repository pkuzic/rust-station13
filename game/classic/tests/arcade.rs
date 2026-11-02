//! Arcade machines. Spec: docs/reference/arcade.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::arcade::{ANSWER_DS, Arcade, ArcadeOp, ArcadePanelOut, PRIZES, PendingArcadeOps};
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_world::{Appearance, GridPos, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'a': (tile: (id: "floor"), objects: ["arcade"]),
    },
    rows: [
        "WWWWWWWW",
        "W.a....W",
        "W......W",
        "WWWWWWWW",
    ],
)"#;

fn setup() -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(2, 1, 0));
    let mut q = app.world_mut().query_filtered::<Entity, With<Arcade>>();
    let machine = q.single(app.world()).unwrap();
    app.world_mut().run_schedule(FixedUpdate);
    (app, mob, machine)
}

fn ticks(app: &mut App, n: u32) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn open(app: &mut App, mob: Entity, machine: Entity) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(machine));
    ticks(app, 2);
}

#[test]
fn clicking_opens_the_window_and_walking_away_closes_it() {
    let (mut app, mob, machine) = setup();
    open(&mut app, mob, machine);
    let view = app
        .world()
        .get::<ArcadePanelOut>(mob)
        .unwrap()
        .0
        .clone()
        .expect("open");
    assert_eq!((view.player_hp, view.player_mp, view.enemy_hp), (30, 10, 45));
    assert_eq!(view.message, "arcade-new");
    assert!(!view.game_over);

    app.world_mut().get_mut::<GridPos>(mob).unwrap().0 = TilePos::new(6, 1, 0);
    ticks(&mut app, 2);
    assert!(app.world().get::<ArcadePanelOut>(mob).unwrap().0.is_none());
}

#[test]
fn a_move_is_answered_a_second_later() {
    let (mut app, mob, machine) = setup();
    open(&mut app, mob, machine);
    app.world_mut()
        .get_mut::<PendingArcadeOps>(mob)
        .unwrap()
        .0
        .push((machine, ArcadeOp::Attack));
    ticks(&mut app, 1);
    let game = app.world().get::<Arcade>(machine).unwrap();
    assert!(game.enemy_hp < 45);
    assert_eq!(game.message, "arcade-attack");
    assert_eq!(game.player_hp, 30);
    ticks(&mut app, (ANSWER_DS * 2.0) as u32 + 1);
    let game = app.world().get::<Arcade>(machine).unwrap();
    assert!(game.answer_at.is_none());
    assert!(game.player_hp < 30, "the villain hit back: {game:?}");
}

#[test]
fn someone_too_far_away_cant_play() {
    let (mut app, mob, machine) = setup();
    app.world_mut().get_mut::<GridPos>(mob).unwrap().0 = TilePos::new(6, 1, 0);
    app.world_mut()
        .get_mut::<PendingArcadeOps>(mob)
        .unwrap()
        .0
        .push((machine, ArcadeOp::Attack));
    ticks(&mut app, 1);
    assert_eq!(app.world().get::<Arcade>(machine).unwrap().enemy_hp, 45);
}

#[test]
fn winning_drops_a_toy_on_the_machines_tile() {
    let (mut app, mob, machine) = setup();
    app.world_mut().get_mut::<Arcade>(machine).unwrap().enemy_hp = 1;
    app.world_mut()
        .get_mut::<PendingArcadeOps>(mob)
        .unwrap()
        .0
        .push((machine, ArcadeOp::Attack));
    ticks(&mut app, (ANSWER_DS * 2.0) as u32 + 2);
    assert!(app.world().get::<Arcade>(machine).unwrap().game_over);
    assert_eq!(app.world().get::<Arcade>(machine).unwrap().message, "arcade-won");
    let at = app.world().get::<GridPos>(machine).unwrap().0;
    let mut q = app.world_mut().query::<(&Appearance, &GridPos)>();
    let toys = q
        .iter(app.world())
        .filter(|(a, p)| PRIZES.contains(&a.sprite.as_str()) && p.0 == at)
        .count();
    assert_eq!(toys, 1);
}
