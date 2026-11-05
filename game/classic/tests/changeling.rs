//! Changelings. Spec: docs/reference/changeling.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::Outbox;
use rs13_classic::changeling::{
    Changeling, Deafened, Husked, LingOp, LingViewer, POWERS, command, hive_text, make_changeling, report,
    setup_round,
};
use rs13_classic::chemistry::Reagents;
use rs13_classic::combat::{Muted, Paralyzed};
use rs13_classic::flashes::Weakened;
use rs13_classic::health::{Body, Vitals};
use rs13_classic::jobs::HasJob;
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_world::{DisplayName, GridPos, HumanLook, MobStat, TileMap, TilePos};

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
    ],
)"#;

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().run_schedule(FixedUpdate);
    app
}

fn person(app: &mut App, name: &str, x: i32) -> Entity {
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(x, 2, 0));
    app.world_mut().entity_mut(mob).insert((
        HasJob("clown".to_owned()),
        DisplayName(name.to_owned()),
        HumanLook::default(),
    ));
    mob
}

fn line(id: &str) -> u8 {
    POWERS.iter().position(|p| p.id == id).unwrap() as u8
}

fn ling_with(app: &mut App, chems: f32, points: i32) -> Entity {
    let mob = person(app, "Ling", 2);
    make_changeling(app.world_mut(), mob);
    let mut ling = app.world_mut().get_mut::<Changeling>(mob).unwrap();
    ling.chems = chems;
    ling.points = points;
    mob
}

fn use_it(app: &mut App, mob: Entity, id: &str, target: Option<Entity>, dna: Option<u8>) {
    command(
        app.world_mut(),
        mob,
        LingOp::Use {
            power: line(id),
            target,
            dna,
        },
    );
}

fn buy(app: &mut App, mob: Entity, id: &str) {
    command(app.world_mut(), mob, LingOp::Buy(line(id)));
}

fn ling(app: &App, mob: Entity) -> Changeling {
    app.world().get::<Changeling>(mob).unwrap().clone()
}

fn tick(app: &mut App, n: u32) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
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
fn a_new_changeling_has_lunas_starting_stock_and_only_free_powers() {
    let mut app = setup();
    let mob = person(&mut app, "Ling", 2);
    make_changeling(app.world_mut(), mob);
    let new = ling(&app, mob);
    assert_eq!(new.chems, 20.0);
    assert_eq!(new.storage, 50.0);
    assert_eq!(new.points, 5);
    assert_eq!(new.dna.len(), 1);
    assert!(new.has("absorb") && new.has("stasis"));
    assert!(!new.has("death-sting"));
}

#[test]
fn powers_cost_points_and_only_once() {
    let mut app = setup();
    let mob = ling_with(&mut app, 20.0, 5);
    buy(&mut app, mob, "death-sting");
    assert!(!ling(&app, mob).has("death-sting"), "ten points");
    buy(&mut app, mob, "chem-synth");
    assert!(ling(&app, mob).has("chem-synth"));
    assert_eq!(ling(&app, mob).points, 1);
    assert_eq!(ling(&app, mob).rate, 1.0);
    buy(&mut app, mob, "chem-synth");
    assert_eq!(ling(&app, mob).points, 1, "already had it");
}

#[test]
fn chemicals_recover_up_to_the_limit() {
    let mut app = setup();
    let mob = ling_with(&mut app, 49.9, 0);
    tick(&mut app, 200);
    assert_eq!(ling(&app, mob).chems, 50.0);
}

#[test]
fn absorbing_needs_a_grab() {
    let mut app = setup();
    let mob = ling_with(&mut app, 20.0, 0);
    use_it(&mut app, mob, "absorb", None, None);
    assert!(told(&app, mob, "ling-need-grab"));
    assert!(ling(&app, mob).absorbing.is_none());
}

#[test]
fn transforming_takes_the_genome_and_hurts_the_genes() {
    let mut app = setup();
    let mob = ling_with(&mut app, 20.0, 0);
    let target = person(&mut app, "Victim", 3);
    let female = HumanLook {
        female: true,
        ..HumanLook::default()
    };
    app.world_mut().entity_mut(target).insert(female);
    // Extract their DNA without harm.
    app.world_mut()
        .get_mut::<Changeling>(mob)
        .unwrap()
        .powers
        .push("extract-dna");
    app.world_mut().get_mut::<Changeling>(mob).unwrap().chems = 50.0;
    use_it(&mut app, mob, "extract-dna", Some(target), None);
    assert_eq!(ling(&app, mob).dna.len(), 2);
    use_it(&mut app, mob, "transform", None, Some(1));
    assert_eq!(app.world().get::<DisplayName>(mob).unwrap().0, "Victim");
    assert!(app.world().get::<HumanLook>(mob).unwrap().female);
    assert_eq!(ling(&app, mob).genetic_damage, 30.0);
    // Not again until they've recovered.
    use_it(&mut app, mob, "transform", None, Some(0));
    assert_eq!(app.world().get::<DisplayName>(mob).unwrap().0, "Victim");
    assert!(told(&app, mob, "ling-genes"));
}

