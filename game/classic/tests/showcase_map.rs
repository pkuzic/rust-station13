//! The graphics showcase map (`game/classic/maps/graphics_showcase.ron`) loads cleanly.

use bevy_app::App;
use rs13_classic::ClassicPlugin;
use rs13_classic::objects::spawn_map;
use rs13_world::TileMap;

#[test]
fn the_showcase_map_loads_with_every_object_known() {
    let text = include_str!("../maps/graphics_showcase.ron");
    let map = TileMap::from_ron(text, 0).unwrap();
    assert_eq!(map.spawns.len(), 2, "one in the lit room, one in the dark");
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), map);
    assert!(problems.is_empty(), "{problems:?}");
}
