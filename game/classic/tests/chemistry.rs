//! Chemistry. Spec: docs/reference/chemistry.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::chemistry::{INJECT_DS, METABOLISM, Reagents, Syringe};
use rs13_classic::flashes::Weakened;
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::health::{Body, Zone};
use rs13_classic::machines::MACHINE_TICKS;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_world::{Appearance, GridPos, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: { 'W': (tile: (id: "wall", solid: true)), '.': (tile: (id: "floor")) },
    rows: ["WWWWWW", "W....W", "WWWWWW"],
)"#;
const AT: TilePos = TilePos::new(1, 1, 0);
const NEXT: TilePos = TilePos::new(2, 1, 0);

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), AT);
    app.world_mut().run_schedule(FixedUpdate);
    (app, mob)
}

fn ticks(app: &mut App, n: u64) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn hold(app: &mut App, mob: Entity, proto: &str) -> Entity {
    let item = spawn_object(app.world_mut(), &ObjectSpec::new(proto), AT).unwrap();
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    let mut hands = app.world_mut().get_mut::<Hands>(mob).unwrap();
    let active = hands.active;
    *hands.slot_mut(active) = Some(item);
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

fn amount(app: &App, who: Entity, reagent: &str) -> f32 {
    app.world().get::<Reagents>(who).unwrap().get(reagent)
}

fn last_message(app: &App, who: Entity) -> String {
    app.world()
        .get::<Outbox>(who)
        .unwrap()
        .0
        .last()
        .unwrap()
        .id
        .clone()
}

#[test]
fn a_syringe_draws_from_a_bottle_and_injects_yourself() {
    let (mut app, mob) = setup();
    let bottle = spawn_object(app.world_mut(), &ObjectSpec::new("bottle-antitoxin"), NEXT).unwrap();
    let syringe = hold(&mut app, mob, "syringe");
    assert_eq!(app.world().get::<Appearance>(syringe).unwrap().state, "d0");
    click(&mut app, mob, bottle);
    assert_eq!(amount(&app, syringe, "anti_toxin"), 5.0);
    assert_eq!(amount(&app, bottle, "anti_toxin"), 25.0);
    assert_eq!(app.world().get::<Appearance>(syringe).unwrap().state, "d5");

    click(&mut app, mob, syringe);
    assert!(app.world().get::<Syringe>(syringe).unwrap().injecting);
    assert_eq!(app.world().get::<Appearance>(syringe).unwrap().state, "i5");
    click(&mut app, mob, mob);
    assert_eq!(amount(&app, syringe, "anti_toxin"), 0.0);
    assert_eq!(amount(&app, mob, "anti_toxin"), 5.0);
    assert!(
        !app.world().get::<Syringe>(syringe).unwrap().injecting,
        "back to drawing"
    );
}

#[test]
fn injecting_someone_else_takes_a_moment_and_they_must_stay() {
    let (mut app, mob) = setup();
    let other = spawn_player_mob(app.world_mut(), NEXT);
    hold(&mut app, mob, "syringe-inaprovaline");
    click(&mut app, mob, other);
    assert_eq!(amount(&app, other, "inaprovaline"), 0.0);
    ticks(&mut app, (INJECT_DS * 2.0) as u64 + 2);
    assert!(amount(&app, other, "inaprovaline") > 4.0);

    click(&mut app, mob, other);
    app.world_mut().get_mut::<GridPos>(other).unwrap().0 = TilePos::new(4, 1, 0);
    let had = amount(&app, other, "inaprovaline");
    ticks(&mut app, (INJECT_DS * 2.0) as u64 + 2);
    assert!(amount(&app, other, "inaprovaline") <= had, "they moved away");
}

#[test]
fn a_syringe_cant_draw_blood() {
    let (mut app, mob) = setup();
    let other = spawn_player_mob(app.world_mut(), NEXT);
    hold(&mut app, mob, "syringe");
    click(&mut app, mob, other);
    assert_eq!(last_message(&app, mob), "syringe-no-blood");
}

#[test]
fn a_pill_is_swallowed_whole() {
    let (mut app, mob) = setup();
    let pill = hold(&mut app, mob, "pill-antitox");
    click(&mut app, mob, mob);
    assert_eq!(amount(&app, mob, "anti_toxin"), 25.0);
    assert!(app.world().get_entity(pill).is_err());
    assert_eq!(app.world().get::<Hands>(mob).unwrap().active_item(), None);
}

#[test]
fn a_hypospray_works_at_once_on_someone_else() {
    let (mut app, mob) = setup();
    let other = spawn_player_mob(app.world_mut(), NEXT);
    let hypo = hold(&mut app, mob, "hypospray");
    click(&mut app, mob, other);
    assert_eq!(amount(&app, other, "tricordrazine"), 5.0);
    assert_eq!(amount(&app, hypo, "tricordrazine"), 25.0);
}

#[test]
fn glassware_pours_and_examining_tells_how_much() {
    let (mut app, mob) = setup();
    let beaker = spawn_object(app.world_mut(), &ObjectSpec::new("beaker"), NEXT).unwrap();
    let bottle = hold(&mut app, mob, "bottle-inaprovaline");
    click(&mut app, mob, beaker);
    assert_eq!(amount(&app, beaker, "inaprovaline"), 10.0);
    assert_eq!(amount(&app, bottle, "inaprovaline"), 20.0);
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::Examine(beaker));
    ticks(&mut app, 1);
    assert_eq!(last_message(&app, mob), "reagents-contains");
}

