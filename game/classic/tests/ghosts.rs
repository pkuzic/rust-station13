//! Dead players walking on as a see-through copy of their body. Spec: docs/reference/ghosts.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::equipment::Equipment;
use rs13_classic::ghosts::{Departures, LeftBody};
use rs13_classic::health::{Body, Vitals};
use rs13_classic::morgue::{Morgue, finish_cremation};
use rs13_classic::movement::{Mob, MoveIntent};
use rs13_classic::objects::{spawn_player_mob, spawn_station};
use rs13_classic::speech::PendingSpeech;
use rs13_world::{Appearance, Dir, DisplayName, Facing, Ghost, GridPos, MobStat, TileMap, TilePos};

/// Two decks with a ladder between them; a wall splits the lower one.
const LOWER: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'L': (tile: (id: "floor"), objects: [(id: "ladder", vars: {"istop": "0"})]),
        'c': (tile: (id: "floor"), objects: [(id: "crematorium", vars: {"id": "crema"})]),
    },
    rows: [
        "WWWWWWW",
        "W.WL.cW",
        "W.W...W",
        "WWWWWWW",
    ],
)"#;
const UPPER: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
    },
    rows: [
        "WWWWWWW",
        "W.....W",
        "W.....W",
        "WWWWWWW",
    ],
)"#;

const STANDING: TilePos = TilePos::new(3, 1, 0);

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let decks = vec![
        TileMap::from_ron(LOWER, 0).unwrap(),
        TileMap::from_ron(UPPER, 1).unwrap(),
    ];
    let problems = spawn_station(app.world_mut(), decks);
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), STANDING);
    ticks(&mut app, 1);
    (app, mob)
}

fn ticks(app: &mut App, n: u32) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn kill(app: &mut App, mob: Entity) -> Entity {
    let mut body = app.world_mut().get_mut::<Body>(mob).unwrap();
    body.oxy = 500.0;
    ticks(app, 2);
    app.world().get::<LeftBody>(mob).expect("they left the body").0
}

fn messages(app: &App, who: Entity) -> Vec<String> {
    let outbox = app.world().get::<Outbox>(who).unwrap();
    outbox.0.iter().map(|m| m.id.clone()).collect()
}

fn at(app: &App, who: Entity) -> TilePos {
    app.world().get::<GridPos>(who).unwrap().0
}

#[test]
fn someone_who_dies_walks_on_looking_as_their_body_did() {
    let (mut app, mob) = setup();
    app.world_mut().get_mut::<Mob>(mob).unwrap().facing = Some(Dir::East);
    app.world_mut().get_mut::<DisplayName>(mob).unwrap().0 = "Ann Hart".into();
    let worn = app.world().get::<Equipment>(mob).unwrap().0.len();
    assert!(worn > 0, "the starting outfit");
    assert!(
        app.world().get::<LeftBody>(mob).is_none(),
        "alive: still in the body"
    );

    let ghost = kill(&mut app, mob);

    let world = app.world();
    assert_eq!(world.get::<Vitals>(mob).unwrap().stat, MobStat::Dead);
    assert_eq!(
        world.get::<Appearance>(mob).unwrap().state,
        "dead",
        "the body lies there"
    );
    assert_eq!(
        *world.get::<Appearance>(ghost).unwrap(),
        Appearance::new("human", "")
    );
    assert_eq!(world.get::<GridPos>(ghost).unwrap().0, STANDING);
    assert_eq!(world.get::<Facing>(ghost).unwrap().0, Facing::of(Dir::East).0);
    assert_eq!(world.get::<DisplayName>(ghost).unwrap().0, "Ann Hart");
    assert_eq!(world.get::<Ghost>(ghost).unwrap().worn.len(), worn);
    assert_eq!(world.resource::<Departures>().0, vec![(mob, ghost)]);
    assert!(messages(&app, ghost).contains(&"ghost-left-body".to_owned()));

    // Only once.
    ticks(&mut app, 5);
    let mut ghosts = app.world_mut().query_filtered::<Entity, With<Ghost>>();
    assert_eq!(ghosts.iter(app.world()).count(), 1);
}

#[test]
fn a_ghost_keeps_what_was_worn_when_the_body_is_stripped() {
    let (mut app, mob) = setup();
    let ghost = kill(&mut app, mob);
    let before = app.world().get::<Ghost>(ghost).unwrap().clone();
    let items: Vec<Entity> = app
        .world()
        .get::<Equipment>(mob)
        .unwrap()
        .0
        .values()
        .copied()
        .collect();
    for item in items {
        app.world_mut().entity_mut(item).despawn();
    }
    app.world_mut().get_mut::<Equipment>(mob).unwrap().0.clear();
    ticks(&mut app, 2);
    assert_eq!(*app.world().get::<Ghost>(ghost).unwrap(), before);
}

