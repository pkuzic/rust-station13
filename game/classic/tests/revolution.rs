//! Revolution and RP revolution. Spec: docs/reference/revolution.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::Outbox;
use rs13_classic::health::Vitals;
use rs13_classic::jobs::HasJob;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::revolution::{
    HEADS, HeadRev, RevAsk, RevState, Revolutionary, add_revolutionary, answer, ask, flashed, members, over,
    remove_revolutionary, report, setup_round,
};
use rs13_classic::round::{Phase, Round};
use rs13_classic::security::Cuffed;
use rs13_world::{DisplayName, GridPos, MobStat, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        '.': (tile: (id: "floor")),
    },
    rows: [
        "..........",
        "..........",
        "..........",
        "..........",
        "..........",
        "..........",
        "..........",
        "..........",
        "..........",
        "..........",
    ],
)"#;

const AT: TilePos = TilePos::new(2, 2, 0);

fn setup(mode: &'static str) -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().insert_resource(Round {
        phase: Phase::Playing { mode },
        forced_mode: None,
    });
    app.world_mut().run_schedule(FixedUpdate);
    app
}

fn crew(app: &mut App, job: &str) -> Entity {
    let mob = spawn_player_mob(app.world_mut(), AT);
    app.world_mut()
        .entity_mut(mob)
        .insert((HasJob(job.to_owned()), DisplayName(format!("The {job}"))));
    mob
}

fn tick(app: &mut App) {
    app.world_mut().run_schedule(FixedUpdate);
}

fn kill(app: &mut App, mob: Entity) {
    app.world_mut().get_mut::<Vitals>(mob).unwrap().stat = MobStat::Dead;
}

fn told(app: &App, mob: Entity, id: &str) -> bool {
    app.world()
        .get::<Outbox>(mob)
        .unwrap()
        .0
        .iter()
        .any(|m| m.id == id)
}

/// A captain, and a leader out of a clown.
fn started(mode: &'static str) -> (App, Entity, Entity) {
    let mut app = setup(mode);
    let captain = crew(&mut app, "captain");
    let clown = crew(&mut app, "clown");
    assert!(setup_round(app.world_mut(), mode == "rp-revolution"));
    assert!(app.world().get::<HeadRev>(clown).is_some());
    (app, captain, clown)
}

#[test]
fn heads_and_security_never_lead() {
    let mut app = setup("revolution");
    let barred: Vec<Entity> = HEADS.iter().map(|job| crew(&mut app, job)).collect();
    let officer = crew(&mut app, "security-officer");
    let others: Vec<Entity> = (0..5).map(|_| crew(&mut app, "clown")).collect();
    assert!(setup_round(app.world_mut(), false));
    for mob in barred.iter().chain([&officer]) {
        assert!(app.world().get::<HeadRev>(*mob).is_none());
    }
    let leading = others
        .iter()
        .filter(|m| app.world().get::<HeadRev>(**m).is_some())
        .count();
    assert_eq!(leading, 3);
}

#[test]
fn with_no_heads_or_no_one_to_lead_there_is_no_revolution() {
    let mut app = setup("revolution");
    crew(&mut app, "clown");
    assert!(!setup_round(app.world_mut(), false));
    let mut app = setup("revolution");
    crew(&mut app, "captain");
    assert!(!setup_round(app.world_mut(), false));
}

#[test]
fn a_leaders_flash_turns_those_it_can() {
    let (mut app, captain, leader) = started("revolution");
    let doctor = crew(&mut app, "medical-doctor");
    flashed(app.world_mut(), doctor, captain);
    assert!(
        app.world().get::<Revolutionary>(captain).is_none(),
        "not a leader"
    );
    flashed(app.world_mut(), leader, captain);
    assert!(
        app.world().get::<Revolutionary>(captain).is_none(),
        "a head can't be turned"
    );
    flashed(app.world_mut(), leader, doctor);
    assert!(app.world().get::<Revolutionary>(doctor).is_some());
    assert!(told(&app, doctor, "rev-joined"));
    assert_eq!(members(app.world_mut()).len(), 2);
}

#[test]
fn a_flash_does_nothing_for_the_revolution_in_rp() {
    let (mut app, _, leader) = started("rp-revolution");
    let doctor = crew(&mut app, "medical-doctor");
    flashed(app.world_mut(), leader, doctor);
    assert!(app.world().get::<Revolutionary>(doctor).is_none());
}

