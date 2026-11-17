//! The captain's display case and antique laser. Spec: docs/reference/weapons.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::display_case::{CASE_HEALTH, DisplayCase, KICK};
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::weapons::{CAPTAIN_LASER, Fires, Gun, SELF_CHARGE};
use rs13_world::{Appearance, Dense, GridPos, ObjectSpec, TileMap, TilePos};

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
    let case = spawn_object(app.world_mut(), &ObjectSpec::new("display-case"), NEXT).unwrap();
    (app, mob, case)
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

fn case(app: &App, e: Entity) -> DisplayCase {
    *app.world().get::<DisplayCase>(e).unwrap()
}

fn lasers(app: &mut App) -> Vec<Entity> {
    let mut q = app.world_mut().query::<(Entity, &Appearance, Option<&GridPos>)>();
    q.iter(app.world())
        .filter(|(_, a, p)| a.sprite == CAPTAIN_LASER && p.is_some_and(|p| p.0 == NEXT))
        .map(|(e, ..)| e)
        .collect()
}

#[test]
fn kicks_wear_the_glass_down_and_a_hand_then_takes_the_antique_laser() {
    let (mut app, mob, display) = setup();
    assert_eq!(app.world().get::<Appearance>(display).unwrap().state, "glassbox1");
    assert!(app.world().get::<Dense>(display).is_some());
    // An empty hand on the whole case only kicks it.
    act(&mut app, mob, Action::UseOn(display));
    assert_eq!(case(&app, display).health, CASE_HEALTH - KICK);
    assert!(lasers(&mut app).is_empty());
    while !case(&app, display).destroyed {
        act(&mut app, mob, Action::UseOn(display));
    }
    assert!(app.world().get::<Dense>(display).is_none());
    assert_eq!(
        app.world().get::<Appearance>(display).unwrap().state,
        "glassboxb1"
    );

    let told = act(&mut app, mob, Action::UseOn(display));
    assert_eq!(told, ["display-case-field-off"]);
    assert_eq!(lasers(&mut app).len(), 1);
    assert!(!case(&app, display).occupied);
    assert_eq!(
        app.world().get::<Appearance>(display).unwrap().state,
        "glassboxb0"
    );
}

#[test]
fn an_item_hits_the_glass_for_its_force() {
    let (mut app, mob, display) = setup();
    let crowbar = spawn_object(app.world_mut(), &ObjectSpec::new("crowbar"), HERE).unwrap();
    let hand = app.world().get::<Hands>(mob).unwrap().active;
    app.world_mut()
        .entity_mut(crowbar)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    *app.world_mut().get_mut::<Hands>(mob).unwrap().slot_mut(hand) = Some(crowbar);
    let force = app
        .world()
        .get::<rs13_classic::hands::Item>(crowbar)
        .unwrap()
        .force;
    act(&mut app, mob, Action::UseOn(display));
    assert_eq!(case(&app, display).health, CASE_HEALTH - force);
}

#[test]
fn the_antique_laser_fires_a_heavy_beam_and_charges_itself() {
    let (mut app, _mob, _case) = setup();
    let gun = spawn_object(app.world_mut(), &ObjectSpec::new(CAPTAIN_LASER), HERE).unwrap();
    let state = *app.world().get::<Gun>(gun).unwrap();
    assert_eq!(state.fires, Fires::HeavyBeam);
    assert!(!state.switchable);
    app.world_mut().get_mut::<Gun>(gun).unwrap().charge = 0;
    // Every 4 seconds, 100 more.
    tick(&mut app, 2 * 41);
    let charge = app.world().get::<Gun>(gun).unwrap().charge;
    assert!((SELF_CHARGE..=2 * SELF_CHARGE).contains(&charge), "{charge}");
}

#[test]
fn the_antique_laser_is_a_theft_objective() {
    assert!(rs13_classic::traitor::THEFTS.contains(&CAPTAIN_LASER));
}

#[test]
fn an_energy_sword_switches_between_a_hilt_and_a_blade_and_blocks_as_a_shield() {
    use rs13_classic::swords::{ENERGY_SWORD, EnergySword, OFF_FORCE, ON_FORCE, is_shield};
    let (mut app, mob, _case) = setup();
    let sword = spawn_object(app.world_mut(), &ObjectSpec::new(ENERGY_SWORD), HERE).unwrap();
    let hand = app.world().get::<Hands>(mob).unwrap().active;
    app.world_mut()
        .entity_mut(sword)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    *app.world_mut().get_mut::<Hands>(mob).unwrap().slot_mut(hand) = Some(sword);
    assert!(!is_shield(app.world(), sword));
    assert_eq!(act(&mut app, mob, Action::UseOn(sword)), ["sword-on"]);
    let state = *app.world().get::<EnergySword>(sword).unwrap();
    assert!(state.active);
    assert_eq!(
        app.world().get::<rs13_classic::hands::Item>(sword).unwrap().force,
        ON_FORCE
    );
    assert_eq!(
        app.world().get::<Appearance>(sword).unwrap().state,
        format!("sword{}", state.colour)
    );
    assert!(is_shield(app.world(), sword));
    assert_eq!(act(&mut app, mob, Action::UseOn(sword)), ["sword-off"]);
    assert_eq!(
        app.world().get::<rs13_classic::hands::Item>(sword).unwrap().force,
        OFF_FORCE
    );
    assert_eq!(app.world().get::<Appearance>(sword).unwrap().state, "sword0");
}

#[test]
fn the_crossbow_fires_bolts_charges_itself_and_is_sold_with_the_sword() {
    use rs13_classic::weapons::ENERGY_CROSSBOW;
    let (mut app, _mob, _case) = setup();
    let bow = spawn_object(app.world_mut(), &ObjectSpec::new(ENERGY_CROSSBOW), HERE).unwrap();
    assert_eq!(app.world().get::<Gun>(bow).unwrap().fires, Fires::Bolt);
    app.world_mut().get_mut::<Gun>(bow).unwrap().charge = 0;
    tick(&mut app, 2 * 41);
    assert!(app.world().get::<Gun>(bow).unwrap().charge >= SELF_CHARGE);
    let goods: Vec<&str> = rs13_classic::traitor::GOODS.iter().map(|(id, _)| *id).collect();
    assert!(goods.contains(&ENERGY_CROSSBOW) && goods.contains(&"energy-sword"));
}
