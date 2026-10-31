//! Pulling. Spec: docs/reference/pulling.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::closets::Closet;
use rs13_classic::doors::Airlock;
use rs13_classic::hands::Item;
use rs13_classic::movement::MoveIntent;
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_classic::pulling::Pulling;
use rs13_world::{Dir, GridPos, TileMap, TilePos};

// Rows run top down: the crate is at (2, 2), the wrench at (1, 1), the airlock at (5, 2).
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'c': (tile: (id: "floor"), objects: ["crate"]),
        'w': (tile: (id: "floor"), objects: ["wrench"]),
        'A': (tile: (id: "floor"), objects: ["airlock"]),
    },
    rows: [
        "WWWWWWWWWW",
        "W.c..A...W",
        "Ww.......W",
        "WWWWWWWWWW",
    ],
)"#;

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().run_schedule(FixedUpdate);
    app
}

fn ticks(app: &mut App, n: u64) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn find<T: Component>(app: &mut App) -> Entity {
    let mut q = app.world_mut().query_filtered::<Entity, With<T>>();
    q.iter(app.world()).next().unwrap()
}

fn act(app: &mut App, mob: Entity, action: Action) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
    ticks(app, 1);
}

fn at(app: &App, e: Entity) -> TilePos {
    app.world().get::<GridPos>(e).unwrap().0
}

/// Hold `dir` until the mob has taken `steps` steps, then stop.
fn walk(app: &mut App, mob: Entity, dir: Dir, steps: i32) {
    let goal = {
        let mut p = at(app, mob);
        for _ in 0..steps {
            p = p.step(dir);
        }
        p
    };
    app.world_mut().get_mut::<MoveIntent>(mob).unwrap().dir = Some(dir);
    for _ in 0..200 {
        if at(app, mob) == goal {
            break;
        }
        ticks(app, 1);
    }
    app.world_mut().get_mut::<MoveIntent>(mob).unwrap().dir = None;
    assert_eq!(at(app, mob), goal, "walked {steps} {dir:?}");
}

#[test]
fn a_pulled_crate_follows_into_the_tile_you_left() {
    let mut app = setup();
    let crate_ = find::<Closet>(&mut app);
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(3, 2, 0));
    ticks(&mut app, 1);
    act(&mut app, mob, Action::Pull(crate_));
    assert_eq!(app.world().get::<Pulling>(mob), Some(&Pulling(crate_)));

    // It was beside us; one step away leaves it two tiles behind, so it follows into the
    // tile we left.
    walk(&mut app, mob, Dir::East, 1);
    assert_eq!(at(&app, crate_), TilePos::new(3, 2, 0));
    walk(&mut app, mob, Dir::South, 1);
    // We are now diagonal to it: it follows into the tile we left.
    assert_eq!(at(&app, mob), TilePos::new(4, 1, 0));
    assert_eq!(at(&app, crate_), TilePos::new(4, 2, 0));
}

#[test]
fn only_what_is_near_and_not_anchored_can_be_pulled() {
    let mut app = setup();
    let crate_ = find::<Closet>(&mut app);
    let airlock = find::<Airlock>(&mut app);
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(4, 2, 0));
    ticks(&mut app, 1);
    act(&mut app, mob, Action::Pull(crate_));
    assert!(app.world().get::<Pulling>(mob).is_none(), "two tiles away");
    act(&mut app, mob, Action::Pull(airlock));
    assert!(
        app.world().get::<Pulling>(mob).is_none(),
        "an airlock is anchored"
    );
    act(&mut app, mob, Action::Pull(mob));
    assert!(app.world().get::<Pulling>(mob).is_none(), "not yourself");
}

#[test]
fn letting_go_picking_up_or_falling_behind_ends_the_pull() {
    let mut app = setup();
    let wrench = find::<Item>(&mut app);
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(2, 1, 0));
    ticks(&mut app, 1);

    act(&mut app, mob, Action::Pull(wrench));
    assert!(app.world().get::<Pulling>(mob).is_some());
    act(&mut app, mob, Action::StopPulling);
    assert!(
        app.world().get::<Pulling>(mob).is_none(),
        "the pull button lets go"
    );

    act(&mut app, mob, Action::Pull(wrench));
    act(&mut app, mob, Action::UseOn(wrench));
    ticks(&mut app, 1);
    assert!(
        app.world().get::<Pulling>(mob).is_none(),
        "picked up, it isn't pulled"
    );
}

#[test]
fn a_pulled_person_is_dragged_and_can_walk_away() {
    let mut app = setup();
    let puller = spawn_player_mob(app.world_mut(), TilePos::new(6, 2, 0));
    let pulled = spawn_player_mob(app.world_mut(), TilePos::new(6, 1, 0));
    ticks(&mut app, 1);
    act(&mut app, puller, Action::Pull(pulled));
    walk(&mut app, puller, Dir::East, 2);
    assert_eq!(
        at(&app, pulled),
        TilePos::new(7, 2, 0),
        "dragged into the tile behind"
    );

    // They walk off; the puller's next step lets go.
    walk(&mut app, pulled, Dir::West, 2);
    walk(&mut app, puller, Dir::South, 1);
    assert!(app.world().get::<Pulling>(puller).is_none());
    assert_eq!(at(&app, pulled), TilePos::new(5, 2, 0));
}