#[test]
fn stasis_brings_back_the_dead() {
    let mut app = setup();
    let mob = ling_with(&mut app, 50.0, 0);
    app.world_mut().get_mut::<Body>(mob).unwrap().tox = 500.0;
    tick(&mut app, 2);
    assert_eq!(app.world().get::<Vitals>(mob).unwrap().stat, MobStat::Dead);
    assert!(
        app.world().get::<rs13_classic::ghosts::LeftBody>(mob).is_none(),
        "stays in the body"
    );
    use_it(&mut app, mob, "stasis", None, None);
    assert!(ling(&app, mob).stasis_until.is_some());
    let wait = ling(&app, mob).stasis_until.unwrap();
    while app
        .world()
        .resource::<rs13_classic::time::WorldTime>()
        .deciseconds()
        < wait + 1.0
    {
        tick(&mut app, 10);
    }
    tick(&mut app, 2);
    assert_eq!(app.world().get::<Vitals>(mob).unwrap().stat, MobStat::Conscious);
    assert!(told(&app, mob, "ling-regenerated"));
}

#[test]
fn stings_reach_one_tile_unless_boosted() {
    let mut app = setup();
    let mob = ling_with(&mut app, 50.0, 0);
    app.world_mut()
        .get_mut::<Changeling>(mob)
        .unwrap()
        .powers
        .extend(["paralysis-sting", "boost-range"]);
    let target = person(&mut app, "Victim", 4);
    use_it(&mut app, mob, "paralysis-sting", Some(target), None);
    assert!(app.world().get::<Weakened>(target).is_none());
    assert_eq!(ling(&app, mob).chems, 50.0, "no chemicals spent");
    use_it(&mut app, mob, "boost-range", None, None);
    use_it(&mut app, mob, "paralysis-sting", Some(target), None);
    assert!(app.world().get::<Weakened>(target).is_some());
    assert_eq!(ling(&app, mob).sting_range, 1, "just for one");
}

#[test]
fn stings_do_what_they_say_but_not_to_another_changeling() {
    let mut app = setup();
    let mob = ling_with(&mut app, 50.0, 0);
    let powers = ["deaf-sting", "silence-sting", "death-sting"];
    app.world_mut()
        .get_mut::<Changeling>(mob)
        .unwrap()
        .powers
        .extend(powers);
    let target = person(&mut app, "Victim", 3);
    use_it(&mut app, mob, "deaf-sting", Some(target), None);
    assert!(app.world().get::<Deafened>(target).is_some());
    use_it(&mut app, mob, "silence-sting", Some(target), None);
    assert!(app.world().get::<Muted>(target).is_some());
    app.world_mut().get_mut::<Changeling>(mob).unwrap().chems = 50.0;
    use_it(&mut app, mob, "death-sting", Some(target), None);
    assert!(app.world().get::<Paralyzed>(target).is_some());
    assert!(
        app.world()
            .get::<Reagents>(target)
            .unwrap()
            .amounts
            .contains_key("lexorin")
    );

    let other = person(&mut app, "Other", 1);
    make_changeling(app.world_mut(), other);
    app.world_mut().get_mut::<Changeling>(mob).unwrap().chems = 50.0;
    use_it(&mut app, mob, "deaf-sting", Some(other), None);
    assert!(app.world().get::<Deafened>(other).is_none());
    assert!(told(&app, other, "ling-prick"));
}

#[test]
fn the_deaf_hear_nobody_for_a_while() {
    let mut app = setup();
    let mob = ling_with(&mut app, 50.0, 0);
    app.world_mut()
        .get_mut::<Changeling>(mob)
        .unwrap()
        .powers
        .push("deaf-sting");
    let target = person(&mut app, "Victim", 3);
    use_it(&mut app, mob, "deaf-sting", Some(target), None);
    app.world_mut()
        .get_mut::<rs13_classic::speech::PendingSpeech>(mob)
        .unwrap()
        .0
        .push("hello".to_owned());
    tick(&mut app, 1);
    assert!(!told(&app, target, "say"));
}

#[test]
fn the_hive_channel_reaches_only_changelings() {
    assert_eq!(hive_text(":g hello"), Some(" hello"));
    assert_eq!(hive_text("hello"), None);
    let mut app = setup();
    let mob = ling_with(&mut app, 50.0, 0);
    let other = person(&mut app, "Other", 3);
    let another = person(&mut app, "Another", 4);
    make_changeling(app.world_mut(), another);
    app.world_mut()
        .get_mut::<rs13_classic::speech::PendingSpeech>(mob)
        .unwrap()
        .0
        .push(":g hello".to_owned());
    tick(&mut app, 1);
    assert!(told(&app, another, "ling-hive-say"));
    assert!(!told(&app, other, "ling-hive-say"));
    assert!(!told(&app, other, "say"), "not said aloud");
}

