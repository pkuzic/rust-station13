//! Crematoriums and their igniters. Spec: docs/reference/morgue.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::closets::InCloset;
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::health::Body;
use rs13_classic::morgue::{Crematorium, Morgue};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::pulling::Pulling;
use rs13_world::{Appearance, GridPos, ObjectSpec, TileMap, TilePos};

/// A crematorium (1, 2) with room for its tray to the south, its igniter, and a card that lights it.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'c': (tile: (id: "floor"), objects: [(id: "crematorium", vars: {"id": "crema"})]),
        's': (tile: (id: "floor"), objects: [(id: "crematorium-igniter", vars: {"id": "crema"})]),
        'i': (tile: (id: "floor"), objects: [(id: "id-card", access: ["crematorium"])]),
    },
    rows: [
        "WWWWWW",
        "Wc.s.W",
        "W...iW",
        "WWWWWW",
    ],
)"#;

const TRAY: TilePos = TilePos::new(1, 1, 0);
const STANDING: TilePos = TilePos::new(2, 1, 0);

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), STANDING);
    app.world_mut().run_schedule(FixedUpdate);
    (app, mob)
}

fn ticks(app: &mut App, n: u32) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn find(app: &mut App, sprite: &str) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &Appearance)>();
    q.iter(app.world()).find(|(_, a)| a.sprite == sprite).unwrap().0
}

fn hold(app: &mut App, mob: Entity, item: Entity) {
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    app.world_mut().get_mut::<Hands>(mob).unwrap().right = Some(item);
}

/// Click and return the ids of every message the mob got.
fn click(app: &mut App, mob: Entity, target: Entity) -> Vec<String> {
    let before = app.world().get::<Outbox>(mob).unwrap().0.len();
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    ticks(app, 2);
    app.world().get::<Outbox>(mob).unwrap().0[before..]
        .iter()
        .map(|m| m.id.clone())
        .collect()
}

fn look(app: &App, e: Entity) -> String {
    app.world().get::<Appearance>(e).unwrap().state.clone()
}

#[test]
fn the_igniter_needs_its_access_and_crackles_when_empty() {
    let (mut app, mob) = setup();
    let crematorium = find(&mut app, "crematorium");
    let igniter = find(&mut app, "crematorium-igniter");
    assert_eq!(click(&mut app, mob, igniter), ["crematorium-denied"]);

    let card = find(&mut app, "id-card");
    hold(&mut app, mob, card);
    let said = click(&mut app, mob, igniter);
    assert!(said.contains(&"crematorium-crackle".to_owned()), "{said:?}");
    assert!(
        app.world()
            .get::<Crematorium>(crematorium)
            .unwrap()
            .lit_at
            .is_none()
    );
}

#[test]
fn whoever_is_inside_burns_to_an_urn_and_loose_things_stay() {
    let (mut app, mob) = setup();
    let crematorium = find(&mut app, "crematorium");
    let igniter = find(&mut app, "crematorium-igniter");
    let card = find(&mut app, "id-card");

    // The tray comes out south; a body is pulled onto it, holding a crowbar, and a
    // wrench lies beside them.
    click(&mut app, mob, crematorium);
    let tray = app
        .world()
        .get::<Morgue>(crematorium)
        .unwrap()
        .tray
        .expect("tray out");
    assert_eq!(app.world().get::<GridPos>(tray).unwrap().0, TRAY);
    assert_eq!(look(&app, crematorium), "crema0");
    assert_eq!(look(&app, tray), "cremat");
    let body = spawn_player_mob(app.world_mut(), TilePos::new(3, 1, 0));
    let crowbar = spawn_object(app.world_mut(), &ObjectSpec::new("crowbar"), TRAY).unwrap();
    hold(&mut app, body, crowbar);
    let wrench = spawn_object(app.world_mut(), &ObjectSpec::new("wrench"), TRAY).unwrap();
    app.world_mut().entity_mut(mob).insert(Pulling(body));
    click(&mut app, mob, tray);
    assert_eq!(app.world().get::<GridPos>(body).unwrap().0, TRAY);
    click(&mut app, mob, tray);
    assert_eq!(look(&app, crematorium), "crema2");

    hold(&mut app, mob, card);
    let said = click(&mut app, mob, igniter);
    assert!(said.contains(&"crematorium-roar".to_owned()), "{said:?}");
    app.world_mut().get_mut::<Hands>(mob).unwrap().right = None;
    assert_eq!(click(&mut app, mob, crematorium), ["crematorium-locked"]);
    assert_eq!(app.world().get::<Morgue>(crematorium).unwrap().tray, None);

    // Burning: after a few seconds they are badly hurt.
    ticks(&mut app, 20 * 5);
    let health = app.world().get::<Body>(body).unwrap().health();
    assert!(health <= -40.0, "{health}");

    ticks(&mut app, 20 * 6);
    assert!(app.world().get_entity(body).is_err(), "the body is gone");
    let crowbar_kept = app.world().get::<InCloset>(crowbar).is_some();
    assert!(crowbar_kept || app.world().get_entity(crowbar).is_err());
    let urn = find(&mut app, "urn");
    assert_eq!(app.world().get::<InCloset>(urn).unwrap().0, crematorium);
    assert_eq!(app.world().get::<InCloset>(wrench).unwrap().0, crematorium);
    assert!(
        app.world()
            .get::<Crematorium>(crematorium)
            .unwrap()
            .lit_at
            .is_none()
    );

    // It opens again, and the urn and wrench come out on the tray.
    click(&mut app, mob, crematorium);
    assert_eq!(app.world().get::<GridPos>(urn).unwrap().0, TRAY);
    assert_eq!(app.world().get::<GridPos>(wrench).unwrap().0, TRAY);
}
