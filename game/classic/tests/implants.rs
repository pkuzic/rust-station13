//! Implants, the implanter, case and pad, and the prisoner console. Spec: docs/reference/implants.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::chemistry::Reagents;
use rs13_classic::equipment::{Equipment, WornBy};
use rs13_classic::filing::PendingPicks;
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::health::Vitals;
use rs13_classic::implants::{
    self, ChoosingPhrase, Implant, ImplantCase, Implanter, Implanting, InStore, Kind, PrisonerConsole,
};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::security::Cuffed;
use rs13_classic::speech::PendingSpeech;
use rs13_classic::surgery::Procedure;
use rs13_world::{GridPos, HumanLook, MobStat, ObjectSpec, Slot, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
    },
    rows: [
        "WWWWWWWWWWWW",
        "W..........W",
        "W..........W",
        "W..........W",
        "W..........W",
        "WWWWWWWWWWWW",
    ],
)"#;

const TICKS_PER_DS: u32 = 2;

fn at(x: i32, y: i32) -> TilePos {
    TilePos::new(x, y, 0)
}

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    tick(&mut app, 1);
    app
}

fn tick(app: &mut App, n: u32) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn wait_ds(app: &mut App, ds: u32) {
    tick(app, ds * TICKS_PER_DS);
}

fn person(app: &mut App, pos: TilePos) -> Entity {
    let mob = spawn_player_mob(app.world_mut(), pos);
    app.world_mut().entity_mut(mob).insert(HumanLook::default());
    mob
}

fn act(app: &mut App, mob: Entity, action: Action) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
    tick(app, 1);
}

fn say(app: &mut App, mob: Entity, text: &str) {
    app.world_mut()
        .get_mut::<PendingSpeech>(mob)
        .unwrap()
        .0
        .push(text.to_owned());
    tick(app, 2);
}

