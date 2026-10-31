//! Bruise packs, ointment, health analyzers and first aid kits. Spec:
//! docs/reference/medical.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::hands::{Hands, HeldBy, Stack};
use rs13_classic::health::{Body, Zone};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::storage::Storage;
use rs13_world::{Appearance, GridPos, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
    },
    rows: [
        "WWWWW",
        "W...W",
        "WWWWW",
    ],
)"#;

const MEDIC_AT: TilePos = TilePos::new(1, 1, 0);
const PATIENT_AT: TilePos = TilePos::new(2, 1, 0);

fn setup() -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let medic = spawn_player_mob(app.world_mut(), MEDIC_AT);
    let patient = spawn_player_mob(app.world_mut(), PATIENT_AT);
    ticks(&mut app, 1);
    (app, medic, patient)
}

fn ticks(app: &mut App, n: u64) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn give(app: &mut App, mob: Entity, proto: &str) -> Entity {
    let item = spawn_object(app.world_mut(), &ObjectSpec::new(proto), MEDIC_AT).unwrap();
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    app.world_mut().get_mut::<Hands>(mob).unwrap().right = Some(item);
    item
}

fn use_on(app: &mut App, mob: Entity, target: Entity) -> Vec<String> {
    let before = app.world().get::<Outbox>(mob).unwrap().0.len();
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    ticks(app, 1);
    app.world().get::<Outbox>(mob).unwrap().0[before..]
        .iter()
        .map(|m| m.id.clone())
        .collect()
}

fn chest(app: &App, mob: Entity) -> (f32, f32) {
    let organ = app.world().get::<Body>(mob).unwrap().organ(Zone::Chest);
    (organ.brute, organ.burn)
}

fn hurt(app: &mut App, mob: Entity, brute: f32, burn: f32) {
    app.world_mut()
        .get_mut::<Body>(mob)
        .unwrap()
        .take_damage(Zone::Chest, brute, burn);
    ticks(app, 1);
}

#[test]
fn a_bruise_pack_heals_brute_on_the_chest_and_is_used_up() {
    let (mut app, medic, patient) = setup();
    hurt(&mut app, patient, 50.0, 10.0);
    let pack = give(&mut app, medic, "bruise-pack");
    assert_eq!(app.world().get::<Stack>(pack).unwrap().amount, 10);

    let said = use_on(&mut app, medic, patient);
    assert!(said.contains(&"medical-applied".to_owned()), "{said:?}");
    assert_eq!(chest(&app, patient), (20.0, 10.0), "Luna: 30 brute, no burn");
    assert_eq!(app.world().get::<Stack>(pack).unwrap().amount, 9);

    // The last one in the stack is gone once used.
    app.world_mut().get_mut::<Stack>(pack).unwrap().amount = 1;
    use_on(&mut app, medic, patient);
    assert_eq!(chest(&app, patient), (0.0, 10.0), "never below zero");
    ticks(&mut app, 1);
    assert!(app.world().get_entity(pack).is_err());
    assert_eq!(app.world().get::<Hands>(medic).unwrap().right, None);
}

#[test]
fn ointment_heals_burns_and_works_on_yourself() {
    let (mut app, medic, _) = setup();
    hurt(&mut app, medic, 5.0, 30.0);
    give(&mut app, medic, "ointment");
    let said = use_on(&mut app, medic, medic);
    assert!(said.contains(&"medical-applied-self".to_owned()), "{said:?}");
    assert_eq!(chest(&app, medic), (5.0, 10.0), "Luna: 20 burn");
}

#[test]
fn packs_do_nothing_for_someone_in_critical_condition() {
    let (mut app, medic, patient) = setup();
    hurt(&mut app, patient, 120.0, 0.0);
    let pack = give(&mut app, medic, "bruise-pack");
    use_on(&mut app, medic, patient);
    assert_eq!(chest(&app, patient).0, 120.0);
    assert_eq!(app.world().get::<Stack>(pack).unwrap().amount, 10);
}

#[test]
fn the_analyzer_reads_out_health_and_damage() {
    let (mut app, medic, patient) = setup();
    hurt(&mut app, patient, 12.0, 3.0);
    give(&mut app, medic, "health-analyzer");
    let said = use_on(&mut app, medic, patient);
    assert_eq!(
        said,
        [
            "analyzer-used-self",
            "analyzer-status",
            "analyzer-damage",
            "analyzer-temperature",
            "analyzer-localized",
            "analyzer-organ",
            "analyzer-summary",
        ]
    );
    let outbox = &app.world().get::<Outbox>(medic).unwrap().0;
    let status = outbox.iter().find(|m| m.id == "analyzer-status").unwrap();
    assert!(format!("{status:?}").contains("85"), "{status:?}");
    assert_eq!(chest(&app, patient), (12.0, 3.0), "scanning heals nothing");
}

#[test]
fn a_first_aid_kit_comes_packed() {
    let (mut app, _, _) = setup();
    let kit = spawn_object(app.world_mut(), &ObjectSpec::new("first-aid-kit"), MEDIC_AT).unwrap();
    let contents: Vec<String> = app
        .world()
        .get::<Storage>(kit)
        .unwrap()
        .items
        .iter()
        .map(|e| app.world().get::<Appearance>(*e).unwrap().sprite.clone())
        .collect();
    assert_eq!(
        contents,
        [
            "bruise-pack",
            "bruise-pack",
            "ointment",
            "ointment",
            "health-analyzer"
        ]
    );
}
