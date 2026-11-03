//! Bullet guns. Spec: docs/reference/weapons.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::health::{Body, Zone};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::weapons::{Ammo, Fires, Gun, Recharger, Round, SHOT_COST, load};
use rs13_world::{Appearance, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: { 'W': (tile: (id: "wall", solid: true)), '.': (tile: (id: "floor")) },
    rows: ["WWWWWWWWWWW", "W.........W", "W.........W", "WWWWWWWWWWW"],
)"#;
const HERE: TilePos = TilePos::new(1, 1, 0);
const THERE: TilePos = TilePos::new(6, 1, 0);

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), HERE);
    app.world_mut().run_schedule(FixedUpdate);
    (app, mob)
}

fn spawn(app: &mut App, proto: &str) -> Entity {
    spawn_object(app.world_mut(), &ObjectSpec::new(proto), HERE).unwrap()
}

fn click(app: &mut App, mob: Entity, target: Entity) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    app.world_mut().run_schedule(FixedUpdate);
}

fn rounds(app: &App, gun: Entity) -> i32 {
    app.world().get::<Gun>(gun).unwrap().charge / SHOT_COST
}

fn last_message(app: &App, mob: Entity) -> String {
    app.world()
        .get::<Outbox>(mob)
        .unwrap()
        .0
        .last()
        .unwrap()
        .id
        .clone()
}

fn empty(app: &mut App, gun: Entity) {
    app.world_mut().get_mut::<Gun>(gun).unwrap().charge = 0;
}

#[test]
fn a_gun_comes_loaded_and_shoots_someone_for_its_rounds_damage() {
    let (mut app, mob) = setup();
    let gun = spawn(&mut app, "combat-shotgun");
    click(&mut app, mob, gun);
    assert_eq!(rounds(&app, gun), 8);
    let victim = spawn_player_mob(app.world_mut(), THERE);
    app.world_mut().run_schedule(FixedUpdate);
    click(&mut app, mob, victim);
    assert_eq!(rounds(&app, gun), 7);
    assert_eq!(
        app.world().get::<Appearance>(gun).unwrap().state,
        "cshotgun",
        "still looks the same"
    );
    for _ in 0..30 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    let body = app.world().get::<Body>(victim).unwrap();
    let total: f32 = Zone::ALL.iter().map(|z| body.organ(*z).brute).sum();
    assert_eq!(total, 60.0);
}

#[test]
fn ammunition_loads_what_fits_and_keeps_the_rest() {
    let (mut app, mob) = setup();
    let gun = spawn(&mut app, "revolver");
    app.world_mut().get_mut::<Gun>(gun).unwrap().charge = 4 * SHOT_COST;
    let loader = spawn(&mut app, "speed-loader-38");
    click(&mut app, mob, loader);
    click(&mut app, mob, gun);
    assert_eq!(rounds(&app, gun), 6);
    assert_eq!(app.world().get::<Ammo>(loader).unwrap().rounds, 4);
    assert_eq!(last_message(&app, mob), "gun-loaded");
    load(app.world_mut(), mob, gun);
    assert_eq!(last_message(&app, mob), "gun-full");
}

#[test]
fn the_wrong_caliber_and_a_second_kind_of_round_are_refused() {
    let (mut app, mob) = setup();
    let gun = spawn(&mut app, "double-barrel");
    let wrong = spawn(&mut app, "magazine-9mm");
    click(&mut app, mob, wrong);
    click(&mut app, mob, gun);
    assert_eq!(last_message(&app, mob), "gun-wrong-ammo");

    let (mut app, mob) = setup();
    let gun = spawn(&mut app, "double-barrel");
    let slugs = spawn(&mut app, "slug-box");
    click(&mut app, mob, slugs);
    app.world_mut().get_mut::<Gun>(gun).unwrap().charge = SHOT_COST;
    click(&mut app, mob, gun);
    assert_eq!(last_message(&app, mob), "gun-other-ammo");

    empty(&mut app, gun);
    click(&mut app, mob, gun);
    assert_eq!(rounds(&app, gun), 2);
    assert_eq!(
        app.world().get::<Gun>(gun).unwrap().fires,
        Fires::Bullet(Round::Slug)
    );
}

#[test]
fn an_empty_gun_only_clicks_and_a_recharger_wont_take_it() {
    let (mut app, mob) = setup();
    let gun = spawn(&mut app, "pistol");
    click(&mut app, mob, gun);
    empty(&mut app, gun);
    let victim = spawn_player_mob(app.world_mut(), THERE);
    app.world_mut().run_schedule(FixedUpdate);
    click(&mut app, mob, victim);
    assert_eq!(last_message(&app, mob), "gun-click");

    let recharger = spawn_object(
        app.world_mut(),
        &ObjectSpec::new("recharger"),
        TilePos::new(2, 1, 0),
    )
    .unwrap();
    click(&mut app, mob, recharger);
    assert!(
        app.world()
            .get::<Recharger>(recharger)
            .unwrap()
            .charging
            .is_none()
    );
}
