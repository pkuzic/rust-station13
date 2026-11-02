//! Flashes and flashers. Spec: docs/reference/flashes.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::equipment::{Equipment, WornBy};
use rs13_classic::flashes::{BURNT, Dazzled, FLASH_WEAKEN_DS, Flasher, Weakened, fire_flasher};
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::movement::MoveIntent;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_world::{Appearance, Dir, GridPos, ObjectSpec, Slot, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'f': (tile: (id: "floor"), objects: [(id: "flasher", vars: {"id": "Cell 1"})]),
        'p': (tile: (id: "floor"), objects: ["portable-flasher"]),
    },
    rows: [
        "WWWWWWWWWWW",
        "W.........W",
        "W.f.....p.W",
        "W.........W",
        "WWWWWWWWWWW",
    ],
)"#;

const AT: TilePos = TilePos::new(5, 1, 0);
const NEXT: TilePos = TilePos::new(6, 1, 0);

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().run_schedule(FixedUpdate);
    app
}

fn ticks(app: &mut App, n: u32) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn holding(app: &mut App, mob: Entity, proto: &str) -> Entity {
    let item = spawn_object(app.world_mut(), &ObjectSpec::new(proto), AT).unwrap();
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    let mut hands = app.world_mut().get_mut::<Hands>(mob).unwrap();
    hands.right = Some(item);
    hands.active = rs13_world::Hand::Right;
    item
}

fn click(app: &mut App, mob: Entity, target: Entity) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    ticks(app, 1);
}

fn messages(app: &App, who: Entity) -> Vec<String> {
    let outbox = app.world().get::<Outbox>(who).unwrap();
    outbox.0.iter().map(|m| m.id.clone()).collect()
}

fn flasher(app: &mut App, portable: bool) -> Entity {
    let mut q = app.world_mut().query::<(Entity, &Flasher)>();
    q.iter(app.world())
        .find(|(_, f)| f.portable == portable)
        .unwrap()
        .0
}

#[test]
fn a_flash_in_the_face_knocks_someone_down_and_they_get_up_again() {
    let mut app = setup();
    let (user, victim) = (
        spawn_player_mob(app.world_mut(), AT),
        spawn_player_mob(app.world_mut(), NEXT),
    );
    let flash = holding(&mut app, user, "flash");
    click(&mut app, user, victim);
    assert!(app.world().get::<Weakened>(victim).is_some());
    assert_eq!(app.world().get::<Dazzled>(victim).unwrap().0, 1);
    ticks(&mut app, 1);
    assert_eq!(app.world().get::<Appearance>(victim).unwrap().state, "lying");
    assert!(messages(&app, user).contains(&"flash-blinds".to_owned()));
    assert!(app.world().get::<GridPos>(flash).is_none());

    // Down: no moving and no acting.
    app.world_mut().get_mut::<MoveIntent>(victim).unwrap().dir = Some(Dir::South);
    ticks(&mut app, 20);
    assert_eq!(app.world().get::<GridPos>(victim).unwrap().0, NEXT);
    app.world_mut().get_mut::<MoveIntent>(victim).unwrap().dir = None;

    // 20 ticks a second, so 2 ticks a decisecond.
    ticks(&mut app, (FLASH_WEAKEN_DS * 2.0) as u32);
    assert!(app.world().get::<Weakened>(victim).is_none());
    assert_eq!(app.world().get::<Appearance>(victim).unwrap().state, "");
}

#[test]
fn sunglasses_save_your_eyes() {
    let mut app = setup();
    let (user, victim) = (
        spawn_player_mob(app.world_mut(), AT),
        spawn_player_mob(app.world_mut(), NEXT),
    );
    let glasses = spawn_object(app.world_mut(), &ObjectSpec::new("sunglasses"), NEXT).unwrap();
    app.world_mut()
        .entity_mut(glasses)
        .remove::<GridPos>()
        .insert(WornBy(victim));
    app.world_mut()
        .get_mut::<Equipment>(victim)
        .unwrap()
        .0
        .insert(Slot::Glasses, glasses);
    holding(&mut app, user, "flash");
    click(&mut app, user, victim);
    assert!(app.world().get::<Weakened>(victim).is_none());
    assert!(app.world().get::<Dazzled>(victim).is_none());
}

