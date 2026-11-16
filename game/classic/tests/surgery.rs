//! Surgery. Spec: docs/reference/surgery.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::chairs::Resting;
use rs13_classic::combat::AimZone;
use rs13_classic::filing::PendingPicks;
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::health::{Vitals, Zone};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::surgery::{self, DrapeChoice, OnTable, Procedure, Removed, Surgeries};
use rs13_world::{Appearance, DisplayName, GridPos, HumanLook, MobStat, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'T': (tile: (id: "floor"), objects: ["operating-table"]),
    },
    rows: [
        "WWWWWW",
        "W.T..W",
        "W....W",
        "WWWWWW",
    ],
)"#;

const TABLE: TilePos = TilePos::new(2, 2, 0);
const BESIDE: TilePos = TilePos::new(2, 1, 0);

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    tick(&mut app, 2);
    app
}

fn tick(app: &mut App, n: u32) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn person(app: &mut App, at: TilePos, name: &str) -> Entity {
    let mob = spawn_player_mob(app.world_mut(), at);
    app.world_mut()
        .entity_mut(mob)
        .insert((HumanLook::default(), DisplayName(name.to_owned())));
    mob
}

/// Someone lying on the operating table.
fn patient(app: &mut App) -> Entity {
    let mob = person(app, TABLE, "Pat");
    app.world_mut().entity_mut(mob).insert((Resting, OnTable(TABLE)));
    tick(app, 1);
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

/// Put a fresh item in the surgeon's active hand, or empty it.
fn hold(app: &mut App, mob: Entity, id: Option<&str>) {
    let old = app.world().get::<Hands>(mob).and_then(Hands::active_item);
    if let Some(old) = old {
        app.world_mut().entity_mut(old).despawn();
    }
    let item = id.map(|id| {
        let item = spawn_object(app.world_mut(), &ObjectSpec::new(id), BESIDE).unwrap();
        app.world_mut()
            .entity_mut(item)
            .remove::<GridPos>()
            .insert(HeldBy(mob));
        item
    });
    let mut hands = app.world_mut().get_mut::<Hands>(mob).unwrap();
    let active = hands.active;
    *hands.slot_mut(active) = item;
}

fn aim(app: &mut App, mob: Entity, zone: Zone) {
    app.world_mut().entity_mut(mob).insert(AimZone(zone));
}

fn drape(app: &mut App, surgeon: Entity, patient: Entity, procedure: Procedure) {
    hold(app, surgeon, Some("surgical-drapes"));
    act(app, surgeon, Action::UseOn(patient));
    let choice = app.world().get::<DrapeChoice>(surgeon).cloned().unwrap();
    let line = choice.offered.iter().position(|p| *p == procedure).unwrap();
    app.world_mut()
        .get_mut::<PendingPicks>(surgeon)
        .unwrap()
        .0
        .push((patient, line));
    tick(app, 1);
}

/// Do a step with a tool (or an empty hand) and wait it out.
fn step(app: &mut App, surgeon: Entity, patient: Entity, tool: Option<&str>) {
    hold(app, surgeon, tool);
    act(app, surgeon, Action::UseOn(patient));
    // The longest step is 6.4 s, at 20 ticks a second.
    tick(app, 140);
}

fn steps_left(app: &App, patient: Entity) -> Option<usize> {
    app.world()
        .get::<Surgeries>(patient)
        .and_then(|s| s.0.first().map(|o| o.procedure.steps().len() - o.step))
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
fn drapes_offer_what_the_part_allows() {
    let mut app = setup();
    let pat = patient(&mut app);
    let doc = person(&mut app, BESIDE, "Doc");
    aim(&mut app, doc, Zone::Head);
    hold(&mut app, doc, Some("surgical-drapes"));
    act(&mut app, doc, Action::UseOn(pat));
    let offered = app.world().get::<DrapeChoice>(doc).unwrap().offered.clone();
    assert_eq!(
        offered,
        [
            Procedure::BrainRemoval,
            Procedure::EyeSurgery,
            Procedure::PlasticSurgery,
            Procedure::CavityImplant
        ]
    );
}

#[test]
fn an_appendectomy_on_the_table_takes_the_appendix_out() {
    let mut app = setup();
    let pat = patient(&mut app);
    let doc = person(&mut app, BESIDE, "Doc");
    aim(&mut app, doc, Zone::Groin);
    drape(&mut app, doc, pat, Procedure::Appendectomy);
    assert_eq!(steps_left(&app, pat), Some(6));
    for tool in [
        Some("scalpel"),
        Some("hemostat"),
        Some("retractor"),
        Some("scalpel"),
        None,
    ] {
        step(&mut app, doc, pat, tool);
    }
    assert!(app.world().get::<Removed>(pat).unwrap().appendix);
    let world = app.world_mut();
    let mut q = world.query::<(&Appearance, &GridPos)>();
    assert!(
        q.iter(world)
            .any(|(a, p)| a.sprite == surgery::APPENDIX && p.0 == TABLE)
    );
    step(&mut app, doc, pat, Some("cautery"));
    assert_eq!(steps_left(&app, pat), None);
}

#[test]
fn the_wrong_tool_does_nothing_for_the_operation() {
    let mut app = setup();
    let pat = patient(&mut app);
    let doc = person(&mut app, BESIDE, "Doc");
    aim(&mut app, doc, Zone::Groin);
    drape(&mut app, doc, pat, Procedure::Appendectomy);
    step(&mut app, doc, pat, Some("crowbar"));
    assert_eq!(steps_left(&app, pat), Some(6));
}

#[test]
fn walking_away_stops_a_step() {
    let mut app = setup();
    let pat = patient(&mut app);
    let doc = person(&mut app, BESIDE, "Doc");
    aim(&mut app, doc, Zone::Groin);
    drape(&mut app, doc, pat, Procedure::Appendectomy);
    hold(&mut app, doc, Some("scalpel"));
    act(&mut app, doc, Action::UseOn(pat));
    app.world_mut()
        .entity_mut(doc)
        .insert(GridPos(TilePos::new(3, 1, 0)));
    tick(&mut app, 60);
    assert_eq!(steps_left(&app, pat), Some(6));
}

#[test]
fn clothes_in_the_way_must_come_off_first() {
    let mut app = setup();
    let pat = patient(&mut app);
    let suit = spawn_object(app.world_mut(), &ObjectSpec::new("forensic-jumpsuit"), TABLE).unwrap();
    rs13_classic::jobs::wear(app.world_mut(), pat, suit);
    let doc = person(&mut app, BESIDE, "Doc");
    aim(&mut app, doc, Zone::Groin);
    hold(&mut app, doc, Some("surgical-drapes"));
    act(&mut app, doc, Action::UseOn(pat));
    assert!(told(&app, doc, "surgery-expose"));
    assert!(app.world().get::<DrapeChoice>(doc).is_none());
}

#[test]
fn taking_the_brain_out_kills() {
    let mut app = setup();
    let pat = patient(&mut app);
    let doc = person(&mut app, BESIDE, "Doc");
    aim(&mut app, doc, Zone::Head);
    drape(&mut app, doc, pat, Procedure::BrainRemoval);
    for tool in ["scalpel", "hemostat", "retractor", "circular-saw", "hemostat"] {
        step(&mut app, doc, pat, Some(tool));
    }
    assert!(app.world().get::<Removed>(pat).unwrap().brain);
    assert_eq!(app.world().get::<Vitals>(pat).unwrap().stat, MobStat::Dead);
}

#[test]
fn gender_reassignment_flips_how_they_look() {
    let mut app = setup();
    let pat = patient(&mut app);
    let doc = person(&mut app, BESIDE, "Doc");
    aim(&mut app, doc, Zone::Groin);
    drape(&mut app, doc, pat, Procedure::GenderReassignment);
    for tool in ["scalpel", "hemostat", "scalpel"] {
        step(&mut app, doc, pat, Some(tool));
    }
    assert!(app.world().get::<HumanLook>(pat).unwrap().female);
}

#[test]
fn a_cavity_holds_something_small() {
    let mut app = setup();
    let pat = patient(&mut app);
    let doc = person(&mut app, BESIDE, "Doc");
    aim(&mut app, doc, Zone::Chest);
    drape(&mut app, doc, pat, Procedure::CavityImplant);
    for tool in ["scalpel", "hemostat", "retractor", "scalpel", "pen"] {
        step(&mut app, doc, pat, Some(tool));
    }
    let cavities = app.world().get::<surgery::Cavities>(pat).unwrap();
    assert_eq!(cavities.0.len(), 1);
    assert_eq!(cavities.0[0].0, Zone::Chest);
}

#[test]
fn off_the_table_they_stand_up() {
    let mut app = setup();
    let pat = patient(&mut app);
    app.world_mut()
        .entity_mut(pat)
        .insert(GridPos(TilePos::new(3, 2, 0)));
    tick(&mut app, 2);
    assert!(app.world().get::<OnTable>(pat).is_none());
    assert!(app.world().get::<Resting>(pat).is_none());
}
