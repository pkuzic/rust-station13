//! The closets and lockers on Luna's decks. Spec: docs/reference/closets.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::closets::{Closet, InCloset};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_world::{Appearance, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: { 'W': (tile: (id: "wall", solid: true)), '.': (tile: (id: "floor")) },
    rows: ["WWWWW", "W...W", "W...W", "WWWWW"],
)"#;
const AT: TilePos = TilePos::new(1, 1, 0);

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app
}

fn contents(app: &mut App, closet: Entity) -> Vec<String> {
    let mut q = app.world_mut().query::<(&InCloset, &Appearance)>();
    q.iter(app.world())
        .filter(|(c, _)| c.0 == closet)
        .map(|(_, a)| a.sprite.clone())
        .collect()
}

#[test]
fn every_kind_can_be_made() {
    let mut app = setup();
    for proto in include_str!("luna_closets_ids.txt").lines() {
        let closet = spawn_object(app.world_mut(), &ObjectSpec::new(proto), AT).expect(proto);
        assert!(app.world().get::<Closet>(closet).is_some(), "{proto}");
    }
}

#[test]
fn a_wardrobe_holds_what_we_have_of_its_clothes_and_opens() {
    let mut app = setup();
    let wardrobe = spawn_object(app.world_mut(), &ObjectSpec::new("wardrobe-black"), AT).unwrap();
    let held = contents(&mut app, wardrobe);
    assert!(held.iter().any(|s| s == "black-shoes"), "{held:?}");
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(2, 1, 0));
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(wardrobe));
    app.world_mut().run_schedule(FixedUpdate);
    assert!(contents(&mut app, wardrobe).is_empty(), "what it held came out");
}

#[test]
fn a_locker_stays_shut_without_its_access() {
    let mut app = setup();
    let locker = spawn_object(app.world_mut(), &ObjectSpec::new("locker-captain"), AT).unwrap();
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(2, 1, 0));
    let closed = app.world().get::<Appearance>(locker).unwrap().state.clone();
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(locker));
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(app.world().get::<Appearance>(locker).unwrap().state, closed);
}