#[test]
fn a_burnt_flash_only_clicks() {
    let mut app = setup();
    let (user, victim) = (
        spawn_player_mob(app.world_mut(), AT),
        spawn_player_mob(app.world_mut(), NEXT),
    );
    let flash = holding(&mut app, user, "flash");
    app.world_mut().get_mut::<Appearance>(flash).unwrap().state = BURNT.to_owned();
    click(&mut app, user, victim);
    assert!(app.world().get::<Weakened>(victim).is_none());
    assert!(messages(&app, user).contains(&"flash-click".to_owned()));
}

#[test]
fn fired_into_the_air_it_dazzles_those_close_but_keeps_them_up() {
    let mut app = setup();
    let (user, near) = (
        spawn_player_mob(app.world_mut(), AT),
        spawn_player_mob(app.world_mut(), NEXT),
    );
    let flash = holding(&mut app, user, "flash");
    click(&mut app, user, flash);
    assert_eq!(app.world().get::<Dazzled>(near).unwrap().0, 1);
    assert!(app.world().get::<Weakened>(near).is_none());
    assert!(app.world().get::<Dazzled>(user).is_none());
}

#[test]
fn a_mounted_flasher_needs_its_bulb_and_rests_between_flashes() {
    let mut app = setup();
    let near = spawn_player_mob(app.world_mut(), TilePos::new(4, 2, 0));
    let far = spawn_player_mob(app.world_mut(), TilePos::new(9, 1, 0));
    let f = flasher(&mut app, false);

    let cutter = spawn_player_mob(app.world_mut(), TilePos::new(2, 3, 0));
    holding(&mut app, cutter, "wirecutters");
    click(&mut app, cutter, f);
    assert!(app.world().get::<Flasher>(f).unwrap().disabled);
    fire_flasher(app.world_mut(), f);
    assert!(app.world().get::<Weakened>(near).is_none(), "no bulb");
    click(&mut app, cutter, f);
    assert!(!app.world().get::<Flasher>(f).unwrap().disabled);

    fire_flasher(app.world_mut(), f);
    assert!(app.world().get::<Weakened>(near).is_some());
    assert!(app.world().get::<Weakened>(far).is_none(), "out of reach");

    // Too soon to go off again.
    app.world_mut().entity_mut(near).remove::<Weakened>();
    fire_flasher(app.world_mut(), f);
    assert!(app.world().get::<Weakened>(near).is_none());
}

#[test]
fn a_portable_flasher_catches_runners_only_once_wrenched_down() {
    let mut app = setup();
    let spot = TilePos::new(7, 2, 0);
    let runner = spawn_player_mob(app.world_mut(), spot);
    let f = flasher(&mut app, true);
    app.world_mut().get_mut::<MoveIntent>(runner).unwrap().dir = Some(Dir::West);
    ticks(&mut app, 1);
    assert!(
        app.world().get::<Weakened>(runner).is_none(),
        "loose: it does nothing"
    );
    app.world_mut().get_mut::<MoveIntent>(runner).unwrap().dir = None;
    app.world_mut().get_mut::<GridPos>(runner).unwrap().0 = spot;

    let fitter = spawn_player_mob(app.world_mut(), TilePos::new(8, 1, 0));
    holding(&mut app, fitter, "wrench");
    click(&mut app, fitter, f);
    assert!(app.world().get::<Flasher>(f).unwrap().anchored);
    assert_eq!(
        app.world().get::<Appearance>(f).unwrap().state,
        "pflash1+pflash-s"
    );

    // Walking past slips by.
    {
        let mut intent = app.world_mut().get_mut::<MoveIntent>(runner).unwrap();
        intent.dir = Some(Dir::North);
        intent.run = false;
    }
    ticks(&mut app, 1);
    assert!(app.world().get::<Weakened>(runner).is_none());

    let mut intent = app.world_mut().get_mut::<MoveIntent>(runner).unwrap();
    intent.run = true;
    ticks(&mut app, 1);
    assert!(app.world().get::<Weakened>(runner).is_some());
}
