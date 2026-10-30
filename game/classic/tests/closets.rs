//! Closets, secure lockers and crates. Spec: docs/reference/closets.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::closets::{Closet, ClosetLock, InCloset};
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::movement::MoveIntent;
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_classic::welding::Welder;
use rs13_world::{Appearance, Dense, Dir, GridPos, Hidden, TileMap, TilePos};

/// A room with a plain closet with a wrench on its tile, an engineer's locker, a crate,
/// a tool closet, an engineering ID card and a welding tool.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'c': (tile: (id: "floor"), objects: ["closet", "wrench"]),
        'l': (tile: (id: "floor"), objects: ["locker-engineer"]),
        'k': (tile: (id: "floor"), objects: ["crate"]),
        't': (tile: (id: "floor"), objects: ["closet-tools"]),
        'i': (tile: (id: "floor"), objects: [(id: "id-card", access: ["engine_equip"])]),
        'w': (tile: (id: "floor"), objects: ["welder"]),
    },
    rows: [
        "WWWWWWWWW",
        "Wc.l.k.tW",
        "W.i.w...W",
        "W.......W",
        "WWWWWWWWW",
    ],
)"#;

const CLOSET: TilePos = TilePos::new(1, 3, 0);
const LOCKER: TilePos = TilePos::new(3, 3, 0);
const CRATE: TilePos = TilePos::new(5, 3, 0);
const TOOLS: TilePos = TilePos::new(7, 3, 0);

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    ticks(&mut app, 2);
    app
}

fn ticks(app: &mut App, n: u32) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn closet_at(app: &mut App, pos: TilePos) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &GridPos, &Closet)>();
    q.iter(app.world()).find(|(_, p, _)| p.0 == pos).unwrap().0
}

fn items_at(app: &mut App, pos: TilePos) -> Vec<String> {
    let mut q = app
        .world_mut()
        .query_filtered::<(&Appearance, &GridPos), With<rs13_classic::hands::Item>>();
    let mut names: Vec<String> = q
        .iter(app.world())
        .filter(|(_, p)| p.0 == pos)
        .map(|(a, _)| a.sprite.clone())
        .collect();
    names.sort();
    names
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
    ticks(app, 1);
    app.world().get::<Outbox>(mob).unwrap().0[before..]
        .iter()
        .map(|m| m.id.clone())
        .collect()
}

fn closet(app: &App, entity: Entity) -> Closet {
    app.world().get::<Closet>(entity).unwrap().clone()
}

#[test]
fn a_closet_keeps_what_was_on_its_tile_and_lets_it_out() {
    let mut app = setup();
    let c = closet_at(&mut app, CLOSET);
    assert_eq!(closet(&app, c).contents.len(), 1, "the wrench went in");
    assert!(items_at(&mut app, CLOSET).is_empty());
    assert!(app.world().get::<Dense>(c).is_some());

    let mob = spawn_player_mob(app.world_mut(), TilePos::new(2, 3, 0));
    click(&mut app, mob, c);
    assert!(closet(&app, c).opened);
    assert_eq!(items_at(&mut app, CLOSET), ["wrench"]);
    assert!(
        app.world().get::<Dense>(c).is_none(),
        "open, it is out of the way"
    );
    assert_eq!(app.world().get::<Appearance>(c).unwrap().state, "open");

    // Closing takes the wrench back in.
    click(&mut app, mob, c);
    assert!(!closet(&app, c).opened);
    assert!(items_at(&mut app, CLOSET).is_empty());
}

