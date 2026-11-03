//! Energy guns, stun batons and rechargers. Spec: docs/reference/weapons.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::combat::CombatIntent;
use rs13_classic::flashes::Weakened;
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::health::Body;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::weapons::{
    BATON_CELL, BATON_HIT, BEAM_BURN, Baton, Fires, GUN_CELL, Gun, Recharger, SHOT_COST, Shot, TASER_CELL,
};
use rs13_world::{Appearance, GridPos, Intent, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        't': (tile: (id: "floor"), objects: ["table"]),
        'r': (tile: (id: "floor"), objects: ["recharger"]),
    },
    rows: [
        "W........W.W",
        "W..........W",
        "W....t.....W",
        "W........W.W",
        "Wr.........W",
        "W........W.W",
    ],
)"#;
const SHOOTER: TilePos = TilePos::new(2, 3, 0);

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let shooter = spawn_player_mob(app.world_mut(), SHOOTER);
    app.world_mut().run_schedule(FixedUpdate);
    (app, shooter)
}

fn ticks(app: &mut App, n: u32) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn give(app: &mut App, mob: Entity, proto: &str) -> Entity {
    let item = spawn_object(app.world_mut(), &ObjectSpec::new(proto), SHOOTER).unwrap();
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    let mut hands = app.world_mut().get_mut::<Hands>(mob).unwrap();
    let active = hands.active;
    *hands.slot_mut(active) = Some(item);
    item
}

fn act(app: &mut App, mob: Entity, action: Action) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
    ticks(app, 1);
}

fn messages(app: &App, who: Entity) -> Vec<String> {
    app.world()
        .get::<Outbox>(who)
        .unwrap()
        .0
        .iter()
        .map(|m| m.id.clone())
        .collect()
}

fn burn(app: &App, who: Entity) -> f32 {
    100.0 - app.world().get::<Body>(who).unwrap().health()
}

fn shots(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<&Shot>();
    q.iter(app.world()).count()
}

#[test]
fn a_laser_burns_who_it_hits_past_a_table_and_uses_charge() {
    let (mut app, shooter) = setup();
    let gun = give(&mut app, shooter, "laser-gun");
    assert_eq!(app.world().get::<Appearance>(gun).unwrap().state, "laser100");
    // Straight along the row with the table in it.
    app.world_mut().get_mut::<GridPos>(shooter).unwrap().0 = TilePos::new(2, 3, 0);
    let target = spawn_player_mob(app.world_mut(), TilePos::new(8, 3, 0));
    act(&mut app, shooter, Action::UseOn(target));
    assert_eq!(shots(&mut app), 1);
    assert_eq!(app.world().get::<Gun>(gun).unwrap().charge, GUN_CELL - SHOT_COST);
    assert!(messages(&app, target).contains(&"gun-fires".to_owned()));
    ticks(&mut app, 30);
    assert_eq!(shots(&mut app), 0, "it stopped");
    assert_eq!(burn(&app, target), BEAM_BURN);
    assert_eq!(burn(&app, shooter), 0.0, "never its shooter");
}

#[test]
fn a_shot_at_the_floor_flies_on_until_a_wall_stops_it() {
    let (mut app, shooter) = setup();
    give(&mut app, shooter, "laser-gun");
    act(&mut app, shooter, Action::UseOnTile(TilePos::new(5, 3, 0)));
    assert_eq!(shots(&mut app), 1);
    ticks(&mut app, 60);
    assert_eq!(shots(&mut app), 0);
}

#[test]
fn someone_behind_a_wall_is_safe() {
    let (mut app, shooter) = setup();
    give(&mut app, shooter, "laser-gun");
    app.world_mut().get_mut::<GridPos>(shooter).unwrap().0 = TilePos::new(5, 2, 0);
    // A wall stands between them on this row.
    let behind = spawn_player_mob(app.world_mut(), TilePos::new(10, 2, 0));
    act(&mut app, shooter, Action::UseOnTile(TilePos::new(8, 2, 0)));
    ticks(&mut app, 60);
    assert_eq!(burn(&app, behind), 0.0);
}

#[test]
fn someone_lying_down_is_passed_over_unless_aimed_at() {
    let (mut app, shooter) = setup();
    give(&mut app, shooter, "laser-gun");
    let lying = spawn_player_mob(app.world_mut(), TilePos::new(5, 3, 0));
    let standing = spawn_player_mob(app.world_mut(), TilePos::new(8, 3, 0));
    app.world_mut().entity_mut(lying).insert(Weakened(1.0e9));
    act(&mut app, shooter, Action::UseOn(standing));
    ticks(&mut app, 30);
    assert_eq!(burn(&app, lying), 0.0);
    assert_eq!(burn(&app, standing), BEAM_BURN);
    ticks(&mut app, 10);
    act(&mut app, shooter, Action::UseOn(lying));
    ticks(&mut app, 30);
    assert_eq!(burn(&app, lying), BEAM_BURN);
}

