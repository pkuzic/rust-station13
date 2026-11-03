//! What clothes do. Spec: docs/reference/combat.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::atmos::StationAtmos;
use rs13_classic::combat::{ArmorKind, armour_check, armour_on};
use rs13_classic::equipment::{Equipment, WornBy};
use rs13_classic::flashes::Weakened;
use rs13_classic::health::{Body, Zone};
use rs13_classic::movement::MoveIntent;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_simulation::atmos::gas::T20C;
use rs13_simulation::atmos::{Gas, GasMix};
use rs13_world::{Dir, GridPos, ObjectSpec, Slot, TileMap, TilePos};

const MAP: &str = r#"(
    legend: { 'W': (tile: (id: "wall", solid: true)), '.': (tile: (id: "floor")) },
    rows: ["WWWWWWW", "W.....W", "WWWWWWW"],
)"#;
const AT: TilePos = TilePos::new(1, 1, 0);

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), AT);
    app.world_mut().get_mut::<Equipment>(mob).unwrap().0.clear();
    app.world_mut().run_schedule(FixedUpdate);
    (app, mob)
}

fn wear(app: &mut App, mob: Entity, slot: Slot, proto: &str) {
    let item = spawn_object(app.world_mut(), &ObjectSpec::new(proto), AT).unwrap();
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(WornBy(mob));
    app.world_mut()
        .get_mut::<Equipment>(mob)
        .unwrap()
        .0
        .insert(slot, item);
}

#[test]
fn armour_counts_where_it_covers_and_against_what_its_made_for() {
    let (mut app, mob) = setup();
    assert_eq!(armour_on(app.world(), mob, Zone::Chest, ArmorKind::Bullet), 0.0);
    wear(&mut app, mob, Slot::Suit, "ablative-vest");
    let laser = armour_on(app.world(), mob, Zone::Chest, ArmorKind::Laser);
    let bullet = armour_on(app.world(), mob, Zone::Chest, ArmorKind::Bullet);
    assert!(
        laser > bullet,
        "an ablative vest is for lasers: {laser} and {bullet}"
    );
    assert_eq!(
        armour_on(app.world(), mob, Zone::Head, ArmorKind::Laser),
        0.0,
        "not on the head"
    );
}

#[test]
fn two_rolls_decide_whether_it_stops_softens_or_doesnt_help() {
    let (mut app, mob) = setup();
    wear(&mut app, mob, Slot::Suit, "ablative-vest");
    let mut seen = [0; 3];
    for _ in 0..300 {
        seen[armour_check(app.world_mut(), mob, Zone::Chest, ArmorKind::Laser) as usize] += 1;
    }
    assert!(seen.iter().all(|n| *n > 0), "{seen:?}");
}

/// How burnt someone gets standing in a fire kept going.
fn burn_after_a_while(app: &mut App, mob: Entity) -> f32 {
    app.world_mut().entity_mut(mob).insert(Body::default());
    for _ in 0..200 {
        let mut gas = GasMix::default();
        gas.temperature = T20C;
        gas.set(Gas::Plasma, 5.0);
        gas.set(Gas::Oxygen, 5.0);
        let mut atmos = app.world_mut().resource_mut::<StationAtmos>();
        atmos.0.add_at(AT, &gas);
        atmos.0.expose(AT, 2000.0);
        app.world_mut().run_schedule(FixedUpdate);
    }
    Zone::ALL
        .into_iter()
        .map(|z| app.world().get::<Body>(mob).unwrap().organ(z).burn)
        .sum()
}

#[test]
fn a_firesuit_keeps_most_of_a_fire_off() {
    let (mut app, mob) = setup();
    let bare = burn_after_a_while(&mut app, mob);
    assert!(bare > 10.0, "a fire burns: {bare}");
    wear(&mut app, mob, Slot::Suit, "firesuit");
    let suited = burn_after_a_while(&mut app, mob);
    assert!(suited < bare / 5.0, "{suited} against {bare}");
}

#[test]
fn shoes_that_dont_slip_keep_you_up() {
    let (mut app, mob) = setup();
    let step_onto_soap = |app: &mut App, mob: Entity| {
        app.world_mut().get_mut::<GridPos>(mob).unwrap().0 = AT;
        app.world_mut().entity_mut(mob).remove::<Weakened>();
        app.world_mut().get_mut::<MoveIntent>(mob).unwrap().dir = Some(Dir::East);
        for _ in 0..4 {
            app.world_mut().run_schedule(FixedUpdate);
            if app.world().get::<GridPos>(mob).unwrap().0 != AT {
                break;
            }
        }
        app.world_mut().get_mut::<MoveIntent>(mob).unwrap().dir = None;
        app.world().get::<Weakened>(mob).is_some()
    };
    spawn_object(app.world_mut(), &ObjectSpec::new("soap"), TilePos::new(2, 1, 0)).unwrap();
    for _ in 0..40 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    assert!(step_onto_soap(&mut app, mob), "barefoot");
    for _ in 0..40 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    wear(&mut app, mob, Slot::Shoes, "galoshes");
    assert!(!step_onto_soap(&mut app, mob), "in galoshes");
}
