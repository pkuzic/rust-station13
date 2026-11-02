//! Security cameras and their consoles. Spec: docs/reference/cameras.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::cameras::{Camera, CameraPanelOut, PendingCameraOps, SecurityConsole};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_world::{CameraView, GridPos, ObjectSpec, TileMap, TilePos};

/// A console in one room; two named cameras and an unnamed one around the corridor.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'k': (tile: (id: "floor"), objects: ["security-console"]),
        'a': (tile: (id: "floor"), objects: [(id: "camera", dir: South, vars: {"c_tag": "Bar"})]),
        'b': (tile: (id: "floor"), objects: [(id: "camera", dir: South, vars: {"c_tag": "Armory"})]),
        'n': (tile: (id: "floor"), objects: [(id: "camera", dir: South, vars: {"c_tag": "null"})]),
    },
    rows: [
        "WWWWWWWWWWW",
        "Wk..W.a.b.W",
        "W...W..n..W",
        "WWWWWWWWWWW",
    ],
)"#;

fn setup() -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().run_schedule(FixedUpdate);
    let console = app
        .world_mut()
        .query_filtered::<Entity, With<SecurityConsole>>()
        .single(app.world())
        .unwrap();
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(2, 2, 0));
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(console));
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(FixedUpdate);
    (app, console, mob)
}

fn watch(app: &mut App, mob: Entity, console: Entity, line: Option<usize>) {
    app.world_mut()
        .get_mut::<PendingCameraOps>(mob)
        .unwrap()
        .0
        .push((console, line));
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(FixedUpdate);
}

#[test]
fn the_console_lists_named_cameras_and_shows_from_the_one_picked() {
    let (mut app, console, mob) = setup();
    let view = app
        .world()
        .get::<CameraPanelOut>(mob)
        .unwrap()
        .0
        .clone()
        .expect("open");
    let tags: Vec<&str> = view.cameras.iter().map(|c| c.tag.as_str()).collect();
    assert_eq!(tags, ["Armory", "Bar"], "sorted, the unnamed one left out");
    assert_eq!(view.current, None);

    watch(&mut app, mob, console, Some(1));
    assert_eq!(
        app.world().get::<CameraView>(mob),
        Some(&CameraView(TilePos::new(6, 2, 0)))
    );
    let view = app.world().get::<CameraPanelOut>(mob).unwrap().0.clone().unwrap();
    assert_eq!(view.current, Some(1));

    watch(&mut app, mob, console, None);
    assert!(app.world().get::<CameraView>(mob).is_none(), "cancelled");
}

#[test]
fn stepping_away_ends_the_view() {
    let (mut app, console, mob) = setup();
    watch(&mut app, mob, console, Some(0));
    assert!(app.world().get::<CameraView>(mob).is_some());
    app.world_mut()
        .entity_mut(mob)
        .insert(GridPos(TilePos::new(3, 1, 0)));
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(FixedUpdate);
    assert!(app.world().get::<CameraView>(mob).is_none());
    assert!(app.world().get::<CameraPanelOut>(mob).unwrap().0.is_none());
}

#[test]
fn cutting_a_camera_turns_the_screen_to_static() {
    let (mut app, console, mob) = setup();
    watch(&mut app, mob, console, Some(1));
    let bar = {
        let mut q = app.world_mut().query::<(Entity, &Camera)>();
        q.iter(app.world())
            .find(|(_, c)| c.tag.as_deref() == Some("Bar"))
            .unwrap()
            .0
    };
    // Someone with wirecutters beside it.
    let cutter = spawn_player_mob(app.world_mut(), TilePos::new(6, 1, 0));
    let cutters = spawn_object(
        app.world_mut(),
        &ObjectSpec::new("wirecutters"),
        TilePos::new(6, 1, 0),
    )
    .unwrap();
    for target in [cutters, bar] {
        app.world_mut()
            .get_mut::<PendingActions>(cutter)
            .unwrap()
            .0
            .push(Action::UseOn(target));
        app.world_mut().run_schedule(FixedUpdate);
    }
    app.world_mut().run_schedule(FixedUpdate);
    assert!(!app.world().get::<Camera>(bar).unwrap().on);
    assert!(app.world().get::<CameraView>(mob).is_none(), "the view is lost");
    let said: Vec<String> = app
        .world()
        .get::<Outbox>(mob)
        .unwrap()
        .0
        .iter()
        .map(|m| m.id.clone())
        .collect();
    assert!(said.contains(&"camera-static".to_owned()), "{said:?}");
    let view = app.world().get::<CameraPanelOut>(mob).unwrap().0.clone().unwrap();
    assert!(!view.cameras[1].on, "listed as deactivated");
}
