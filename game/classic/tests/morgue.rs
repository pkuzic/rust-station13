//! Morgue drawers. Spec: docs/reference/morgue.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::closets::InCloset;
use rs13_classic::morgue::{Morgue, MorgueTray};
use rs13_classic::movement::MoveIntent;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::pulling::Pulling;
use rs13_world::{Appearance, Dir, GridPos, Hidden, ObjectSpec, TileMap, TilePos};

/// One drawer with room for its tray to the east, and one against the wall.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'm': (tile: (id: "floor"), objects: ["morgue"]),
    },
    rows: [
        "WWWWWW",
        "Wm...W",
        "W...mW",
        "WWWWWW",
    ],
)"#;

const DRAWER: TilePos = TilePos::new(1, 2, 0);
const TRAY: TilePos = TilePos::new(2, 2, 0);
const BY_DRAWER: TilePos = TilePos::new(2, 1, 0);
const WALLED: TilePos = TilePos::new(4, 1, 0);

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), BY_DRAWER);
    app.world_mut().run_schedule(FixedUpdate);
    (app, mob)
}

fn drawer_at(app: &mut App, at: TilePos) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &GridPos, &Morgue)>();
    q.iter(app.world()).find(|(_, p, _)| p.0 == at).unwrap().0
}

fn click(app: &mut App, mob: Entity, target: Entity) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(FixedUpdate);
}

fn tray(app: &App, drawer: Entity) -> Option<Entity> {
    app.world().get::<Morgue>(drawer).unwrap().tray
}

#[test]
fn the_tray_slides_out_and_takes_what_lies_on_it_back_in() {
    let (mut app, mob) = setup();
    let drawer = drawer_at(&mut app, DRAWER);
    click(&mut app, mob, drawer);
    let out = tray(&app, drawer).expect("tray out");
    assert_eq!(app.world().get::<GridPos>(out).unwrap().0, TRAY);
    assert_eq!(app.world().get::<Appearance>(drawer).unwrap().state, "morgue0");

    // A body pulled onto the tray, and a crowbar lying on it, go in when it shuts.
    let body = spawn_player_mob(app.world_mut(), TilePos::new(3, 1, 0));
    app.world_mut().entity_mut(mob).insert(Pulling(body));
    click(&mut app, mob, out);
    assert_eq!(app.world().get::<GridPos>(body).unwrap().0, TRAY);
    assert!(app.world().get::<Pulling>(mob).is_none());
    let crowbar = spawn_object(app.world_mut(), &ObjectSpec::new("crowbar"), TRAY).unwrap();
    click(&mut app, mob, out);
    assert_eq!(tray(&app, drawer), None);
    assert!(app.world().get_entity(out).is_err(), "the tray is gone");
    assert_eq!(app.world().get::<InCloset>(body).unwrap().0, drawer);
    assert!(app.world().get::<Hidden>(body).is_some());
    assert!(app.world().get::<GridPos>(crowbar).is_none());
    assert_eq!(app.world().get::<Appearance>(drawer).unwrap().state, "morgue2");

    // Awake inside, trying to walk out brings the tray out with them on it.
    app.world_mut().get_mut::<MoveIntent>(body).unwrap().dir = Some(Dir::North);
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().get_mut::<MoveIntent>(body).unwrap().dir = None;
    assert!(tray(&app, drawer).is_some());
    assert_eq!(app.world().get::<GridPos>(body).unwrap().0, TRAY);
    assert!(app.world().get::<Hidden>(body).is_none());
    assert_eq!(app.world().get::<GridPos>(crowbar).unwrap().0, TRAY);
}

#[test]
fn a_drawer_against_a_wall_has_nowhere_to_put_its_tray() {
    let (mut app, _) = setup();
    let walled = drawer_at(&mut app, WALLED);
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(3, 1, 0));
    click(&mut app, mob, walled);
    assert_eq!(tray(&app, walled), None);
    let mut trays = app.world_mut().query::<&MorgueTray>();
    assert_eq!(trays.iter(app.world()).count(), 0);
}
