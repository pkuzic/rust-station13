//! Window doors. Spec: docs/reference/windoors.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::atmos::StationAtmos;
use rs13_classic::movement::MoveIntent;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::windoors::{Swing, Windoor};
use rs13_world::{Appearance, Dir, GridPos, ObjectSpec, TileMap, TilePos};

/// Two corridors, the upper one with a window door anyone may use and the lower one with
/// one that wants bar access, each on the east edge of column 2. A wall between the
/// rows keeps them apart.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'd': (tile: (id: "floor"), objects: [(id: "windoor", dir: East, vars: {"base_state": "right"})]),
        's': (tile: (id: "floor"), objects: [(id: "brig-windoor", dir: East, access: ["bar"])]),
    },
    rows: [
        "WWWWWW",
        "W.d..W",
        "WWWWWW",
        "W.s..W",
        "WWWWWW",
    ],
)"#;

// Rows count up from the bottom.
const FREE: TilePos = TilePos::new(2, 3, 0);
const SECURE: TilePos = TilePos::new(2, 1, 0);

fn setup(at: TilePos) -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), at);
    seconds(&mut app, 0.25);
    (app, mob)
}

fn seconds(app: &mut App, s: f32) {
    for _ in 0..(s * 20.0).round() as usize {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn door_at(app: &mut App, at: TilePos) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &GridPos, &Windoor)>();
    q.iter(app.world()).find(|(_, p, _)| p.0 == at).unwrap().0
}

fn swing(app: &App, door: Entity) -> Swing {
    app.world().get::<Windoor>(door).unwrap().swing
}

fn state(app: &App, door: Entity) -> String {
    app.world().get::<Appearance>(door).unwrap().state.clone()
}

fn walk(app: &mut App, mob: Entity, dir: Dir, s: f32) -> TilePos {
    app.world_mut().get_mut::<MoveIntent>(mob).unwrap().dir = Some(dir);
    seconds(app, s);
    app.world_mut().get_mut::<MoveIntent>(mob).unwrap().dir = None;
    seconds(app, 0.1);
    app.world().get::<GridPos>(mob).unwrap().0
}

fn click(app: &mut App, mob: Entity, target: Entity) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    seconds(app, 0.1);
}

fn pressure(app: &App, pos: TilePos) -> f64 {
    app.world().resource::<StationAtmos>().0.pressure_at(pos)
}

fn empty(app: &mut App, pos: TilePos) {
    let mut atmos = app.world_mut().resource_mut::<StationAtmos>();
    let all = atmos.0.air_at(pos).total_moles() * 10.0;
    atmos.0.remove_at(pos, all);
}

#[test]
fn walking_into_a_window_door_opens_it_and_it_shuts_by_itself() {
    let (mut app, mob) = setup(FREE);
    let door = door_at(&mut app, FREE);
    assert_eq!(state(&app, door), "right-east");
    // Bumping it starts it opening; it lets nobody through until it has.
    assert_eq!(walk(&mut app, mob, Dir::East, 0.3), FREE);
    assert!(matches!(swing(&app, door), Swing::Opening { .. }));
    assert_eq!(state(&app, door), "rightopening-east");
    seconds(&mut app, 1.0);
    assert!(matches!(swing(&app, door), Swing::Open { .. }));
    assert_eq!(state(&app, door), "rightopen-east");
    assert!(walk(&mut app, mob, Dir::East, 0.3).x > FREE.x, "through");
    // Ten seconds later it closes again, and the way back is shut.
    seconds(&mut app, 10.0);
    assert!(matches!(swing(&app, door), Swing::Closing { .. } | Swing::Closed));
    seconds(&mut app, 1.0);
    assert_eq!(swing(&app, door), Swing::Closed);
    assert_eq!(state(&app, door), "right-east");
}

#[test]
fn a_shut_window_door_holds_the_air_and_an_open_one_lets_it_through() {
    let (mut app, mob) = setup(FREE);
    let door = door_at(&mut app, FREE);
    let east = TilePos::new(4, 3, 0);
    empty(&mut app, east);
    seconds(&mut app, 2.0);
    assert!(pressure(&app, east) < 1.0, "sealed: {}", pressure(&app, east));
    click(&mut app, mob, door);
    seconds(&mut app, 3.0);
    assert!(
        pressure(&app, east) > 20.0,
        "air came through: {}",
        pressure(&app, east)
    );
}

#[test]
fn a_secure_window_door_wants_its_access() {
    let (mut app, mob) = setup(SECURE);
    let door = door_at(&mut app, SECURE);
    assert_eq!(state(&app, door), "leftsecure-east");
    // Without the card: walking in does nothing, and clicking shows it is denied.
    assert_eq!(walk(&mut app, mob, Dir::East, 0.5), SECURE);
    assert_eq!(swing(&app, door), Swing::Closed);
    click(&mut app, mob, door);
    assert_eq!(swing(&app, door), Swing::Closed);
    assert_eq!(state(&app, door), "leftsecuredeny-east");

    // With a bar card in hand, a click opens it and another shuts it.
    let card = spawn_object(
        app.world_mut(),
        &ObjectSpec {
            access: vec!["bar".to_owned()],
            ..ObjectSpec::new("id-card")
        },
        SECURE,
    )
    .unwrap();
    click(&mut app, mob, card);
    click(&mut app, mob, door);
    assert!(matches!(swing(&app, door), Swing::Opening { .. }));
    seconds(&mut app, 1.1);
    assert!(matches!(swing(&app, door), Swing::Open { .. }));
    click(&mut app, mob, door);
    assert!(matches!(swing(&app, door), Swing::Closing { .. }));
    seconds(&mut app, 1.1);
    // Walking into it with the card opens it, and it shuts again after four seconds.
    assert!(walk(&mut app, mob, Dir::East, 1.5).x > SECURE.x, "through");
    seconds(&mut app, 4.0);
    assert!(!matches!(swing(&app, door), Swing::Open { .. }));
}
