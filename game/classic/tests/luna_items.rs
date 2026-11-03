//! The simple items on Luna's decks. Spec: docs/reference/items.md.

use bevy_app::{App, FixedUpdate};
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::hands::Hands;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_world::{ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: { 'W': (tile: (id: "wall", solid: true)), '.': (tile: (id: "floor")) },
    rows: ["WWWWW", "W...W", "WWWWW"],
)"#;
const AT: TilePos = TilePos::new(1, 1, 0);

#[test]
fn every_one_can_be_made_and_picked_up() {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let ids = include_str!("luna_items_ids.txt");
    assert!(ids.lines().count() > 50);
    for proto in ids.lines() {
        let mob = spawn_player_mob(app.world_mut(), AT);
        let item = spawn_object(app.world_mut(), &ObjectSpec::new(proto), AT).expect(proto);
        app.world_mut()
            .get_mut::<PendingActions>(mob)
            .unwrap()
            .0
            .push(Action::UseOn(item));
        app.world_mut().run_schedule(FixedUpdate);
        assert_eq!(
            app.world().get::<Hands>(mob).unwrap().active_item(),
            Some(item),
            "{proto}"
        );
        app.world_mut().entity_mut(mob).despawn();
        app.world_mut().entity_mut(item).despawn();
    }
}
