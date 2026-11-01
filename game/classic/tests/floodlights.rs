//! Emergency floodlights. Spec: docs/reference/floodlights.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::cells::PowerCell;
use rs13_classic::floodlights::{FLOODLIGHT_USE, Floodlight};
use rs13_classic::machines::MACHINE_TICKS;
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_world::{Appearance, LightSource, TileMap, TilePos};

/// A floodlight beside the player, who stands on a screwdriver and a crowbar.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        't': (tile: (id: "floor"), objects: ["screwdriver", "crowbar"]),
        'f': (tile: (id: "floor"), objects: ["floodlight"]),
    },
    rows: [
        "WWWW",
        "WtfW",
        "WWWW",
    ],
)"#;

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    spawn_player_mob(app.world_mut(), TilePos::new(1, 1, 0));
    ticks(&mut app, 2);
    app
}

fn ticks(app: &mut App, n: u64) {
    for _ in 0..n {
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

/// The player does `action`; what they are told.
fn act(app: &mut App, action: Action) -> Vec<String> {
    let mob = one::<PendingActions>(app);
    let before = app.world().get::<Outbox>(mob).unwrap().0.len();
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
    ticks(app, 1);
    app.world().get::<Outbox>(mob).unwrap().0[before..]
        .iter()
        .map(|m| m.id.clone())
        .collect()
}

fn look(app: &mut App) -> String {
    let e = one::<Floodlight>(app);
    app.world().get::<Appearance>(e).unwrap().state.clone()
}

fn lit(app: &mut App) -> bool {
    let e = one::<Floodlight>(app);
    app.world().get::<LightSource>(e).is_some()
}

#[test]
fn a_hand_switches_it_and_it_lights_until_its_cell_runs_flat() {
    let mut app = setup();
    let flood = one::<Floodlight>(&mut app);
    assert!(!lit(&mut app));
    assert_eq!(act(&mut app, Action::UseOn(flood)), ["floodlight-on"]);
    ticks(&mut app, 1);
    assert_eq!(look(&mut app), "flood01");
    assert!(lit(&mut app), "it gives light");
    assert_eq!(act(&mut app, Action::UseOn(flood)), ["floodlight-off"]);
    ticks(&mut app, 1);
    assert!(!lit(&mut app));

    // Two cycles' worth left in the cell.
    let cell = app.world().get::<Floodlight>(flood).unwrap().cell.unwrap();
    app.world_mut().get_mut::<PowerCell>(cell).unwrap().charge = 2.0 * FLOODLIGHT_USE;
    act(&mut app, Action::UseOn(flood));
    ticks(&mut app, 3 * MACHINE_TICKS);
    assert!(
        !app.world().get::<Floodlight>(flood).unwrap().on,
        "flat, it went out"
    );
    assert!(!lit(&mut app));
    assert_eq!(
        act(&mut app, Action::UseOn(flood)),
        Vec::<String>::new(),
        "nor turns on"
    );
}

#[test]
fn its_cell_comes_out_through_the_panel_and_goes_back_in() {
    let mut app = setup();
    let flood = one::<Floodlight>(&mut app);
    let screwdriver = find(&mut app, "screwdriver");
    let crowbar = find(&mut app, "crowbar");
    act(&mut app, Action::UseOn(screwdriver));
    assert_eq!(act(&mut app, Action::UseOn(flood)), ["floodlight-unscrewed"]);
    act(&mut app, Action::SwapHands);
    act(&mut app, Action::UseOn(crowbar));
    assert_eq!(act(&mut app, Action::UseOn(flood)), ["floodlight-panel-removed"]);
    ticks(&mut app, 1);
    assert_eq!(look(&mut app), "floodob00", "the cell shows");

    act(&mut app, Action::Drop);
    assert_eq!(act(&mut app, Action::UseOn(flood)), ["floodlight-cell-removed"]);
    ticks(&mut app, 1);
    assert_eq!(look(&mut app), "floodo00");
    assert!(app.world().get::<Floodlight>(flood).unwrap().cell.is_none());

    assert_eq!(act(&mut app, Action::UseOn(flood)), ["floodlight-cell-inserted"]);
    ticks(&mut app, 1);
    assert_eq!(look(&mut app), "floodob00");
}