#[test]
fn a_mimicked_voice_is_what_others_hear() {
    let mut app = setup();
    let mob = ling_with(&mut app, 50.0, 0);
    app.world_mut()
        .get_mut::<Changeling>(mob)
        .unwrap()
        .powers
        .push("mimic-voice");
    let other = person(&mut app, "Other", 3);
    command(app.world_mut(), mob, LingOp::Mimic("The Captain".to_owned()));
    app.world_mut()
        .get_mut::<rs13_classic::speech::PendingSpeech>(mob)
        .unwrap()
        .0
        .push("hello".to_owned());
    tick(&mut app, 1);
    let said = app
        .world()
        .get::<Outbox>(other)
        .unwrap()
        .0
        .iter()
        .find(|m| m.id == "say")
        .cloned()
        .unwrap();
    assert!(format!("{said:?}").contains("The Captain"), "{said:?}");
}

#[test]
fn the_window_only_opens_for_a_changeling() {
    let mut app = setup();
    let mob = ling_with(&mut app, 20.0, 0);
    let other = person(&mut app, "Other", 3);
    command(app.world_mut(), other, LingOp::Open);
    assert!(app.world().get::<LingViewer>(other).is_none());
    command(app.world_mut(), mob, LingOp::Open);
    tick(&mut app, 1);
    let view = app
        .world()
        .get::<rs13_classic::changeling::LingPanelOut>(mob)
        .unwrap()
        .0
        .clone()
        .unwrap();
    assert_eq!(view.chems, 20);
    assert_eq!(view.powers.len(), POWERS.len());
    assert_eq!(view.nearby.len(), 1);
    command(app.world_mut(), mob, LingOp::Open);
    tick(&mut app, 1);
    assert!(
        app.world()
            .get::<rs13_classic::changeling::LingPanelOut>(mob)
            .unwrap()
            .0
            .is_none()
    );
}

#[test]
fn a_round_makes_one_and_briefs_them() {
    let mut app = setup();
    let mobs: Vec<Entity> = (0..5)
        .map(|i| person(&mut app, &format!("Crew {i}"), i))
        .collect();
    assert!(setup_round(app.world_mut()));
    let lings: Vec<Entity> = mobs
        .iter()
        .copied()
        .filter(|m| app.world().get::<Changeling>(*m).is_some())
        .collect();
    assert_eq!(lings.len(), 1);
    assert!(told(&app, lings[0], "ling-you"));
    assert!(ling(&app, lings[0]).objectives.len() >= 2);
    let lines = report(app.world_mut());
    assert!(lines.iter().any(|l| l.id == "ling-was"));
    assert!(lines.iter().any(|l| l.id == "ling-lost"));
}

#[test]
fn husks_cant_be_absorbed_or_changed() {
    let mut app = setup();
    let mob = ling_with(&mut app, 50.0, 0);
    let target = person(&mut app, "Husk", 3);
    app.world_mut().entity_mut(target).insert(Husked);
    app.world_mut()
        .get_mut::<Changeling>(mob)
        .unwrap()
        .powers
        .push("transformation-sting");
    use_it(&mut app, mob, "transformation-sting", Some(target), Some(0));
    assert!(told(&app, mob, "ling-sting-useless"));
}

fn until(app: &mut App, ds: f32) {
    let at = app
        .world()
        .resource::<rs13_classic::time::WorldTime>()
        .deciseconds()
        + ds;
    while app
        .world()
        .resource::<rs13_classic::time::WorldTime>()
        .deciseconds()
        < at
    {
        tick(app, 1);
    }
}

#[test]
fn absorbing_takes_three_stages_and_leaves_a_husk() {
    let mut app = setup();
    let mob = ling_with(&mut app, 20.0, 0);
    let victim = person(&mut app, "Victim", 3);
    rs13_classic::grabs::start(app.world_mut(), mob, victim);
    tick(&mut app, 1);
    use_it(&mut app, mob, "absorb", None, None);
    assert!(
        ling(&app, mob).absorbing.is_some(),
        "{:?}",
        app.world().get::<Outbox>(mob).unwrap().0
    );
    until(&mut app, 160.0);
    assert!(told(&app, mob, "ling-absorb-2"));
    until(&mut app, 160.0);
    assert!(told(&app, mob, "ling-absorb-3"));
    until(&mut app, 160.0);
    let done = ling(&app, mob);
    assert_eq!(done.absorbed, 1);
    assert_eq!(done.points, 2);
    assert_eq!(done.dna.len(), 2);
    assert!(app.world().get::<Husked>(victim).is_some());
    assert_eq!(app.world().get::<Vitals>(victim).unwrap().stat, MobStat::Dead);
}

#[test]
fn moving_interrupts_absorbing() {
    let mut app = setup();
    let mob = ling_with(&mut app, 20.0, 0);
    let victim = person(&mut app, "Victim", 3);
    rs13_classic::grabs::start(app.world_mut(), mob, victim);
    tick(&mut app, 1);
    use_it(&mut app, mob, "absorb", None, None);
    app.world_mut()
        .entity_mut(mob)
        .insert(GridPos(TilePos::new(5, 5, 0)));
    tick(&mut app, 2);
    assert!(ling(&app, mob).absorbing.is_none());
    assert!(told(&app, mob, "ling-interrupted"));
    assert!(app.world().get::<Husked>(victim).is_none());
}