#[test]
fn leaders_can_only_be_turned_back_if_they_were_turned() {
    let (mut app, _, leader) = started("revolution");
    let doctor = crew(&mut app, "medical-doctor");
    add_revolutionary(app.world_mut(), doctor);
    assert!(!remove_revolutionary(app.world_mut(), leader));
    assert!(remove_revolutionary(app.world_mut(), doctor));
    assert!(app.world().get::<Revolutionary>(doctor).is_none());
    assert!(app.world().resource::<RevState>().converted.is_empty());
}

#[test]
fn rp_asks_and_takes_the_answer() {
    let (mut app, _, leader) = started("rp-revolution");
    let doctor = crew(&mut app, "medical-doctor");
    ask(app.world_mut(), leader, doctor);
    assert_eq!(app.world().get::<RevAsk>(doctor).unwrap().from, Some(leader));
    // Too soon to be asked again.
    answer(app.world_mut(), doctor, false);
    assert!(app.world().get::<Revolutionary>(doctor).is_none());
    assert!(told(&app, leader, "rev-refused"));
    ask(app.world_mut(), leader, doctor);
    assert_eq!(app.world().get::<RevAsk>(doctor).unwrap().from, None);
    assert!(told(&app, leader, "rev-wait"));
    // Later on they say yes.
    for _ in 0..200 {
        tick(&mut app);
    }
    ask(app.world_mut(), leader, doctor);
    answer(app.world_mut(), doctor, true);
    assert!(app.world().get::<Revolutionary>(doctor).is_some());
    assert!(told(&app, leader, "rev-accepted"));
}

#[test]
fn no_one_far_away_or_on_the_wrong_side_can_be_asked() {
    let (mut app, captain, leader) = started("rp-revolution");
    ask(app.world_mut(), leader, captain);
    assert!(told(&app, leader, "rev-cannot"));
    let doctor = crew(&mut app, "medical-doctor");
    app.world_mut()
        .entity_mut(doctor)
        .insert(GridPos(TilePos::new(9, 9, 1)));
    ask(app.world_mut(), leader, doctor);
    assert!(app.world().get::<RevAsk>(doctor).is_none());
    // And not by someone who isn't in on it.
    let other = crew(&mut app, "clown");
    let bystander = crew(&mut app, "clown");
    app.world_mut().entity_mut(other).remove::<HeadRev>();
    ask(app.world_mut(), other, bystander);
    assert!(app.world().get::<RevAsk>(bystander).is_none());
}

#[test]
fn the_revolution_wins_when_the_heads_are_dead() {
    let (mut app, captain, _) = started("revolution");
    tick(&mut app);
    assert!(!over(app.world()));
    kill(&mut app, captain);
    tick(&mut app);
    assert_eq!(app.world().resource::<RevState>().revs_won, Some(true));
    assert!(told(&app, captain, "rev-win"));
}

#[test]
fn the_heads_win_when_the_leaders_are_dead() {
    let (mut app, captain, leader) = started("revolution");
    kill(&mut app, leader);
    tick(&mut app);
    assert_eq!(app.world().resource::<RevState>().revs_won, Some(false));
    assert!(told(&app, captain, "rev-lose"));
}

#[test]
fn in_rp_cuffs_are_enough_for_either_side() {
    let (mut app, captain, _) = started("rp-revolution");
    let cuffs = spawn_object(app.world_mut(), &ObjectSpec::new("handcuffs"), AT).unwrap();
    app.world_mut().entity_mut(captain).insert(Cuffed(cuffs));
    tick(&mut app);
    assert_eq!(app.world().resource::<RevState>().revs_won, Some(true));

    let (mut app, _, leader) = started("rp-revolution");
    let cuffs = spawn_object(app.world_mut(), &ObjectSpec::new("handcuffs"), AT).unwrap();
    app.world_mut().entity_mut(leader).insert(Cuffed(cuffs));
    tick(&mut app);
    assert_eq!(app.world().resource::<RevState>().revs_won, Some(false));
}

#[test]
fn cuffs_arent_enough_in_a_plain_revolution() {
    let (mut app, captain, _) = started("revolution");
    let cuffs = spawn_object(app.world_mut(), &ObjectSpec::new("handcuffs"), AT).unwrap();
    app.world_mut().entity_mut(captain).insert(Cuffed(cuffs));
    tick(&mut app);
    assert!(!over(app.world()));
}

#[test]
fn an_undecided_round_still_says_who_was_who() {
    let (mut app, _, _) = started("revolution");
    let lines = report(app.world_mut());
    assert!(lines.iter().any(|l| l.id == "rev-roll-leaders"));
    assert!(lines.iter().any(|l| l.id == "rev-roll-heads"));
}
