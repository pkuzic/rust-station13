//! Jobs. Spec: docs/reference/jobs.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::access::{IdCard, allowed};
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::equipment::Equipment;
use rs13_classic::hands::Hands;
use rs13_classic::jobs::{HasJob, JobCard, jobs, pick_start, take_up};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::storage::Storage;
use rs13_classic::{ClassicPlugin, locales};
use rs13_i18n::{Lexicon, Localizer, langid};
use rs13_world::{Appearance, GridPos, Hidden, ObjectSpec, Slot, TileMap, TilePos};

const MAP: &str = r#"(
    legend: { 'W': (tile: (id: "wall", solid: true)), '.': (tile: (id: "floor")) },
    rows: ["WWWWWWW", "W.....W", "WWWWWWW"],
)"#;
const A: TilePos = TilePos::new(1, 1, 0);
const B: TilePos = TilePos::new(2, 1, 0);
const C: TilePos = TilePos::new(3, 1, 0);

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app
}

fn card_for(app: &mut App, at: TilePos, job: &str, access: &[&str]) -> Entity {
    let mut spec = ObjectSpec::new("id-card");
    spec.access = access.iter().map(|a| (*a).to_owned()).collect();
    spec.vars.insert("job".to_owned(), format!("{job:?}"));
    spawn_object(app.world_mut(), &spec, at).unwrap()
}

fn sprite_in(app: &App, mob: Entity, slot: Slot) -> String {
    let item = app
        .world()
        .get::<Equipment>(mob)
        .unwrap()
        .get(slot)
        .expect("worn");
    app.world().get::<Appearance>(item).unwrap().sprite.clone()
}

#[test]
fn every_job_is_kitted_out_with_things_that_exist() {
    let mut app = setup();
    assert_eq!(jobs().len(), 26);
    for job in jobs() {
        for proto in job.wears.iter().chain(&job.holds).chain(&job.packed) {
            let thing = spawn_object(app.world_mut(), &ObjectSpec::new(proto.as_str()), A).expect(proto);
            app.world_mut().entity_mut(thing).despawn();
        }
    }
}

#[test]
fn every_job_has_a_name_in_both_languages() {
    for ftl in [
        include_str!("../locales/en/messages.ftl"),
        include_str!("../locales/ru/messages.ftl"),
    ] {
        for job in jobs() {
            assert!(ftl.contains(&format!("\njob-{} = ", job.id)), "{}", job.id);
        }
    }
}

#[test]
fn starting_at_a_job_gives_its_card_clothes_and_tools() {
    let mut app = setup();
    let card = card_for(&mut app, A, "engineer", &["engine"]);
    let mob = spawn_player_mob(app.world_mut(), A);
    assert_eq!(take_up(app.world_mut(), mob, A).as_deref(), Some("engineer"));
    app.world_mut().run_schedule(FixedUpdate);

    assert_eq!(app.world().get::<HasJob>(mob).unwrap().0, "engineer");
    let told = app.world().get::<Outbox>(mob).unwrap().0.last().unwrap().clone();
    assert_eq!(told.id, "job-assigned");
    let lexicon = Lexicon::from_ron(locales::EN_NOUNS).unwrap();
    let en = Localizer::new(langid!("en"), &[locales::EN_MESSAGES], lexicon).unwrap();
    assert_eq!(en.format_msg(&told), "You are the Engineer.");
    assert_eq!(
        app.world().get::<Equipment>(mob).unwrap().get(Slot::Id),
        Some(card)
    );
    assert!(app.world().get::<Hidden>(card).is_none());
    assert!(app.world().get::<JobCard>(card).is_none());
    assert!(allowed(&["engine".to_owned()], app.world().get::<IdCard>(card)));
    assert_eq!(sprite_in(&app, mob, Slot::Uniform), "engineer-jumpsuit");
    assert_eq!(sprite_in(&app, mob, Slot::Shoes), "orange-shoes");

    let held = app
        .world()
        .get::<Hands>(mob)
        .unwrap()
        .active_item()
        .expect("toolbox");
    assert_eq!(
        app.world().get::<Appearance>(held).unwrap().sprite,
        "toolbox-mechanical"
    );
    let pack = app
        .world()
        .get::<Equipment>(mob)
        .unwrap()
        .get(Slot::Back)
        .expect("pack");
    assert_eq!(app.world().get::<Storage>(pack).unwrap().items.len(), 2);
}

#[test]
fn a_waiting_card_cant_be_taken_or_seen() {
    let mut app = setup();
    let card = card_for(&mut app, B, "captain", &["heads"]);
    let mob = spawn_player_mob(app.world_mut(), A);
    app.world_mut().run_schedule(FixedUpdate);
    assert!(app.world().get::<Hidden>(card).is_some());
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(card));
    app.world_mut().run_schedule(FixedUpdate);
    assert!(app.world().get::<Hands>(mob).unwrap().active_item().is_none());
    assert!(app.world().get::<GridPos>(card).is_some());
}

#[test]
fn people_spread_across_jobs_rather_than_filling_one() {
    let mut app = setup();
    card_for(&mut app, A, "engineer", &["engine"]);
    card_for(&mut app, B, "engineer", &["engine"]);
    card_for(&mut app, C, "medical-doctor", &["medical"]);
    let starts = [A, B, C];
    assert_eq!(
        pick_start(app.world_mut(), &starts, &[]),
        Some(A),
        "engineer is first"
    );
    let first = spawn_player_mob(app.world_mut(), A);
    take_up(app.world_mut(), first, A);
    assert_eq!(
        pick_start(app.world_mut(), &starts, &[A]),
        Some(C),
        "not a second engineer yet"
    );
    let second = spawn_player_mob(app.world_mut(), C);
    take_up(app.world_mut(), second, C);
    assert_eq!(pick_start(app.world_mut(), &starts, &[A, C]), Some(B));
    assert_eq!(pick_start(app.world_mut(), &starts, &[A, B, C]), None);
}

#[test]
fn a_map_without_jobs_starts_people_as_before() {
    let mut app = setup();
    let starts = [A, B];
    assert_eq!(pick_start(app.world_mut(), &starts, &[]), Some(A));
    let mob = spawn_player_mob(app.world_mut(), A);
    assert_eq!(take_up(app.world_mut(), mob, A), None);
    assert_eq!(sprite_in(&app, mob, Slot::Uniform), "grey-jumpsuit");
    assert!(app.world().get::<Equipment>(mob).unwrap().get(Slot::Id).is_none());
}