#[test]
fn someone_shut_in_a_closet_is_hidden_and_walks_out() {
    let mut app = setup();
    let c = closet_at(&mut app, CLOSET);
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(2, 3, 0));
    click(&mut app, mob, c);
    let victim = spawn_player_mob(app.world_mut(), CLOSET);
    click(&mut app, mob, c);
    assert_eq!(app.world().get::<InCloset>(victim).map(|i| i.0), Some(c));
    assert!(app.world().get::<Hidden>(victim).is_some());

    // Inside, nothing outside can be reached.
    let before = app.world().get::<Hands>(victim).unwrap().right;
    let wrench = find(&mut app, "wrench");
    click(&mut app, victim, wrench);
    assert_eq!(app.world().get::<Hands>(victim).unwrap().right, before);

    // Walking pushes the door open.
    app.world_mut().get_mut::<MoveIntent>(victim).unwrap().dir = Some(Dir::East);
    ticks(&mut app, 2);
    app.world_mut().get_mut::<MoveIntent>(victim).unwrap().dir = None;
    assert!(closet(&app, c).opened);
    assert!(app.world().get::<InCloset>(victim).is_none());
    assert!(app.world().get::<Hidden>(victim).is_none());
}

#[test]
fn a_welded_closet_wont_budge() {
    let mut app = setup();
    let c = closet_at(&mut app, CLOSET);
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(2, 3, 0));
    let welder = find(&mut app, "welder");
    hold(&mut app, mob, welder);
    app.world_mut().get_mut::<Welder>(welder).unwrap().lit = true;
    let said = click(&mut app, mob, c);
    assert!(said.contains(&"closet-welded".to_owned()), "{said:?}");
    assert!(closet(&app, c).welded);
    assert_eq!(app.world().get::<Appearance>(c).unwrap().state, "closed+welded");

    // Hands free: it won't open.
    app.world_mut().get_mut::<Hands>(mob).unwrap().right = None;
    assert_eq!(click(&mut app, mob, c), ["closet-wont-budge"]);
    assert!(!closet(&app, c).opened);
}

#[test]
fn a_locker_opens_only_for_the_right_card() {
    let mut app = setup();
    let locker = closet_at(&mut app, LOCKER);
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(3, 2, 0));
    assert_eq!(app.world().get::<Appearance>(locker).unwrap().state, "secureeng1");
    assert_eq!(click(&mut app, mob, locker), ["locker-access-denied"]);
    assert!(app.world().get::<ClosetLock>(locker).unwrap().locked);

    let card = find(&mut app, "id-card");
    hold(&mut app, mob, card);
    let said = click(&mut app, mob, locker);
    assert!(said.contains(&"locker-unlocked".to_owned()), "{said:?}");
    assert!(!app.world().get::<ClosetLock>(locker).unwrap().locked);

    // Hands free: it opens, with Luna's engineer kit inside.
    app.world_mut().get_mut::<Hands>(mob).unwrap().right = None;
    click(&mut app, mob, locker);
    assert!(closet(&app, locker).opened);
    assert_eq!(
        items_at(&mut app, LOCKER),
        ["insulated-gloves", "toolbox-mechanical", "utility-belt"]
    );
}

#[test]
fn a_crate_stays_in_the_way_and_takes_no_one_in() {
    let mut app = setup();
    let crate_ = closet_at(&mut app, CRATE);
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(5, 2, 0));
    click(&mut app, mob, crate_);
    assert!(closet(&app, crate_).opened);
    assert!(app.world().get::<Dense>(crate_).is_some());
    // Someone standing in it is left out when it closes.
    let other = spawn_player_mob(app.world_mut(), CRATE);
    click(&mut app, mob, crate_);
    assert!(!closet(&app, crate_).opened);
    assert!(app.world().get::<InCloset>(other).is_none());
}

#[test]
fn a_tool_closet_always_has_the_four_basic_tools() {
    let mut app = setup();
    let tools = closet_at(&mut app, TOOLS);
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(7, 2, 0));
    click(&mut app, mob, tools);
    let inside = items_at(&mut app, TOOLS);
    for tool in ["crowbar", "screwdriver", "wirecutters", "wrench"] {
        assert!(inside.contains(&tool.to_owned()), "{inside:?}");
    }
}
