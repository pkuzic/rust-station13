//! Deployable barriers. Spec: docs/reference/security.md.

use bevy_app::{App, FixedUpdate};
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::security::Barrier;
use rs13_world::{Appearance, Dense, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: { 'W': (tile: (id: "wall", solid: true)), '.': (tile: (id: "floor")) },
    rows: ["WWWWW", "W...W", "WWWWW"],
)"#;
const HERE: TilePos = TilePos::new(1, 1, 0);

#[test]
fn any_card_locks_and_unlocks_one_and_nothing_else_does() {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), HERE);
    let barrier = spawn_object(
        app.world_mut(),
        &ObjectSpec::new("barrier"),
        TilePos::new(2, 1, 0),
    )
    .unwrap();
    let card = spawn_object(app.world_mut(), &ObjectSpec::new("id-card"), HERE).unwrap();
    app.world_mut().run_schedule(FixedUpdate);
    let click = |app: &mut App, target| {
        app.world_mut()
            .get_mut::<PendingActions>(mob)
            .unwrap()
            .0
            .push(Action::UseOn(target));
        app.world_mut().run_schedule(FixedUpdate);
    };
    click(&mut app, barrier);
    assert!(
        !app.world().get::<Barrier>(barrier).unwrap().locked,
        "an empty hand does nothing"
    );

    click(&mut app, card);
    click(&mut app, barrier);
    assert!(app.world().get::<Barrier>(barrier).unwrap().locked);
    assert_eq!(app.world().get::<Appearance>(barrier).unwrap().state, "barrier1");
    assert_eq!(
        app.world().get::<Outbox>(mob).unwrap().0.last().unwrap().id,
        "barrier-locked"
    );
    assert!(app.world().get::<Dense>(barrier).is_some());

    click(&mut app, barrier);
    assert!(!app.world().get::<Barrier>(barrier).unwrap().locked);
    assert_eq!(app.world().get::<Appearance>(barrier).unwrap().state, "barrier0");
}
