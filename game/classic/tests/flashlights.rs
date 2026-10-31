//! Flashlights and desk lamps. Spec: docs/reference/flashlights.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::closets::InCloset;
use rs13_classic::equipment::WornBy;
use rs13_classic::flashlights::Flashlight;
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_world::{Appearance, GridPos, LightSource, TileMap, TilePos};

/// A dark room with a flashlight, a lantern and a desk lamp.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "plating")),
        'f': (tile: (id: "plating"), objects: ["flashlight"]),
        'n': (tile: (id: "plating"), objects: ["lantern"]),
        'd': (tile: (id: "plating"), objects: ["desk-lamp"]),
    },
    rows: [
        "WWWWWWW",
        "Wf.n.dW",
        "W.....W",
        "WWWWWWW",
    ],
)"#;

const START: TilePos = TilePos::new(1, 1, 0);

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), START);
    ticks(&mut app, 2);
    (app, mob)
}

fn ticks(app: &mut App, n: u64) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn find(app: &mut App, sprite: &str) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &Appearance)>();
    q.iter(app.world())
        .find(|(_, a)| a.sprite == sprite)
        .map(|(e, _)| e)
        .unwrap_or_else(|| panic!("no {sprite}"))
}

fn act(app: &mut App, mob: Entity, action: Action) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
    ticks(app, 2);
}

fn hold(app: &mut App, mob: Entity, item: Entity) {
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    let mut hands = app.world_mut().get_mut::<Hands>(mob).unwrap();
    if hands.right.is_none() {
        hands.right = Some(item);
    } else {
        hands.left = Some(item);
    }
}

fn source(app: &App, entity: Entity) -> Option<[u8; 3]> {
    app.world().get::<LightSource>(entity).map(|s| s.0)
}

#[test]
fn a_flashlight_lights_whoever_holds_it() {
    let (mut app, mob) = setup();
    let flashlight = find(&mut app, "flashlight");
    assert_eq!(source(&app, flashlight), None, "flashlights start off");

    hold(&mut app, mob, flashlight);
    act(&mut app, mob, Action::UseOn(flashlight));
    assert!(app.world().get::<Flashlight>(flashlight).unwrap().on);
    assert_eq!(app.world().get::<Appearance>(flashlight).unwrap().state, "on");
    // Luna's 4, 4, 4, from where the holder stands.
    assert_eq!(source(&app, mob), Some([4, 4, 4]));
    assert_eq!(source(&app, flashlight), None);

    // Dropped, it lights its own tile.
    act(&mut app, mob, Action::Drop);
    assert_eq!(source(&app, mob), None);
    assert_eq!(source(&app, flashlight), Some([4, 4, 4]));

    // Switched off in hand, nothing.
    hold(&mut app, mob, flashlight);
    act(&mut app, mob, Action::UseOn(flashlight));
    assert_eq!(app.world().get::<Appearance>(flashlight).unwrap().state, "");
    assert_eq!(source(&app, mob), None);
    assert_eq!(source(&app, flashlight), None);
}

#[test]
fn carried_lights_add_up_and_belts_count() {
    let (mut app, mob) = setup();
    let flashlight = find(&mut app, "flashlight");
    let lantern = find(&mut app, "lantern");
    for item in [flashlight, lantern] {
        app.world_mut().get_mut::<Flashlight>(item).unwrap().on = true;
    }
    hold(&mut app, mob, flashlight);
    hold(&mut app, mob, lantern);
    ticks(&mut app, 1);
    assert_eq!(
        source(&app, mob),
        Some([10, 10, 9]),
        "4, 4, 4 and the lantern's 6, 6, 5"
    );

    // Worn on the belt rather than held, the flashlight still lights its wearer.
    app.world_mut()
        .entity_mut(flashlight)
        .remove::<HeldBy>()
        .insert(WornBy(mob));
    ticks(&mut app, 1);
    assert_eq!(source(&app, mob), Some([10, 10, 9]));
}

#[test]
fn desk_lamps_start_lit_and_closets_keep_light_in() {
    let (mut app, mob) = setup();
    let lamp = find(&mut app, "desk-lamp");
    assert_eq!(source(&app, lamp), Some([6, 6, 6]));
    assert_eq!(app.world().get::<Appearance>(lamp).unwrap().state, "on");

    let flashlight = find(&mut app, "flashlight");
    app.world_mut().get_mut::<Flashlight>(flashlight).unwrap().on = true;
    hold(&mut app, mob, flashlight);
    ticks(&mut app, 1);
    assert!(source(&app, mob).is_some());
    let closet = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(mob).insert(InCloset(closet));
    ticks(&mut app, 1);
    assert_eq!(source(&app, mob), None, "no light gets out of a closet");
}
