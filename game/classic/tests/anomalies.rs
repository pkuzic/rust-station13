//! Anomalies. Spec: docs/reference/anomalies.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::anomalies::{
    self, AnomalyBattery, Artifact, ArtifactField, Effect, HarvestMode, Harvester, Kind, Mode, Trigger,
    Utilizer,
};
use rs13_classic::filing::PendingPicks;
use rs13_classic::flashes::Weakened;
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::health::{Body, Zone};
use rs13_classic::machines::MACHINE_TICKS;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::paperwork::Paper;
use rs13_world::{Appearance, GridPos, HumanLook, ObjectSpec, TileMap, TilePos};

/// The analyser and the harvester each with a pad east of them, and room to stand.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'a': (tile: (id: "floor"), objects: ["anomaly-analyser"]),
        'h': (tile: (id: "floor"), objects: ["anomaly-harvester"]),
    },
    rows: [
        "WWWWWWWWWWWWWWWW",
        "W..............W",
        "W.a............W",
        "W..............W",
        "W.h............W",
        "W..............W",
        "W..............W",
        "W..............W",
        "WWWWWWWWWWWWWWWW",
    ],
)"#;

fn at(x: i32, y: i32) -> TilePos {
    TilePos::new(x, y, 0)
}

const ANALYSER: TilePos = TilePos::new(2, 6, 0);
const ANALYSER_PAD: TilePos = TilePos::new(3, 6, 0);
const HARVESTER: TilePos = TilePos::new(2, 4, 0);
const HARVESTER_PAD: TilePos = TilePos::new(3, 4, 0);

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

