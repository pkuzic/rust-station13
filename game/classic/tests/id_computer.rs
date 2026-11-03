//! The identification computer. Spec: docs/reference/access.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::access::IdCard;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::equipment::{Equipment, WornBy};
use rs13_classic::hands::Hands;
use rs13_classic::id_computer::{CHANGE_IDS, IdComputer, IdOp, IdPanelOut, PendingIdOps, examine};
use rs13_classic::jobs::{CardOwner, jobs, take_up};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_world::{DisplayName, GridPos, ObjectSpec, Slot, TileMap, TilePos};

const MAP: &str = r#"(
    legend: { 'W': (tile: (id: "wall", solid: true)), '.': (tile: (id: "floor")) },
    rows: ["WWWWWW", "W....W", "WWWWWW"],
)"#;
const NEAR: TilePos = TilePos::new(1, 1, 0);
const AT: TilePos = TilePos::new(2, 1, 0);

fn setup(worn_access: &[&str]) -> (App, Entity, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), NEAR);
    let worn = spawn_object(app.world_mut(), &ObjectSpec::new("id-card"), NEAR).unwrap();
    app.world_mut().entity_mut(worn).remove::<GridPos>().insert((
        WornBy(mob),
        IdCard {
            access: worn_access.iter().map(|a| (*a).to_owned()).collect(),
        },
    ));
    app.world_mut()
        .get_mut::<Equipment>(mob)
        .unwrap()
        .0
        .insert(Slot::Id, worn);
    let machine = spawn_object(app.world_mut(), &ObjectSpec::new("id-computer"), AT).unwrap();
    let card = spawn_object(app.world_mut(), &ObjectSpec::new("id-card"), NEAR).unwrap();
    app.world_mut().run_schedule(FixedUpdate);
    click(&mut app, mob, card);
    click(&mut app, mob, machine);
    assert_eq!(app.world().get::<IdComputer>(machine).unwrap().card, Some(card));
    assert!(app.world().get::<Hands>(mob).unwrap().active_item().is_none());
    (app, mob, machine, card)
}

fn click(app: &mut App, mob: Entity, target: Entity) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    app.world_mut().run_schedule(FixedUpdate);
}

fn press(app: &mut App, mob: Entity, machine: Entity, op: IdOp) {
    app.world_mut()
        .get_mut::<PendingIdOps>(mob)
        .unwrap()
        .0
        .push((machine, op));
    app.world_mut().run_schedule(FixedUpdate);
}

fn line_of(job: &str) -> usize {
    jobs().iter().position(|j| j.id == job).unwrap()
}

#[test]
fn it_gives_a_card_another_jobs_access() {
    let (mut app, mob, machine, card) = setup(&[CHANGE_IDS]);
    app.world_mut().run_schedule(FixedUpdate);
    let view = app
        .world()
        .get::<IdPanelOut>(mob)
        .unwrap()
        .0
        .clone()
        .expect("open");
    assert!(view.has_card);
    assert_eq!(view.jobs.len(), 26);

    press(&mut app, mob, machine, IdOp::Assign(line_of("chemist")));
    let access = &app.world().get::<IdCard>(card).unwrap().access;
    assert!(access.contains(&"chemistry".to_owned()));
    assert_eq!(app.world().get::<CardOwner>(card).unwrap().job, "chemist");
}

#[test]
fn without_change_ids_it_refuses_but_still_gives_the_card_back() {
    let (mut app, mob, machine, card) = setup(&["medical"]);
    press(&mut app, mob, machine, IdOp::Assign(line_of("chemist")));
    assert!(app.world().get::<IdCard>(card).unwrap().access.is_empty());
    assert_eq!(
        app.world().get::<Outbox>(mob).unwrap().0.last().unwrap().id,
        "records-denied"
    );

    press(&mut app, mob, machine, IdOp::Eject);
    assert!(app.world().get::<IdComputer>(machine).unwrap().card.is_none());
    assert_eq!(app.world().get::<GridPos>(card).unwrap().0, AT);
}

#[test]
fn a_jobs_card_says_whose_it_is() {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    let mut spec = ObjectSpec::new("id-card");
    spec.vars.insert("job".to_owned(), "\"engineer\"".to_owned());
    let card = spawn_object(app.world_mut(), &spec, NEAR).unwrap();
    let mob = spawn_player_mob(app.world_mut(), NEAR);
    app.world_mut()
        .entity_mut(mob)
        .insert(DisplayName("Pat Doe".to_owned()));
    take_up(app.world_mut(), mob, NEAR);
    let told = examine(app.world(), card).expect("a message");
    assert_eq!(told.id, "examine-card");
    let locale = rs13_i18n::Lexicon::from_ron(rs13_classic::locales::EN_NOUNS).unwrap();
    let en = rs13_i18n::Localizer::new(
        rs13_i18n::langid!("en"),
        &[rs13_classic::locales::EN_MESSAGES],
        locale,
    )
    .unwrap();
    assert_eq!(en.format_msg(&told), "It belongs to Pat Doe, Engineer.");
}

#[test]
fn every_job_has_access() {
    for job in jobs() {
        assert!(!job.access.is_empty(), "{}", job.id);
    }
}
