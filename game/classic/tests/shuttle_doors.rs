//! Shuttle doors. Spec: docs/reference/scenery.md.

use bevy_app::{App, FixedUpdate};
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::doors::{Airlock, DoorState};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::power::PowerUser;
use rs13_world::{Appearance, Dense, ObjectSpec, Opaque, TileMap, TilePos};

const MAP: &str = r#"(
    legend: { 'W': (tile: (id: "wall", solid: true)), '.': (tile: (id: "floor")) },
    rows: ["WWWWW", "W...W", "WWWWW"],
)"#;

#[test]
fn a_click_opens_it_without_any_power() {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let door = spawn_object(
        app.world_mut(),
        &ObjectSpec::new("shuttle-door"),
        TilePos::new(2, 1, 0),
    )
    .unwrap();
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(1, 1, 0));
    app.world_mut().run_schedule(FixedUpdate);
    assert!(app.world().get::<PowerUser>(door).is_none());
    assert!(app.world().get::<Dense>(door).is_some());
    assert!(app.world().get::<Opaque>(door).is_some());
    assert_eq!(app.world().get::<Appearance>(door).unwrap().state, "closed");

    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(door));
    for _ in 0..40 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    assert!(matches!(
        app.world().get::<Airlock>(door).unwrap().state,
        DoorState::Open { .. }
    ));
    assert!(app.world().get::<Dense>(door).is_none());
    assert!(app.world().get::<Opaque>(door).is_none());
    assert_eq!(app.world().get::<Appearance>(door).unwrap().state, "open");
}