#[test]
fn a_taser_knocks_down_without_hurting_and_runs_out_after_five() {
    let (mut app, shooter) = setup();
    let gun = give(&mut app, shooter, "taser");
    let target = spawn_player_mob(app.world_mut(), TilePos::new(6, 3, 0));
    act(&mut app, shooter, Action::UseOn(target));
    ticks(&mut app, 20);
    assert!(app.world().get::<Weakened>(target).is_some());
    assert_eq!(burn(&app, target), 0.0);
    app.world_mut().get_mut::<Gun>(gun).unwrap().charge = SHOT_COST - 1;
    act(&mut app, shooter, Action::UseOn(target));
    assert_eq!(shots(&mut app), 0);
    assert!(messages(&app, shooter).contains(&"gun-click".to_owned()));
    assert_eq!(TASER_CELL / SHOT_COST, 5);
}

#[test]
fn an_energy_gun_switches_between_stun_and_kill() {
    let (mut app, shooter) = setup();
    let gun = give(&mut app, shooter, "energy-gun");
    assert_eq!(app.world().get::<Gun>(gun).unwrap().fires, Fires::Electrode);
    assert_eq!(app.world().get::<Appearance>(gun).unwrap().state, "energystun100");
    act(&mut app, shooter, Action::UseOn(gun));
    ticks(&mut app, 1);
    assert_eq!(app.world().get::<Gun>(gun).unwrap().fires, Fires::Beam);
    assert_eq!(app.world().get::<Appearance>(gun).unwrap().state, "energykill100");
    assert!(messages(&app, shooter).contains(&"gun-set-kill".to_owned()));
}

#[test]
fn a_gun_in_reach_doesnt_fire() {
    let (mut app, shooter) = setup();
    let gun = give(&mut app, shooter, "laser-gun");
    let target = spawn_player_mob(app.world_mut(), TilePos::new(3, 3, 0));
    act(&mut app, shooter, Action::UseOn(target));
    assert_eq!(shots(&mut app), 0);
    assert_eq!(app.world().get::<Gun>(gun).unwrap().charge, GUN_CELL);
}

#[test]
fn a_baton_stuns_when_on_and_only_prods_when_off() {
    let (mut app, officer) = setup();
    let baton = give(&mut app, officer, "stun-baton");
    let target = spawn_player_mob(app.world_mut(), TilePos::new(3, 3, 0));
    act(&mut app, officer, Action::UseOn(target));
    assert!(app.world().get::<Weakened>(target).is_none());
    assert!(messages(&app, officer).contains(&"baton-prod".to_owned()));

    act(&mut app, officer, Action::UseOn(baton));
    assert!(app.world().get::<Baton>(baton).unwrap().on);
    assert_eq!(
        app.world().get::<Appearance>(baton).unwrap().state,
        "stunbaton_active"
    );
    act(&mut app, officer, Action::UseOn(target));
    assert!(app.world().get::<Weakened>(target).is_some());
    assert_eq!(
        app.world().get::<Baton>(baton).unwrap().charge,
        BATON_CELL - BATON_HIT
    );
    assert_eq!(burn(&app, target), 0.0, "no blow on help intent");

    app.world_mut().get_mut::<CombatIntent>(officer).unwrap().0 = Intent::Harm;
    act(&mut app, officer, Action::UseOn(target));
    ticks(&mut app, 1);
    assert!(burn(&app, target) > 0.0, "a blow as well");
}

#[test]
fn a_recharger_refills_a_gun_and_gives_it_back() {
    let (mut app, shooter) = setup();
    let gun = give(&mut app, shooter, "laser-gun");
    app.world_mut().get_mut::<Gun>(gun).unwrap().charge = 0;
    let mut q = app.world_mut().query_filtered::<Entity, With<Recharger>>();
    let recharger = q.single(app.world()).unwrap();
    app.world_mut().get_mut::<GridPos>(shooter).unwrap().0 = TilePos::new(2, 1, 0);
    act(&mut app, shooter, Action::UseOn(recharger));
    assert_eq!(
        app.world().get::<Recharger>(recharger).unwrap().charging,
        Some(gun)
    );
    assert_eq!(app.world().get::<Hands>(shooter).unwrap().active_item(), None);
    ticks(&mut app, 40 * 12);
    assert_eq!(app.world().get::<Gun>(gun).unwrap().charge, GUN_CELL);
    assert_eq!(
        app.world().get::<Appearance>(recharger).unwrap().state,
        "recharger2"
    );
    assert_eq!(app.world().get::<Appearance>(gun).unwrap().state, "laser100");
    act(&mut app, shooter, Action::UseOn(recharger));
    assert_eq!(
        app.world().get::<Hands>(shooter).unwrap().active_item(),
        Some(gun)
    );
}