#[test]
fn a_ghost_goes_through_walls_but_not_off_the_deck() {
    let (mut app, mob) = setup();
    let ghost = kill(&mut app, mob);
    app.world_mut().get_mut::<MoveIntent>(ghost).unwrap().dir = Some(Dir::West);
    ticks(&mut app, 80);
    assert_eq!(
        at(&app, ghost),
        TilePos::new(0, 1, 0),
        "through the wall, to the edge"
    );
    assert_eq!(
        app.world().get::<Facing>(ghost).unwrap().0,
        Facing::of(Dir::West).0
    );
    assert_eq!(at(&app, mob), STANDING, "the body stays");
}

#[test]
fn clicking_a_ladder_takes_a_ghost_to_the_other_deck_from_anywhere() {
    let (mut app, mob) = setup();
    let ghost = kill(&mut app, mob);
    app.world_mut().get_mut::<GridPos>(ghost).unwrap().0 = TilePos::new(1, 1, 0);
    let mut q = app.world_mut().query::<(Entity, &Appearance)>();
    let ladder = q.iter(app.world()).find(|(_, a)| a.sprite == "ladder").unwrap().0;
    app.world_mut()
        .get_mut::<PendingActions>(ghost)
        .unwrap()
        .0
        .push(Action::UseOn(ladder));
    ticks(&mut app, 2);
    assert_eq!(at(&app, ghost).z, 1);
    assert_eq!(at(&app, ghost).x, 1, "the same spot, not the ladder's");
}

#[test]
fn a_ghost_only_looks_at_what_it_clicks() {
    let (mut app, mob) = setup();
    let ghost = kill(&mut app, mob);
    let mut q = app.world_mut().query::<(Entity, &Morgue)>();
    let drawer = q.single(app.world()).unwrap().0;
    app.world_mut()
        .get_mut::<PendingActions>(ghost)
        .unwrap()
        .0
        .push(Action::UseOn(drawer));
    ticks(&mut app, 2);
    assert!(
        app.world().get::<Morgue>(drawer).unwrap().tray.is_none(),
        "it stays shut"
    );
    assert!(messages(&app, ghost).contains(&"examine".to_owned()));
}

#[test]
fn the_dead_hear_the_living_but_not_the_other_way_round() {
    let (mut app, mob) = setup();
    let ghost = kill(&mut app, mob);
    let other_body = spawn_player_mob(app.world_mut(), TilePos::new(4, 1, 0));
    let other_ghost = kill(&mut app, other_body);
    let living = spawn_player_mob(app.world_mut(), TilePos::new(4, 2, 0));
    ticks(&mut app, 1);

    app.world_mut()
        .get_mut::<PendingSpeech>(ghost)
        .unwrap()
        .0
        .push("Boo!".into());
    ticks(&mut app, 2);
    assert!(
        messages(&app, other_ghost).contains(&"ghost-say".to_owned()),
        "ghost to ghost"
    );
    assert!(
        messages(&app, ghost).contains(&"ghost-say".to_owned()),
        "to themselves"
    );
    assert!(
        !messages(&app, living).contains(&"ghost-say".to_owned()),
        "the living don't"
    );

    app.world_mut()
        .get_mut::<PendingSpeech>(living)
        .unwrap()
        .0
        .push("Hello".into());
    ticks(&mut app, 2);
    assert!(
        messages(&app, ghost).contains(&"say".to_owned()),
        "ghosts listen in"
    );
}

#[test]
fn someone_burnt_alive_leaves_as_a_ghost_too() {
    let (mut app, mob) = setup();
    let mut q = app.world_mut().query::<(Entity, &Morgue)>();
    let drawer = q.single(app.world()).unwrap().0;
    app.world_mut()
        .get_mut::<Morgue>(drawer)
        .unwrap()
        .contents
        .push(mob);
    finish_cremation(app.world_mut(), drawer);
    assert!(app.world().get_entity(mob).is_err(), "no body left");
    let departures = &app.world().resource::<Departures>().0;
    assert_eq!(departures.len(), 1);
    assert_eq!(departures[0].0, mob);
    assert!(app.world().get::<Ghost>(departures[0].1).is_some());
}

#[test]
fn an_observer_is_a_ghost_with_no_body_behind_it() {
    let (mut app, _) = setup();
    let at = TilePos::new(2, 2, 0);
    let watcher = rs13_classic::ghosts::spawn_observer(app.world_mut(), at, Some("Watcher".to_owned()));
    ticks(&mut app, 2);
    assert!(app.world().get::<Ghost>(watcher).is_some());
    assert_eq!(app.world().get::<DisplayName>(watcher).unwrap().0, "Watcher");
    assert_eq!(app.world().get::<GridPos>(watcher).unwrap().0, at);
    assert!(app.world().get::<Vitals>(watcher).is_none());
    assert_eq!(app.world().resource::<Departures>().0.len(), 0);
    let unnamed = rs13_classic::ghosts::spawn_observer(app.world_mut(), at, None);
    assert!(!app.world().get::<DisplayName>(unnamed).unwrap().0.is_empty());
}