fn pick(app: &mut App, mob: Entity, target: Entity, line: usize) {
    app.world_mut()
        .get_mut::<PendingPicks>(mob)
        .unwrap()
        .0
        .push((target, line));
    tick(app, 1);
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

fn gone(app: &App, e: Entity) -> bool {
    app.world().get_entity(e).is_err()
}

fn implant_of(app: &App, implanter: Entity) -> Entity {
    app.world().get::<Implanter>(implanter).unwrap().0.unwrap()
}

/// Implant `target` with what's in the implanter, as `doctor`.
fn implant(app: &mut App, doctor: Entity, implanter: Entity, target: Entity) -> Entity {
    let imp = implant_of(app, implanter);
    give(app, doctor, implanter);
    act(app, doctor, Action::UseOn(target));
    assert!(app.world().get::<Implanting>(doctor).is_some());
    wait_ds(app, 62);
    assert_eq!(app.world().get::<Implant>(imp).unwrap().host, Some(target));
    imp
}

fn told(app: &App, mob: Entity, id: &str) -> bool {
    app.world()
        .get::<Outbox>(mob)
        .unwrap()
        .0
        .iter()
        .any(|m| m.id == id)
}

#[test]
fn an_implanter_puts_its_implant_into_a_free_body_part() {
    let mut app = setup();
    let doctor = person(&mut app, at(3, 2));
    let patient = person(&mut app, at(4, 2));
    let implanter = spawn(&mut app, "implanter-explosive", at(3, 2));
    let imp = implant(&mut app, doctor, implanter, patient);
    assert!(app.world().get::<Implant>(imp).unwrap().zone.is_some());
    assert!(app.world().get::<Implanter>(implanter).unwrap().0.is_none());
    assert!(app.world().get::<GridPos>(imp).is_none());
}

#[test]
fn stepping_away_stops_the_implanting() {
    let mut app = setup();
    let doctor = person(&mut app, at(3, 2));
    let patient = person(&mut app, at(4, 2));
    let implanter = spawn(&mut app, "implanter-freedom", at(3, 2));
    give(&mut app, doctor, implanter);
    act(&mut app, doctor, Action::UseOn(patient));
    app.world_mut().entity_mut(patient).insert(GridPos(at(5, 2)));
    wait_ds(&mut app, 62);
    assert!(app.world().get::<Implanting>(doctor).is_none());
    assert!(app.world().get::<Implanter>(implanter).unwrap().0.is_some());
}

#[test]
fn the_freedom_implant_slips_the_cuffs_on_its_emote() {
    let mut app = setup();
    let doctor = person(&mut app, at(3, 2));
    let prisoner = person(&mut app, at(4, 2));
    let implanter = spawn(&mut app, "implanter-freedom", at(3, 2));
    let imp = implant(&mut app, doctor, implanter, prisoner);
    // Line 0 is the title; line 4 is "chuckle".
    pick(&mut app, prisoner, imp, 4);
    assert!(matches!(
        &app.world().get::<Implant>(imp).unwrap().kind,
        Kind::Freedom { emote: Some(e), .. } if e == "chuckle"
    ));
    let cuffs = spawn(&mut app, "handcuffs", at(4, 2));
    app.world_mut()
        .entity_mut(cuffs)
        .remove::<GridPos>()
        .insert(WornBy(prisoner));
    app.world_mut()
        .get_mut::<Equipment>(prisoner)
        .unwrap()
        .0
        .insert(Slot::Cuffs, cuffs);
    app.world_mut().entity_mut(prisoner).insert(Cuffed(cuffs));
    say(&mut app, prisoner, "*chuckle");
    assert!(app.world().get::<Cuffed>(prisoner).is_none());
    assert_eq!(app.world().get::<GridPos>(cuffs).unwrap().0, at(4, 2));
    assert!(app.world().get::<Reagents>(prisoner).unwrap().get("synaptizine") > 0.0);
}

#[test]
fn the_explosive_implant_goes_off_when_its_phrase_is_said_nearby() {
    let mut app = setup();
    let doctor = person(&mut app, at(3, 2));
    let agent = person(&mut app, at(4, 2));
    let bystander = person(&mut app, at(9, 3));
    let implanter = spawn(&mut app, "implanter-explosive", at(3, 2));
    let imp = implant(&mut app, doctor, implanter, agent);
    assert!(app.world().get::<ChoosingPhrase>(agent).is_some());
    app.world_mut().get_mut::<Outbox>(doctor).unwrap().0.clear();
    say(&mut app, agent, "Red Sky");
    assert!(app.world().get::<ChoosingPhrase>(agent).is_none());
    assert!(!told(&app, doctor, "say"), "the phrase is said to no one");
    // Something else said is harmless.
    say(&mut app, bystander, "hello there");
    assert!(!gone(&app, imp));
    say(&mut app, bystander, "the red sky at night");
    tick(&mut app, 2);
    assert!(gone(&app, imp));
    assert_eq!(app.world().get::<Vitals>(agent).unwrap().stat, MobStat::Dead);
}

#[test]
fn chemical_implants_fill_from_a_syringe_and_inject_from_the_console() {
    let mut app = setup();
    let warden = person(&mut app, at(3, 2));
    let prisoner = person(&mut app, at(4, 2));
    let case = spawn(&mut app, "implant-case-chem", at(3, 2));
    let syringe = spawn(&mut app, "syringe", at(3, 2));
    app.world_mut()
        .get_mut::<Reagents>(syringe)
        .unwrap()
        .add("toxin", 15.0);
    give(&mut app, warden, syringe);
    act(&mut app, warden, Action::UseOn(case));
    act(&mut app, warden, Action::UseOn(case));
    let imp = app.world().get::<ImplantCase>(case).unwrap().0.unwrap();
    assert_eq!(app.world().get::<Reagents>(imp).unwrap().total(), 10.0);
    // An empty implanter takes it out of the case.
    let implanter = spawn(&mut app, "implanter", at(3, 2));
    give(&mut app, warden, implanter);
    act(&mut app, warden, Action::UseOn(case));
    assert_eq!(app.world().get::<Implanter>(implanter).unwrap().0, Some(imp));
    assert!(app.world().get::<ImplantCase>(case).unwrap().0.is_none());
    implant(&mut app, warden, implanter, prisoner);

    let console = spawn(&mut app, implants::CONSOLE, at(2, 2));
    app.world_mut()
        .get_mut::<PrisonerConsole>(console)
        .unwrap()
        .unlocked = true;
    tick(&mut app, 2);
    // 0 title, 1 the prisoner, 2-4 inject 1, 5, 10.
    pick(&mut app, warden, console, 3);
    assert_eq!(app.world().get::<Reagents>(prisoner).unwrap().get("toxin"), 5.0);
    assert_eq!(app.world().get::<Reagents>(imp).unwrap().total(), 5.0);
    // The rest goes in when the prisoner dies.
    app.world_mut().get_mut::<Vitals>(prisoner).unwrap().stat = MobStat::Dead;
    tick(&mut app, 2);
    assert!(gone(&app, imp));
}

#[test]
fn the_console_stays_locked_without_armoury_access() {
    let mut app = setup();
    let crew = person(&mut app, at(3, 2));
    let console = spawn(&mut app, implants::CONSOLE, at(2, 2));
    tick(&mut app, 2);
    pick(&mut app, crew, console, 0);
    assert!(!app.world().get::<PrisonerConsole>(console).unwrap().unlocked);
    assert!(told(&app, crew, "prisoner-denied"));
}

#[test]
fn the_pad_sets_a_tracking_implant_and_the_teleporter_finds_it() {
    let mut app = setup();
    let officer = person(&mut app, at(3, 2));
    let prisoner = person(&mut app, at(4, 2));
    let pad = spawn(&mut app, implants::PAD, at(3, 2));
    let case = spawn(&mut app, "implant-case-tracking", at(3, 2));
    give(&mut app, officer, case);
    act(&mut app, officer, Action::UseOn(pad));
    let imp = app.world().get::<ImplantCase>(case).unwrap().0.unwrap();
    // 0 info, 1 frequency, 2-5 its steps, 6 id, 7-10 its steps.
    pick(&mut app, officer, pad, 5);
    pick(&mut app, officer, pad, 9);
    assert_eq!(
        app.world().get::<Implant>(imp).unwrap().kind,
        Kind::Tracking {
            frequency: 1461,
            id: 2
        }
    );
    // Out of the pad (held, clicked from the other, empty hand), onto the floor, into an
    // implanter, into the prisoner.
    give(&mut app, officer, pad);
    let other = match app.world().get::<Hands>(officer).unwrap().active {
        rs13_world::Hand::Left => rs13_world::Hand::Right,
        rs13_world::Hand::Right => rs13_world::Hand::Left,
    };
    {
        let mut hands = app.world_mut().get_mut::<Hands>(officer).unwrap();
        hands.active = other;
        *hands.slot_mut(other) = None;
    }
    act(&mut app, officer, Action::UseOn(pad));
    assert_eq!(
        app.world().get::<Hands>(officer).unwrap().active_item(),
        Some(case)
    );
    *app.world_mut().get_mut::<Hands>(officer).unwrap().slot_mut(other) = None;
    app.world_mut()
        .entity_mut(case)
        .remove::<HeldBy>()
        .insert(GridPos(at(3, 2)));
    let implanter = spawn(&mut app, "implanter", at(3, 2));
    give(&mut app, officer, implanter);
    act(&mut app, officer, Action::UseOn(case));
    let imp_now = implant_of(&app, implanter);
    assert_eq!(imp_now, imp);
    implant(&mut app, officer, implanter, prisoner);
    let targets = implants::tracking_targets(app.world_mut());
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].1, imp);
    assert_eq!(implants::tracking_place(app.world(), imp), Some(at(4, 2)));
}

