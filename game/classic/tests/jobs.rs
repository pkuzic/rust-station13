//! Jobs. Spec: docs/reference/jobs.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::access::{IdCard, allowed};
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::equipment::Equipment;
use rs13_classic::hands::Hands;
use rs13_classic::jobs::{
    CHOOSING_DS, ChoosingJob, HasJob, JobCard, JobPanelOut, PendingJobOps, jobs, offer, pick_start, take_up,
};
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

fn line_of(job: &str) -> usize {
    jobs().iter().position(|j| j.id == job).unwrap()
}

fn answer(app: &mut App, mob: Entity, line: Option<usize>) {
    app.world_mut()
        .get_mut::<PendingJobOps>(mob)
        .unwrap()
        .0
        .push(line);
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(FixedUpdate);
}

fn arrive(app: &mut App, at: TilePos) -> Entity {
    let mob = spawn_player_mob(app.world_mut(), at);
    assert!(offer(app.world_mut(), mob, at));
    app.world_mut().run_schedule(FixedUpdate);
    mob
}

#[test]
fn a_new_arrival_chooses_before_they_are_in_the_world() {
    let mut app = setup();
    let offered = card_for(&mut app, A, "engineer", &["engine"]);
    let chosen = card_for(&mut app, C, "medical-doctor", &["medical"]);
    let mob = arrive(&mut app, A);

    assert!(app.world().get::<Hidden>(mob).is_some(), "not in the world yet");
    assert!(
        app.world().get::<JobCard>(offered).is_some(),
        "nothing handed out"
    );
    assert_eq!(sprite_in(&app, mob, Slot::Uniform), "grey-jumpsuit");
    let view = app
        .world()
        .get::<JobPanelOut>(mob)
        .unwrap()
        .0
        .clone()
        .expect("offered");
    assert_eq!(view.current, "engineer");
    assert_eq!(view.rows[line_of("medical-doctor")].1, 1);
    assert_eq!(view.rows[line_of("engineer")].1, 0, "the post is held for them");

    answer(&mut app, mob, Some(line_of("medical-doctor")));

    assert!(app.world().get::<Hidden>(mob).is_none());
    assert!(app.world().get::<ChoosingJob>(mob).is_none());
    assert_eq!(app.world().get::<HasJob>(mob).unwrap().0, "medical-doctor");
    assert_eq!(app.world().get::<GridPos>(mob).unwrap().0, C);
    assert_eq!(sprite_in(&app, mob, Slot::Uniform), "medical-jumpsuit");
    assert_eq!(
        app.world().get::<Equipment>(mob).unwrap().get(Slot::Id),
        Some(chosen)
    );
    assert_eq!(
        app.world().get::<JobCard>(offered).unwrap().0,
        "engineer",
        "still there"
    );
    assert!(app.world().get::<Hidden>(offered).is_some());
    assert!(app.world().get::<JobPanelOut>(mob).unwrap().0.is_none());

    answer(&mut app, mob, Some(line_of("engineer")));
    assert_eq!(
        app.world().get::<HasJob>(mob).unwrap().0,
        "medical-doctor",
        "only once"
    );
}

#[test]
fn staying_starts_them_and_a_full_job_leaves_the_window_open() {
    let mut app = setup();
    let card = card_for(&mut app, A, "engineer", &["engine"]);
    let mob = arrive(&mut app, A);
    answer(&mut app, mob, Some(line_of("captain")));
    assert!(
        app.world().get::<ChoosingJob>(mob).is_some(),
        "no post for it: choose again"
    );
    assert!(app.world().get::<JobPanelOut>(mob).unwrap().0.is_some());

    answer(&mut app, mob, None);
    assert_eq!(app.world().get::<HasJob>(mob).unwrap().0, "engineer");
    assert_eq!(
        app.world().get::<Equipment>(mob).unwrap().get(Slot::Id),
        Some(card)
    );
    assert!(app.world().get::<Hidden>(mob).is_none());
}

#[test]
fn whoever_takes_too_long_starts_in_the_job_they_were_offered() {
    let mut app = setup();
    let card = card_for(&mut app, A, "engineer", &["engine"]);
    let mob = arrive(&mut app, A);
    for _ in 0..(CHOOSING_DS as u64) * 2 + 2 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    assert!(app.world().get::<ChoosingJob>(mob).is_none());
    assert!(app.world().get::<Hidden>(mob).is_none());
    assert_eq!(
        app.world().get::<Equipment>(mob).unwrap().get(Slot::Id),
        Some(card)
    );
}

#[test]
fn nobody_is_asked_on_a_map_without_jobs() {
    let mut app = setup();
    let mob = spawn_player_mob(app.world_mut(), A);
    assert!(!offer(app.world_mut(), mob, A));
    assert!(app.world().get::<ChoosingJob>(mob).is_none());
}
