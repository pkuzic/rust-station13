//! Fixtures that only stand there. Spec: docs/reference/scenery.md.

use bevy_app::App;
use rs13_classic::ClassicPlugin;
use rs13_classic::objects::{spawn_map, spawn_object};
use rs13_world::{Dense, ObjectSpec, Opaque, TileMap, TilePos};

const MAP: &str = r#"(
    legend: { 'W': (tile: (id: "wall", solid: true)), '.': (tile: (id: "floor")) },
    rows: ["WWWWW", "W...W", "WWWWW"],
)"#;
const AT: TilePos = TilePos::new(1, 1, 0);

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app
}

#[test]
fn each_blocks_what_it_should() {
    let mut app = setup();
    for (proto, dense, opaque) in [
        ("shuttle-engine", true, true),
        ("bookcase", true, false),
        ("plastic-flaps", false, true),
        ("mirror", false, false),
    ] {
        let thing = spawn_object(app.world_mut(), &ObjectSpec::new(proto), AT).unwrap();
        assert_eq!(app.world().get::<Dense>(thing).is_some(), dense, "{proto}");
        assert_eq!(app.world().get::<Opaque>(thing).is_some(), opaque, "{proto}");
    }
}
