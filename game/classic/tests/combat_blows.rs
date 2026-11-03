//! What blows do, with enough of them for chance to even out. Spec: docs/reference/combat.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::combat::{AimZone, CombatIntent, Paralyzed};
use rs13_classic::equipment::{Equipment, WornBy};
use rs13_classic::flashes::Weakened;
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::health::{Body, Vitals, Zone};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_world::{DisplayName, GridPos, Intent, MobStat, ObjectSpec, Slot, TileMap, TilePos};

const MAP: &str = r#"(
    legend: { 'W': (tile: (id: "wall", solid: true)), '.': (tile: (id: "floor")) },
    rows: ["WWWWWW", "W....W", "WWWWWW"],
)"#;
const A: TilePos = TilePos::new(1, 1, 0);
const B: TilePos = TilePos::new(2, 1, 0);

fn setup() -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let attacker = spawn_player_mob(app.world_mut(), A);
    let victim = spawn_player_mob(app.world_mut(), B);
    app.world_mut().get_mut::<DisplayName>(victim).unwrap().0 = "Victim".to_owned();
    app.world_mut().run_schedule(FixedUpdate);
    (app, attacker, victim)
}

fn intent(app: &mut App, mob: Entity, intent: Intent) {
    app.world_mut().get_mut::<CombatIntent>(mob).unwrap().0 = intent;
}

fn hit(app: &mut App, attacker: Entity, victim: Entity) {
    app.world_mut()
        .get_mut::<PendingActions>(attacker)
        .unwrap()
        .0
        .push(Action::UseOn(victim));
    app.world_mut().run_schedule(FixedUpdate);
}