/// A few machine cycles.
fn cycles(app: &mut App, n: u32) {
    tick(app, n * MACHINE_TICKS as u32);
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

fn pick(app: &mut App, mob: Entity, target: Entity, line: usize) {
    app.world_mut()
        .get_mut::<PendingPicks>(mob)
        .unwrap()
        .0
        .push((target, line));
    tick(app, 1);
}

fn hold(app: &mut App, mob: Entity, id: &str) -> Entity {
    let item = spawn_object(app.world_mut(), &ObjectSpec::new(id), at(1, 1)).unwrap();
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    let mut hands = app.world_mut().get_mut::<Hands>(mob).unwrap();
    let active = hands.active;
    *hands.slot_mut(active) = Some(item);
    item
}

fn effect(kind: Kind, mode: Mode, trigger: Trigger) -> Effect {
    Effect {
        kind,
        mode,
        trigger,
        range: 3,
        charge: 0,
        charge_max: 3,
        id: "sigma-417".to_owned(),
        emission: 5,
        active: false,
        strength: 0.0,
        field: Vec::new(),
        field_at: None,
    }
}

/// An artifact with just this effect.
fn artifact(app: &mut App, pos: TilePos, primary: Effect) -> Entity {
    let art = spawn_object(app.world_mut(), &ObjectSpec::new(anomalies::ARTIFACT), pos).unwrap();
    app.world_mut().entity_mut(art).insert(Artifact {
        look: 2,
        primary,
        secondary: None,
        busy: false,
    });
    art
}

fn thing_at<T: Component>(app: &mut App, pos: TilePos) -> Entity {
    let world = app.world_mut();
    let mut q = world.query_filtered::<(Entity, &GridPos), With<T>>();
    q.iter(world).find(|(_, p)| p.0 == pos).unwrap().0
}

fn active(app: &App, art: Entity) -> bool {
    app.world().get::<Artifact>(art).unwrap().primary.active
}

fn health(app: &App, mob: Entity) -> f32 {
    app.world().get::<Body>(mob).unwrap().health()
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
fn a_bare_hand_wakes_a_touch_artifact_and_its_touch_heals() {
    let mut app = setup();
    let art = artifact(
        &mut app,
        at(5, 2),
        effect(Kind::Heal, Mode::Touch, Trigger::Touch),
    );
    let mob = person(&mut app, at(5, 1));
    app.world_mut()
        .get_mut::<Body>(mob)
        .unwrap()
        .take_damage(Zone::Chest, 40.0, 0.0);
    let hurt = health(&app, mob);
    act(&mut app, mob, Action::UseOn(art));
    assert!(active(&app, art));
    assert_eq!(app.world().get::<Appearance>(art).unwrap().state, "ano21");
    assert!(health(&app, mob) > hurt);
    assert!(told(&app, mob, "anomaly-touch"));
    // Touched again, it goes back to sleep.
    act(&mut app, mob, Action::UseOn(art));
    assert!(!active(&app, art));
}

#[test]
fn gloves_keep_a_hand_off_it() {
    let mut app = setup();
    let art = artifact(
        &mut app,
        at(5, 2),
        effect(Kind::Heal, Mode::Touch, Trigger::Touch),
    );
    let mob = person(&mut app, at(5, 1));
    let gloves = spawn_object(app.world_mut(), &ObjectSpec::new("black-gloves"), at(5, 1)).unwrap();
    rs13_classic::jobs::wear(app.world_mut(), mob, gloves);
    act(&mut app, mob, Action::UseOn(art));
    assert!(!active(&app, art));
    assert!(told(&app, mob, "anomaly-touch-gloved"));
}

#[test]
fn only_a_hard_blow_sets_off_force() {
    let mut app = setup();
    let art = artifact(&mut app, at(5, 2), effect(Kind::Heal, Mode::Aura, Trigger::Force));
    let mob = person(&mut app, at(5, 1));
    hold(&mut app, mob, "crowbar");
    act(&mut app, mob, Action::UseOn(art));
    assert!(!active(&app, art), "a crowbar hits for 5");
    let crowbar = app.world().get::<Hands>(mob).unwrap().active_item().unwrap();
    app.world_mut().entity_mut(crowbar).despawn();
    hold(&mut app, mob, "fire-extinguisher");
    act(&mut app, mob, Action::UseOn(art));
    assert!(active(&app, art), "an extinguisher hits for 12");
}

#[test]
fn a_hurting_aura_reaches_only_so_far() {
    let mut app = setup();
    let mut hurt = effect(Kind::Hurt, Mode::Aura, Trigger::Touch);
    hurt.active = true;
    hurt.range = 2;
    artifact(&mut app, at(8, 4), hurt);
    let near = person(&mut app, at(9, 4));
    let far = person(&mut app, at(13, 4));
    cycles(&mut app, 3);
    assert!(health(&app, near) < 100.0);
    assert_eq!(health(&app, far), 100.0);
}

#[test]
fn a_stunning_pulse_comes_once_charged() {
    let mut app = setup();
    let mut stun = effect(Kind::Stun, Mode::Pulse, Trigger::Touch);
    stun.active = true;
    artifact(&mut app, at(8, 4), stun);
    let mob = person(&mut app, at(9, 4));
    cycles(&mut app, 4);
    assert!(app.world().get::<Weakened>(mob).is_some());
    assert!(told(&app, mob, "anomaly-stun-pulse"));
}

#[test]
fn the_air_sets_off_a_gas_trigger() {
    let mut app = setup();
    let art = artifact(
        &mut app,
        at(8, 4),
        effect(Kind::Heal, Mode::Aura, Trigger::Nitrogen),
    );
    let plasma = artifact(
        &mut app,
        at(10, 4),
        effect(Kind::Heal, Mode::Aura, Trigger::Plasma),
    );
    cycles(&mut app, 1);
    assert!(active(&app, art), "the station's air is mostly nitrogen");
    assert!(!active(&app, plasma));
}

#[test]
fn a_force_field_goes_up_round_it_and_follows_it() {
    let mut app = setup();
    let art = artifact(
        &mut app,
        at(8, 4),
        effect(Kind::ForceField, Mode::Touch, Trigger::Touch),
    );
    let mob = person(&mut app, at(8, 3));
    act(&mut app, mob, Action::UseOn(art));
    let tiles = |app: &mut App| {
        let world = app.world_mut();
        let mut q = world.query_filtered::<&GridPos, With<ArtifactField>>();
        let mut v: Vec<TilePos> = q.iter(world).map(|p| p.0).collect();
        v.sort_by_key(|t| (t.x, t.y));
        v
    };
    let up = tiles(&mut app);
    assert_eq!(up.len(), 16);
    assert!(up.contains(&at(10, 6)) && up.contains(&at(6, 2)));
    app.world_mut().entity_mut(art).insert(GridPos(at(9, 4)));
    tick(&mut app, 1);
    assert!(tiles(&mut app).contains(&at(11, 6)));
    act(&mut app, mob, Action::UseOn(art));
    assert!(tiles(&mut app).is_empty());
}

#[test]
fn the_analyser_reports_what_wakes_an_artifact() {
    let mut app = setup();
    let art = artifact(
        &mut app,
        ANALYSER_PAD,
        effect(Kind::Heal, Mode::Aura, Trigger::Acid),
    );
    let analyser = thing_at::<anomalies::Analyser>(&mut app, ANALYSER);
    let mob = person(&mut app, at(2, 5));
    act(&mut app, mob, Action::UseOn(analyser));
    pick(&mut app, mob, analyser, 1);
    assert!(app.world().get::<Artifact>(art).unwrap().busy);
    // Ten seconds.
    tick(&mut app, 200);
    let world = app.world_mut();
    let mut papers = world.query::<(&Paper, &GridPos)>();
    let (paper, _) = papers.iter(world).find(|(_, p)| p.0 == ANALYSER).unwrap();
    assert!(
        paper.text.contains("[msg:anomaly-report-device]"),
        "{}",
        paper.text
    );
    assert!(paper.text.contains("[msg:anomaly-emission-5]"));
    assert!(paper.text.contains("[msg:anomaly-trigger-physical]"));
    assert!(paper.name.ends_with("#1"));
    assert!(!app.world().get::<Artifact>(art).unwrap().busy);
}

#[test]
fn the_analyser_calls_anything_else_mundane() {
    let mut app = setup();
    spawn_object(app.world_mut(), &ObjectSpec::new("crowbar"), ANALYSER_PAD).unwrap();
    let analyser = thing_at::<anomalies::Analyser>(&mut app, ANALYSER);
    let mob = person(&mut app, at(2, 5));
    act(&mut app, mob, Action::UseOn(analyser));
    pick(&mut app, mob, analyser, 1);
    tick(&mut app, 200);
    let world = app.world_mut();
    let mut papers = world.query::<&Paper>();
    let paper = papers.iter(world).next().unwrap();
    assert!(
        paper
            .text
            .contains("[noun:crowbar] - [msg:anomaly-report-mundane]")
    );
}

#[test]
fn the_harvester_charges_a_battery_and_dumping_it_sets_the_effect_off() {
    let mut app = setup();
    let mut hurt = effect(Kind::Hurt, Mode::Aura, Trigger::Touch);
    hurt.range = 1;
    let art = artifact(&mut app, HARVESTER_PAD, hurt);
    let harvester = thing_at::<Harvester>(&mut app, HARVESTER);
    let mob = person(&mut app, at(2, 3));
    let battery = hold(&mut app, mob, anomalies::BATTERY);
    act(&mut app, mob, Action::UseOn(harvester));
    assert_eq!(
        app.world().get::<Harvester>(harvester).unwrap().battery,
        Some(battery)
    );
    assert!(app.world().get::<Hands>(mob).unwrap().active_item().is_none());
    act(&mut app, mob, Action::UseOn(harvester));
    // Charge, signature, eject, drain, harvest.
    pick(&mut app, mob, harvester, 4);
    assert_eq!(
        app.world().get::<Harvester>(harvester).unwrap().mode,
        HarvestMode::Harvesting
    );
    assert!(app.world().get::<Artifact>(art).unwrap().busy);
    cycles(&mut app, 5);
    let b = app.world().get::<AnomalyBattery>(battery).unwrap().clone();
    assert!(b.charge >= 4.0, "{}", b.charge);
    assert_eq!(b.effect.as_ref().unwrap().id, "sigma-417");
    // Halt, then dump: the battery's copy hurts whoever is near the harvester.
    pick(&mut app, mob, harvester, 1);
    assert!(!app.world().get::<Artifact>(art).unwrap().busy);
    pick(&mut app, mob, harvester, 3);
    assert_eq!(
        app.world().get::<Harvester>(harvester).unwrap().mode,
        HarvestMode::Dumping
    );
    cycles(&mut app, 3);
    assert!(health(&app, mob) < 100.0);
    let b = app.world().get::<AnomalyBattery>(battery).unwrap().clone();
    assert_eq!(b.charge, 0.0);
    assert!(!b.effect.unwrap().active);
    assert_eq!(
        app.world().get::<Harvester>(harvester).unwrap().mode,
        HarvestMode::Idle
    );
}

#[test]
fn the_harvester_wants_the_artifact_alone_on_its_pad() {
    let mut app = setup();
    artifact(
        &mut app,
        HARVESTER_PAD,
        effect(Kind::Heal, Mode::Aura, Trigger::Touch),
    );
    spawn_object(app.world_mut(), &ObjectSpec::new("crowbar"), HARVESTER_PAD).unwrap();
    let harvester = thing_at::<Harvester>(&mut app, HARVESTER);
    let mob = person(&mut app, at(2, 3));
    hold(&mut app, mob, anomalies::BATTERY);
    act(&mut app, mob, Action::UseOn(harvester));
    act(&mut app, mob, Action::UseOn(harvester));
    pick(&mut app, mob, harvester, 4);
    assert_eq!(
        app.world().get::<Harvester>(harvester).unwrap().mode,
        HarvestMode::Idle
    );
}

#[test]
fn the_utilizer_runs_a_battery_where_it_is_carried() {
    let mut app = setup();
    let mob = person(&mut app, at(8, 4));
    let near = person(&mut app, at(9, 4));
    let utilizer = hold(&mut app, mob, anomalies::UTILIZER);
    // A battery already charged with a hurting aura, in the other hand.
    let battery = spawn_object(app.world_mut(), &ObjectSpec::new(anomalies::BATTERY), at(1, 1)).unwrap();
    let mut hurt = effect(Kind::Hurt, Mode::Aura, Trigger::Touch);
    hurt.range = 2;
    app.world_mut().entity_mut(battery).remove::<GridPos>().insert((
        HeldBy(mob),
        AnomalyBattery {
            effect: Some(hurt),
            charge: 3.0,
            capacity: 200.0,
        },
    ));
    {
        let mut hands = app.world_mut().get_mut::<Hands>(mob).unwrap();
        let active = hands.active;
        *hands.slot_mut(active) = Some(battery);
        let other = if active == rs13_world::Hand::Left {
            rs13_world::Hand::Right
        } else {
            rs13_world::Hand::Left
        };
        *hands.slot_mut(other) = Some(utilizer);
    }
    act(&mut app, mob, Action::UseOn(utilizer));
    assert_eq!(
        app.world().get::<Utilizer>(utilizer).unwrap().battery,
        Some(battery)
    );
    // Now the utilizer in the active hand, used in the hand: its list.
    {
        let mut hands = app.world_mut().get_mut::<Hands>(mob).unwrap();
        let active = hands.active;
        *hands.slot_mut(active) = Some(utilizer);
        let other = if active == rs13_world::Hand::Left {
            rs13_world::Hand::Right
        } else {
            rs13_world::Hand::Left
        };
        *hands.slot_mut(other) = None;
    }
    act(&mut app, mob, Action::UseOn(utilizer));
    // Status, id, power, time, four timer lines, start.
    pick(&mut app, mob, utilizer, 8);
    assert!(app.world().get::<Utilizer>(utilizer).unwrap().active);
    cycles(&mut app, 4);
    assert!(health(&app, near) < 100.0);
    let u = *app.world().get::<Utilizer>(utilizer).unwrap();
    assert!(!u.active, "three charge lasts three cycles");
    assert!(u.cooldown > 0);
    assert_eq!(app.world().get::<AnomalyBattery>(battery).unwrap().charge, 0.0);
}

#[test]
fn a_heavy_blast_destroys_it() {
    let mut app = setup();
    let art = artifact(&mut app, at(8, 4), effect(Kind::Heal, Mode::Aura, Trigger::Touch));
    rs13_classic::explosions::strike(app.world_mut(), at(8, 4), 1);
    assert!(app.world().get_entity(art).is_err());
}

#[test]
fn a_light_blast_wakes_a_heat_trigger() {
    let mut app = setup();
    let art = artifact(&mut app, at(8, 4), effect(Kind::Heal, Mode::Aura, Trigger::Heat));
    rs13_classic::explosions::strike(app.world_mut(), at(8, 4), 3);
    assert!(active(&app, art));
}

#[test]
fn spawned_artifacts_roll_their_effects() {
    let mut app = setup();
    for x in 4..14 {
        let art = spawn_object(app.world_mut(), &ObjectSpec::new(anomalies::ARTIFACT), at(x, 2)).unwrap();
        let a = app.world().get::<Artifact>(art).unwrap();
        assert!(a.look <= 5);
        assert!(a.primary.range >= 1 && a.primary.charge_max >= 3);
        if a.primary.kind == Kind::Emp {
            assert_eq!(a.primary.mode, Mode::Pulse);
        }
        if a.primary.kind == Kind::ForceField {
            assert_eq!(a.primary.trigger, Trigger::Touch);
        }
    }
}
