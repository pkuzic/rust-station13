//! The laundromat. Spec: docs/reference/laundry.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::chemistry::Reagents;
use rs13_classic::forensics::Traces;
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::washers::{InWasher, WASH_DS, Washer};
use rs13_world::{Appearance, GridPos, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
    },
    rows: [
        "WWWWWW",
        "W....W",
        "WWWWWW",
    ],
)"#;

const HERE: TilePos = TilePos::new(2, 1, 0);
const NEXT: TilePos = TilePos::new(3, 1, 0);

fn setup() -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    tick(&mut app, 1);
    let mob = spawn_player_mob(app.world_mut(), HERE);
    let washer = spawn(&mut app, "washer", NEXT);
    tick(&mut app, 2);
    (app, mob, washer)
}

fn tick(app: &mut App, n: u32) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn act(app: &mut App, mob: Entity, action: Action) -> Vec<String> {
    let before = app.world().get::<Outbox>(mob).unwrap().0.len();
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
    tick(app, 2);
    app.world().get::<Outbox>(mob).unwrap().0[before..]
        .iter()
        .map(|m| m.id.clone())
        .collect()
}

fn spawn(app: &mut App, id: &str, pos: TilePos) -> Entity {
    spawn_object(app.world_mut(), &ObjectSpec::new(id), pos).unwrap()
}

fn give(app: &mut App, mob: Entity, item: Entity) {
    let hand = app.world().get::<Hands>(mob).unwrap().active;
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    *app.world_mut().get_mut::<Hands>(mob).unwrap().slot_mut(hand) = Some(item);
}

fn washer(app: &App, e: Entity) -> Washer {
    app.world().get::<Washer>(e).unwrap().clone()
}

fn printed_gloves(app: &mut App) -> Entity {
    let gloves = spawn(app, "black-gloves", HERE);
    app.world_mut().entity_mut(gloves).insert(Traces {
        prints: vec!["0123".to_owned()],
        fibres: Vec::new(),
    });
    gloves
}

#[test]
fn a_wash_takes_the_prints_off_clothes_and_uses_a_unit_of_cleaner() {
    let (mut app, mob, laundromat) = setup();
    // Nothing to wash with yet.
    assert_eq!(
        act(&mut app, mob, Action::UseOn(laundromat)),
        ["washer-no-cleaner"]
    );

    let beaker = spawn(&mut app, "beaker", HERE);
    app.world_mut()
        .get_mut::<Reagents>(beaker)
        .unwrap()
        .add("cleaner", 10.0);
    give(&mut app, mob, beaker);
    assert_eq!(act(&mut app, mob, Action::UseOn(laundromat)), ["washer-poured"]);
    assert_eq!(washer(&app, laundromat).cleaner, 10.0);
    assert_eq!(app.world().get::<Reagents>(beaker).unwrap().get("cleaner"), 0.0);
    // Anything but clothes is turned away.
    assert_eq!(act(&mut app, mob, Action::UseOn(laundromat)), ["washer-beeps"]);

    let gloves = printed_gloves(&mut app);
    give(&mut app, mob, gloves);
    act(&mut app, mob, Action::UseOn(laundromat));
    assert_eq!(washer(&app, laundromat).contents, [gloves]);
    assert_eq!(app.world().get::<InWasher>(gloves).map(|w| w.0), Some(laundromat));
    assert!(app.world().get::<HeldBy>(gloves).is_none());

    // An empty hand starts it; nothing goes in while it washes.
    let hand = app.world().get::<Hands>(mob).unwrap().active;
    *app.world_mut().get_mut::<Hands>(mob).unwrap().slot_mut(hand) = None;
    act(&mut app, mob, Action::UseOn(laundromat));
    assert!(washer(&app, laundromat).done_at.is_some());
    assert_eq!(
        app.world().get::<Appearance>(laundromat).unwrap().state,
        "landanim"
    );
    let more = printed_gloves(&mut app);
    give(&mut app, mob, more);
    assert_eq!(act(&mut app, mob, Action::UseOn(laundromat)), ["washer-busy"]);
    assert_eq!(
        act(
            &mut app,
            mob,
            Action::Verb(laundromat, rs13_classic::verbs::EMPTY_WASHER)
        ),
        ["washer-locked"]
    );

    tick(&mut app, (WASH_DS as u32 + 1) * 2);
    let done = washer(&app, laundromat);
    assert!(done.done_at.is_none() && done.contents.is_empty());
    assert_eq!(done.cleaner, 9.0);
    assert_eq!(app.world().get::<GridPos>(gloves).map(|p| p.0), Some(NEXT));
    assert!(app.world().get::<Traces>(gloves).unwrap().prints.is_empty());
    assert_eq!(app.world().get::<Appearance>(laundromat).unwrap().state, "");
}

#[test]
fn empty_lets_clothes_out_unwashed() {
    let (mut app, mob, laundromat) = setup();
    let gloves = printed_gloves(&mut app);
    give(&mut app, mob, gloves);
    act(&mut app, mob, Action::UseOn(laundromat));
    assert_eq!(washer(&app, laundromat).contents, [gloves]);
    act(
        &mut app,
        mob,
        Action::Verb(laundromat, rs13_classic::verbs::EMPTY_WASHER),
    );
    assert!(washer(&app, laundromat).contents.is_empty());
    assert_eq!(app.world().get::<GridPos>(gloves).map(|p| p.0), Some(NEXT));
    // Still carrying the old print (and now the hands that put them in).
    assert!(
        app.world()
            .get::<Traces>(gloves)
            .unwrap()
            .prints
            .contains(&"0123".to_owned())
    );
}