#[test]
fn the_compressed_matter_implant_brings_back_what_it_took() {
    let mut app = setup();
    let agent = person(&mut app, at(3, 2));
    let implanter = spawn(&mut app, implants::COMPRESS_IMPLANTER, at(3, 2));
    let crowbar = spawn(&mut app, "crowbar", at(4, 2));
    give(&mut app, agent, implanter);
    act(&mut app, agent, Action::UseOn(crowbar));
    assert!(app.world().get::<InStore>(crowbar).is_some());
    assert!(app.world().get::<GridPos>(crowbar).is_none());
    let imp = implant(&mut app, agent, implanter, agent);
    // Line 1 is "blink".
    pick(&mut app, agent, imp, 1);
    app.world_mut().entity_mut(agent).insert(GridPos(at(7, 3)));
    say(&mut app, agent, "*blink");
    assert_eq!(app.world().get::<GridPos>(crowbar).unwrap().0, at(7, 3));
    assert!(gone(&app, imp));
}

#[test]
fn a_death_alarm_calls_out_on_the_common_channel() {
    let mut app = setup();
    let doctor = person(&mut app, at(3, 2));
    let patient = person(&mut app, at(4, 2));
    let listener = person(&mut app, at(9, 3));
    let radio = spawn(&mut app, "radio", at(9, 3));
    give(&mut app, listener, radio);
    let case = spawn(&mut app, "implant-case-death-alarm", at(3, 2));
    let implanter = spawn(&mut app, "implanter", at(3, 2));
    give(&mut app, doctor, implanter);
    act(&mut app, doctor, Action::UseOn(case));
    implant(&mut app, doctor, implanter, patient);
    app.world_mut().get_mut::<Vitals>(patient).unwrap().stat = MobStat::Dead;
    tick(&mut app, 2);
    assert!(told(&app, listener, "implant-death-alarm"));
}

#[test]
fn surgery_takes_an_implant_out() {
    assert!(Procedure::ALL.contains(&Procedure::ImplantRemoval));
    let mut app = setup();
    let doctor = person(&mut app, at(3, 2));
    let patient = person(&mut app, at(4, 2));
    let implanter = spawn(&mut app, "implanter-explosive", at(3, 2));
    let imp = implant(&mut app, doctor, implanter, patient);
    let zone = app.world().get::<Implant>(imp).unwrap().zone.unwrap();
    assert_eq!(implants::extract(app.world_mut(), patient, zone), Some(imp));
    assert_eq!(app.world().get::<Implant>(imp).unwrap().host, None);
    assert_eq!(app.world().get::<GridPos>(imp).unwrap().0, at(4, 2));
    assert!(app.world().get::<ChoosingPhrase>(patient).is_none());
}