fn wear(app: &mut App, mob: Entity, slot: Slot, proto: &str) {
    let item = spawn_object(app.world_mut(), &ObjectSpec::new(proto), B).unwrap();
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

fn hold(app: &mut App, mob: Entity, proto: &str) -> Entity {
    let item = spawn_object(app.world_mut(), &ObjectSpec::new(proto), A).unwrap();
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    let mut hands = app.world_mut().get_mut::<Hands>(mob).unwrap();
    let active = hands.active;
    *hands.slot_mut(active) = Some(item);
    item
}

fn heal(app: &mut App, mob: Entity) {
    app.world_mut().entity_mut(mob).insert(Body::default());
    app.world_mut().entity_mut(mob).remove::<Weakened>();
    app.world_mut().entity_mut(mob).remove::<Paralyzed>();
    app.world_mut().get_mut::<Outbox>(mob).unwrap().0.clear();
}

/// Punches at this part, and what share were stopped.
fn stopped_share(app: &mut App, attacker: Entity, victim: Entity, zone: Zone) -> f32 {
    app.world_mut().get_mut::<AimZone>(attacker).unwrap().0 = zone;
    intent(app, attacker, Intent::Harm);
    let mut stopped = 0;
    for _ in 0..400 {
        heal(app, victim);
        hit(app, attacker, victim);
        let outbox = &app.world().get::<Outbox>(victim).unwrap().0;
        if outbox.iter().any(|m| m.id.starts_with("protected-")) {
            stopped += 1;
        }
    }
    stopped as f32 / 400.0
}

#[test]
fn aiming_decides_where_blows_land() {
    let (mut app, attacker, victim) = setup();
    app.world_mut().get_mut::<AimZone>(attacker).unwrap().0 = Zone::LLeg;
    intent(&mut app, attacker, Intent::Harm);
    let mut on_leg = 0;
    for _ in 0..200 {
        heal(&mut app, victim);
        hit(&mut app, attacker, victim);
        if app.world().get::<Body>(victim).unwrap().organ(Zone::LLeg).brute > 0.0 {
            on_leg += 1;
        }
    }
    assert!((90..170).contains(&on_leg), "{on_leg} of 200 on the leg aimed at");

    // Set the way a client does.
    app.world_mut()
        .get_mut::<PendingActions>(attacker)
        .unwrap()
        .0
        .push(Action::SetZone(Zone::Head));
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(app.world().get::<AimZone>(attacker).unwrap().0, Zone::Head);
}

#[test]
fn clothes_stop_punches_and_helmets_stop_them_at_the_head() {
    let (mut app, attacker, victim) = setup();
    // People arrive dressed.
    app.world_mut().get_mut::<Equipment>(victim).unwrap().0.clear();
    let bare = stopped_share(&mut app, attacker, victim, Zone::Chest);
    assert_eq!(bare, 0.0, "nothing on");
    wear(&mut app, victim, Slot::Uniform, "grey-jumpsuit");
    let covered = stopped_share(&mut app, attacker, victim, Zone::Chest);
    assert!(covered > 0.5, "{covered}");
    let head = stopped_share(&mut app, attacker, victim, Zone::Head);
    // Only what strays to the chest: a third of blows, 85% of those.
    assert!(head < 0.4, "a uniform doesn't cover the head: {head}");
    wear(&mut app, victim, Slot::Head, "helmet");
    let head = stopped_share(&mut app, attacker, victim, Zone::Head);
    assert!(head > 0.6, "{head}");
}

#[test]
fn armour_stops_weapons_at_the_chest() {
    let (mut app, attacker, victim) = setup();
    hold(&mut app, attacker, "toolbox-mechanical");
    let share = |app: &mut App| {
        let mut stopped = 0;
        for _ in 0..300 {
            heal(app, victim);
            hit(app, attacker, victim);
            let outbox = &app.world().get::<Outbox>(victim).unwrap().0;
            if outbox.iter().any(|m| m.id == "protected-chest") {
                stopped += 1;
            }
        }
        stopped as f32 / 300.0
    };
    assert_eq!(share(&mut app), 0.0);
    wear(&mut app, victim, Slot::Suit, "armor-vest");
    assert!(share(&mut app) > 0.5);
}

#[test]
fn a_heavy_blow_to_the_head_can_knock_someone_out_and_they_come_round() {
    let (mut app, attacker, victim) = setup();
    hold(&mut app, attacker, "toolbox-mechanical");
    app.world_mut().get_mut::<AimZone>(attacker).unwrap().0 = Zone::Head;
    let mut out = false;
    for _ in 0..200 {
        heal(&mut app, victim);
        hit(&mut app, attacker, victim);
        if app.world().get::<Paralyzed>(victim).is_some() {
            out = true;
            break;
        }
    }
    assert!(out, "never knocked out");
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(
        app.world().get::<Vitals>(victim).unwrap().stat,
        MobStat::Unconscious
    );
    for _ in 0..2 * 300 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    assert!(app.world().get::<Paralyzed>(victim).is_none());
    assert_eq!(
        app.world().get::<Vitals>(victim).unwrap().stat,
        MobStat::Conscious
    );
}

#[test]
fn disarm_pushes_down_or_takes_what_they_hold() {
    let (mut app, attacker, victim) = setup();
    intent(&mut app, attacker, Intent::Disarm);
    let mut dropped = 0;
    let mut pushed = 0;
    for _ in 0..200 {
        heal(&mut app, victim);
        let held = hold(&mut app, victim, "crowbar");
        hit(&mut app, attacker, victim);
        if app.world().get::<Weakened>(victim).is_some() {
            pushed += 1;
        }
        if app.world().get::<GridPos>(held).map(|p| p.0) == Some(B) {
            assert!(app.world().get::<Hands>(victim).unwrap().active_item().is_none());
            dropped += 1;
        } else {
            let mut hands = app.world_mut().get_mut::<Hands>(victim).unwrap();
            let active = hands.active;
            *hands.slot_mut(active) = None;
        }
    }
    assert!((25..85).contains(&pushed), "{pushed}");
    assert!((40..110).contains(&dropped), "{dropped}");
}

#[test]
fn help_shakes_others_and_checks_yourself() {
    let (mut app, attacker, victim) = setup();
    app.world_mut().entity_mut(victim).insert(Weakened(1000.0));
    hit(&mut app, attacker, victim);
    assert!(app.world().get::<Weakened>(victim).unwrap().0 < 1000.0);

    app.world_mut()
        .get_mut::<Body>(attacker)
        .unwrap()
        .take_damage(Zone::Head, 40.0, 25.0);
    app.world_mut().get_mut::<Outbox>(attacker).unwrap().0.clear();
    hit(&mut app, attacker, attacker);
    let told = &app.world().get::<Outbox>(attacker).unwrap().0;
    assert_eq!(
        told.iter().filter(|m| m.id == "self-part-two").count(),
        1,
        "{told:?}"
    );
    assert!(
        told.iter().filter(|m| m.id == "self-part-ok").count() >= 9,
        "{told:?}"
    );
}
