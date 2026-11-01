//! Blast doors and their buttons. Spec: docs/reference/blast-doors.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::atmos::StationAtmos;
use rs13_classic::blastdoors::{BlastDoor, DoorButton, Shutter};
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_classic::power::{AreaPower, AreaState};
use rs13_world::{Appearance, Dense, Opaque, TileMap, TilePos};

/// A room with a button by the player and a blast door in its east wall, space beyond;
/// a crowbar on the player's tile. All one area, so its power can be cut.
const MAP: &str = r#"(
    legend: {
        ' ': (tile: (id: "space")),
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'c': (tile: (id: "floor"), objects: ["crowbar"]),
        'x': (tile: (id: "floor"), objects: [(id: "door-button", vars: {"id": "vent"})]),
        'B': (tile: (id: "floor"), objects: [(id: "blast-door", vars: {"id": "vent"})]),
    },
    area_legend: { 'a': "lab" },
    area_rows: [
        "      ",
        "aaaaa ",
        "aaaaa ",
        "aaaaa ",
        "      ",
    ],
    rows: [
        "      ",
        "WWWWW ",
        "Wcx.B ",
        "WWWWW ",
        "      ",
    ],
)"#;

const ROOM: TilePos = TilePos::new(2, 2, 0);
const PLAYER: TilePos = TilePos::new(1, 2, 0);

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    spawn_player_mob(app.world_mut(), PLAYER);
    seconds(&mut app, 2);
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

fn act(app: &mut App, action: Action) {
    let mob = one::<PendingActions>(app);
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
    app.world_mut().run_schedule(FixedUpdate);
}

fn door(app: &mut App) -> (Shutter, bool, String) {
    let e = one::<BlastDoor>(app);
    let w = app.world();
    let blocking = w.get::<Dense>(e).is_some() && w.get::<Opaque>(e).is_some();
    (
        w.get::<BlastDoor>(e).unwrap().shutter,
        blocking,
        w.get::<Appearance>(e).unwrap().state.clone(),
    )
}

fn pressure(app: &App) -> f64 {
    app.world().resource::<StationAtmos>().0.pressure_at(ROOM)
}

#[test]
fn the_button_opens_the_door_and_the_room_vents_then_shuts_it() {
    let mut app = setup();
    let (shutter, blocking, look) = door(&mut app);
    assert_eq!(
        (shutter, blocking, look.as_str()),
        (Shutter::Closed, true, "pdoor1")
    );
    let full = pressure(&app);
    seconds(&mut app, 5);
    assert!((pressure(&app) - full).abs() < 0.1, "shut, it holds the air");

    let button = one::<DoorButton>(&mut app);
    act(&mut app, Action::UseOn(button));
    let (shutter, blocking, look) = door(&mut app);
    assert!(matches!(shutter, Shutter::Opening(_)), "{shutter:?}");
    assert!(blocking, "still in the way while it opens");
    assert_eq!(look, "pdoorc0");
    seconds(&mut app, 2);
    let (shutter, blocking, look) = door(&mut app);
    assert_eq!(
        (shutter, blocking, look.as_str()),
        (Shutter::Open, false, "pdoor0")
    );
    seconds(&mut app, 5);
    assert!(
        pressure(&app) < full / 2.0,
        "open, the room vents: {}",
        pressure(&app)
    );

    // The wind has blown the player off: back by the button.
    let mob = one::<PendingActions>(&mut app);
    app.world_mut()
        .entity_mut(mob)
        .insert(rs13_world::GridPos(PLAYER));
    act(&mut app, Action::UseOn(button));
    let (shutter, blocking, _) = door(&mut app);
    assert!(matches!(shutter, Shutter::Closing(_)), "{shutter:?}");
    assert!(blocking, "it blocks at once as it closes");
    seconds(&mut app, 2);
    assert_eq!(door(&mut app).0, Shutter::Closed);
}

#[test]
fn without_power_the_button_does_nothing_but_a_crowbar_pries_it_open() {
    let mut app = setup();
    app.world_mut()
        .resource_mut::<AreaPower>()
        .0
        .insert("lab".to_owned(), AreaState::default());
    seconds(&mut app, 2);
    let button = one::<DoorButton>(&mut app);
    act(&mut app, Action::UseOn(button));
    seconds(&mut app, 2);
    assert_eq!(door(&mut app).0, Shutter::Closed);

    let crowbar = find(&mut app, "crowbar");
    act(&mut app, Action::UseOn(crowbar));
    // The door is out of reach from the player's tile: step next to it first.
    let mob = one::<PendingActions>(&mut app);
    app.world_mut()
        .entity_mut(mob)
        .insert(rs13_world::GridPos(TilePos::new(3, 2, 0)));
    let blast = one::<BlastDoor>(&mut app);
    act(&mut app, Action::UseOn(blast));
    seconds(&mut app, 2);
    assert_eq!(door(&mut app).0, Shutter::Open);
}

#[test]
fn with_power_a_crowbar_does_nothing() {
    let mut app = setup();
    let crowbar = find(&mut app, "crowbar");
    act(&mut app, Action::UseOn(crowbar));
    let mob = one::<PendingActions>(&mut app);
    app.world_mut()
        .entity_mut(mob)
        .insert(rs13_world::GridPos(TilePos::new(3, 2, 0)));
    let blast = one::<BlastDoor>(&mut app);
    act(&mut app, Action::UseOn(blast));
    seconds(&mut app, 2);
    assert_eq!(door(&mut app).0, Shutter::Closed);
}