fn dose(app: &mut App, mob: Entity, reagent: &str, units: f32) {
    app.world_mut()
        .get_mut::<Reagents>(mob)
        .unwrap()
        .add(reagent, units);
}

#[test]
fn medicines_heal_and_are_used_up() {
    let (mut app, mob) = setup();
    {
        let mut body = app.world_mut().get_mut::<Body>(mob).unwrap();
        body.take_damage(Zone::Chest, 10.0, 10.0);
        body.tox = 10.0;
    }
    dose(&mut app, mob, "kelotane", 1.0);
    dose(&mut app, mob, "bicaridine", 1.0);
    dose(&mut app, mob, "anti_toxin", 1.0);
    dose(&mut app, mob, "toxin", 5.0);
    ticks(&mut app, MACHINE_TICKS);
    let body = app.world().get::<Body>(mob).unwrap();
    assert!(body.organ(Zone::Chest).burn <= 8.0);
    assert!(body.organ(Zone::Chest).brute <= 9.0);
    assert!(amount(&app, mob, "kelotane") <= 1.0 - METABOLISM + 0.001);
    assert!(amount(&app, mob, "toxin") <= 5.0 - 2.0, "anti-toxin clears toxin");
    ticks(&mut app, 10 * MACHINE_TICKS);
    assert_eq!(amount(&app, mob, "kelotane"), 0.0, "used up");
}

#[test]
fn toxin_hurts_and_sleep_toxin_knocks_down_and_synaptizine_wakes() {
    let (mut app, mob) = setup();
    dose(&mut app, mob, "toxin", 1.0);
    ticks(&mut app, MACHINE_TICKS);
    assert!(app.world().get::<Body>(mob).unwrap().tox >= 2.0);

    dose(&mut app, mob, "stoxin", 2.0);
    ticks(&mut app, MACHINE_TICKS);
    assert!(app.world().get::<Weakened>(mob).is_some());
    dose(&mut app, mob, "synaptizine", 1.0);
    app.world_mut()
        .get_mut::<Reagents>(mob)
        .unwrap()
        .amounts
        .remove("stoxin");
    ticks(&mut app, MACHINE_TICKS);
    assert!(app.world().get::<Weakened>(mob).is_none());
}

#[test]
fn the_dead_dont_metabolise() {
    let (mut app, mob) = setup();
    app.world_mut().get_mut::<Body>(mob).unwrap().oxy = 500.0;
    ticks(&mut app, 2);
    dose(&mut app, mob, "kelotane", 1.0);
    ticks(&mut app, 3 * MACHINE_TICKS);
    assert_eq!(amount(&app, mob, "kelotane"), 1.0);
}

#[test]
fn reagents_mix_as_they_arrive() {
    let (mut app, mob) = setup();
    let beaker = hold(&mut app, mob, "beaker");
    let other = spawn_object(app.world_mut(), &ObjectSpec::new("beaker"), NEXT).unwrap();
    app.world_mut()
        .get_mut::<Reagents>(beaker)
        .unwrap()
        .add("silicon", 10.0);
    app.world_mut()
        .get_mut::<Reagents>(other)
        .unwrap()
        .add("carbon", 10.0);
    click(&mut app, mob, other);
    let mixed = app.world().get::<Reagents>(other).unwrap();
    assert_eq!(mixed.get("silicon"), 0.0);
    assert_eq!(mixed.get("carbon"), 0.0);
    assert_eq!(mixed.get("kelotane"), 20.0);
}

#[test]
fn a_dispenser_fills_a_loaded_beaker_on_its_energy() {
    use rs13_classic::chem_dispenser::{ChemDispenser, ChemOp, ChemPanelOut, PendingChemOps};
    let (mut app, mob) = setup();
    let machine = spawn_object(app.world_mut(), &ObjectSpec::new("chem-dispenser"), NEXT).unwrap();
    let beaker = hold(&mut app, mob, "beaker");
    click(&mut app, mob, machine);
    assert_eq!(
        app.world().get::<ChemDispenser>(machine).unwrap().beaker,
        Some(beaker)
    );
    ticks(&mut app, 1);
    let view = app
        .world()
        .get::<ChemPanelOut>(mob)
        .unwrap()
        .0
        .clone()
        .expect("open");
    assert_eq!(view.beaker, Some((0, 50)));
    let carbon = view.reagents.iter().position(|r| r == "carbon").unwrap();
    let silicon = view.reagents.iter().position(|r| r == "silicon").unwrap();
    {
        let mut ops = app.world_mut().get_mut::<PendingChemOps>(mob).unwrap();
        ops.0.push((machine, ChemOp::Amount(10)));
        ops.0.push((machine, ChemOp::Dispense(carbon)));
        ops.0.push((machine, ChemOp::Dispense(silicon)));
    }
    ticks(&mut app, 1);
    assert_eq!(amount(&app, beaker, "kelotane"), 20.0);
    assert_eq!(app.world().get::<ChemDispenser>(machine).unwrap().energy, 98.0);
    app.world_mut()
        .get_mut::<PendingChemOps>(mob)
        .unwrap()
        .0
        .push((machine, ChemOp::Eject));
    ticks(&mut app, 1);
    assert_eq!(app.world().get::<GridPos>(beaker).unwrap().0, NEXT);
    assert_eq!(app.world().get::<ChemDispenser>(machine).unwrap().beaker, None);
}
