//! Disposals: put something in a bin, it pumps up, flushes, and the pipes carry it to an
//! outlet that throws it out. Spec: docs/reference/disposals.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::disposals::{DisposalBin, InDisposal, Pump};
use rs13_classic::objects::{spawn_player_mob, spawn_station};
use rs13_world::{Appearance, GridPos, TileMap, TilePos};

/// A bin at (1, 1) on its trunk, pipes east to an outlet on its trunk at (5, 1) facing
/// east, open floor beyond it; a wrench where the player stands, beside the bin. `gap`
/// leaves the pipe at (3, 1) out.
fn line(gap: bool) -> String {
    let middle = if gap { "W" } else { "p" };
    format!(
        r#"(
            legend: {{
                'W': (tile: (id: "wall", solid: true)),
                '.': (tile: (id: "floor")),
                'w': (tile: (id: "floor"), objects: ["wrench"]),
                'b': (tile: (id: "floor"), objects: ["disposal-bin", (id: "disposal-pipe", dir: East, state: Some("pipe-t"))]),
                'p': (tile: (id: "floor"), objects: [(id: "disposal-pipe", dir: East, state: Some("pipe-s"))]),
                'g': (tile: (id: "floor")),
                'o': (tile: (id: "floor"), objects: [(id: "disposal-outlet", dir: East), (id: "disposal-pipe", dir: West, state: Some("pipe-t"))]),
            }},
            rows: [
                "WWWWWWWWWWW",
                "Wbp{}po....W",
                "Ww........W",
                "WWWWWWWWWWW",
            ],
        )"#,
        if gap { "g" } else { middle }
    )
}

const BIN: TilePos = TilePos::new(1, 2, 0);
const PLAYER: TilePos = TilePos::new(1, 1, 0);

fn station(decks: &[String]) -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let maps = decks
        .iter()
        .enumerate()
        .map(|(z, t)| TileMap::from_ron(t, z as i32).unwrap())
        .collect();
    let problems = spawn_station(app.world_mut(), maps);
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().run_schedule(FixedUpdate);
    app
}

fn seconds(app: &mut App, s: u64) {
    for _ in 0..s * 20 {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn one<C: Component>(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<C>>()
        .single(app.world())
        .unwrap()
}

fn find(app: &mut App, sprite: &str) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &Appearance)>();
    q.iter(app.world()).find(|(_, a)| a.sprite == sprite).unwrap().0
}

fn act(app: &mut App, mob: Entity, action: Action) -> Vec<String> {
    let before = app.world().get::<Outbox>(mob).unwrap().0.len();
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
    app.world_mut().run_schedule(FixedUpdate);
    app.world().get::<Outbox>(mob).unwrap().0[before..]
        .iter()
        .map(|m| m.id.clone())
        .collect()
}

/// The player puts the wrench in the bin; where it is once it has had time to arrive.
fn flush_wrench(app: &mut App) -> Option<TilePos> {
    let mob = spawn_player_mob(app.world_mut(), PLAYER);
    let wrench = find(app, "wrench");
    act(app, mob, Action::UseOn(wrench));
    let bin = one::<DisposalBin>(app);
    assert_eq!(act(app, mob, Action::UseOn(bin)), ["disposal-placed-self"]);
    assert!(app.world().get::<InDisposal>(wrench).is_some(), "into the bin");
    assert!(app.world().get::<GridPos>(wrench).is_none(), "off the map");
    // Pumping up from empty takes about a minute.
    seconds(app, 80);
    app.world().get::<GridPos>(wrench).map(|p| p.0)
}

#[test]
fn a_bin_pumps_up_flushes_and_the_outlet_throws_it_out() {
    let mut app = station(&[line(false)]);
    let bin = one::<DisposalBin>(&mut app);
    assert_eq!(app.world().get::<GridPos>(bin).unwrap().0, BIN);
    let landed = flush_wrench(&mut app);
    assert_eq!(landed, Some(TilePos::new(8, 2, 0)), "three tiles past the outlet");
    let bin = app.world().get::<DisposalBin>(bin).unwrap();
    assert!(bin.contents.is_empty() && !bin.handle);
    assert_ne!(bin.pump, Pump::Off);
}

#[test]
fn a_gap_in_the_pipes_spills_what_was_flushed() {
    let mut app = station(&[line(true)]);
    assert_eq!(
        flush_wrench(&mut app),
        Some(TilePos::new(3, 2, 0)),
        "out at the gap"
    );
}

/// Lower deck: the bin, and a pipe up at (2, 2). Upper deck: the pipe down above it, pipes
/// east and an outlet facing east.
const LOWER: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'w': (tile: (id: "floor"), objects: ["wrench"]),
        'b': (tile: (id: "floor"), objects: ["disposal-bin", (id: "disposal-pipe", dir: East, state: Some("pipe-t"))]),
        'u': (tile: (id: "floor"), objects: [(id: "disposal-pipe", dir: West, state: Some("pipe-u"))]),
    },
    rows: [
        "WWWWWWWWWWW",
        "Wbu.......W",
        "Ww........W",
        "WWWWWWWWWWW",
    ],
)"#;

const UPPER: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'd': (tile: (id: "floor"), objects: [(id: "disposal-pipe", dir: East, state: Some("pipe-d"))]),
        'p': (tile: (id: "floor"), objects: [(id: "disposal-pipe", dir: East, state: Some("pipe-s"))]),
        'o': (tile: (id: "floor"), objects: [(id: "disposal-outlet", dir: East), (id: "disposal-pipe", dir: West, state: Some("pipe-t"))]),
    },
    rows: [
        "WWWWWWWWWWW",
        "W.dpo.....W",
        "W.........W",
        "WWWWWWWWWWW",
    ],
)"#;

#[test]
fn the_pipes_carry_things_up_to_the_next_deck() {
    let mut app = station(&[LOWER.to_owned(), UPPER.to_owned()]);
    assert_eq!(
        flush_wrench(&mut app),
        Some(TilePos::new(7, 2, 1)),
        "out of the upper outlet"
    );
}
