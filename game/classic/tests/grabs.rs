//! Grabs. Spec: docs/reference/combat.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::combat::CombatIntent;
use rs13_classic::environment::Lungs;
use rs13_classic::equipment::{Equipment, WornBy};
use rs13_classic::flashes::Weakened;
use rs13_classic::grabs::{Grab, Hold};
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::movement::MoveIntent;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_world::{Dir, GridPos, Intent, ObjectSpec, Slot, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        't': (tile: (id: "floor"), objects: ["table"]),
    },
    rows: ["WWWWWWWWWW", "Wt.......W", "W........W", "WWWWWWWWWW"],
)"#;
const A: TilePos = TilePos::new(2, 1, 0);
const B: TilePos = TilePos::new(3, 1, 0);

fn setup() -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let attacker = spawn_player_mob(app.world_mut(), A);
    let victim = spawn_player_mob(app.world_mut(), B);
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().get_mut::<CombatIntent>(attacker).unwrap().0 = Intent::Grab;
    (app, attacker, victim)
}

fn click(app: &mut App, mob: Entity, target: Entity) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    app.world_mut().run_schedule(FixedUpdate);
}

fn ticks(app: &mut App, n: u32) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn the_grab(app: &mut App) -> Option<(Entity, Grab)> {
    let mut grabs = app.world_mut().query::<(Entity, &Grab)>();
    grabs.iter(app.world()).next().map(|(e, g)| (e, *g))
}

/// A hold tightened as far as this, trying again whenever the chance goes against it.
fn hold_to(app: &mut App, attacker: Entity, victim: Entity, hold: Hold) -> Entity {
    for _ in 0..50 {
        if the_grab(app).is_none() {
            click(app, attacker, victim);
        }
        let (grab, held) = the_grab(app).expect("just grabbed");
        if held.hold >= hold {
            return grab;
        }
        click(app, attacker, victim);
    }
    panic!("never got to {hold:?}");
}

fn at(app: &App, mob: Entity) -> TilePos {
    app.world().get::<GridPos>(mob).unwrap().0
}

#[test]
fn a_grab_sits_in_the_hand_and_dropping_it_lets_go() {
    let (mut app, attacker, victim) = setup();
    click(&mut app, attacker, victim);
    let (grab, held) = the_grab(&mut app).unwrap();
    assert_eq!(held.hold, Hold::Passive);
    assert_eq!(held.victim, victim);
    assert_eq!(
        app.world().get::<Hands>(attacker).unwrap().active_item(),
        Some(grab)
    );

    app.world_mut()
        .get_mut::<PendingActions>(attacker)
        .unwrap()
        .0
        .push(Action::Drop);
    ticks(&mut app, 2);
    assert!(the_grab(&mut app).is_none());
    assert!(app.world().get_entity(grab).is_err(), "not left on the floor");
    assert!(
        app.world()
            .get::<Hands>(attacker)
            .unwrap()
            .active_item()
            .is_none()
    );
}

#[test]
fn it_needs_an_empty_hand_and_not_yourself() {
    let (mut app, attacker, victim) = setup();
    click(&mut app, attacker, attacker);
    assert!(the_grab(&mut app).is_none());
    let tool = spawn_object(app.world_mut(), &ObjectSpec::new("crowbar"), A).unwrap();
    click(&mut app, attacker, tool);
    click(&mut app, attacker, victim);
    assert!(the_grab(&mut app).is_none(), "that's a blow with the crowbar");
}

#[test]
fn someone_walking_off_ends_a_passive_grab() {
    let (mut app, attacker, victim) = setup();
    click(&mut app, attacker, victim);
    app.world_mut().get_mut::<MoveIntent>(victim).unwrap().dir = Some(Dir::East);
    ticks(&mut app, 2);
    assert!(the_grab(&mut app).is_none());
    assert_eq!(at(&app, victim).x, B.x + 1, "and off they go");
}

#[test]
fn both_hands_make_them_drop_things_and_they_have_to_struggle() {
    let (mut app, attacker, victim) = setup();
    let held_item = spawn_object(app.world_mut(), &ObjectSpec::new("crowbar"), B).unwrap();
    click(&mut app, victim, held_item);
    assert!(app.world().get::<HeldBy>(held_item).is_some());
    hold_to(&mut app, attacker, victim, Hold::Aggressive);
    ticks(&mut app, 2);
    assert!(app.world().get::<HeldBy>(held_item).is_none());
    assert_eq!(at(&app, held_item), B);

    // Struggling keeps them put, until they break free.
    app.world_mut().get_mut::<MoveIntent>(victim).unwrap().dir = Some(Dir::East);
    let mut seconds = 0;
    while the_grab(&mut app).is_some() {
        assert_eq!(at(&app, victim), B, "held fast");
        ticks(&mut app, 20);
        seconds += 1;
        assert!(seconds < 200, "never broke free");
    }
}

#[test]
fn a_neck_hold_brings_them_onto_your_tile_unless_a_helmet_is_in_the_way() {
    let (mut app, attacker, victim) = setup();
    let helmet = spawn_object(app.world_mut(), &ObjectSpec::new("helmet"), B).unwrap();
    app.world_mut()
        .entity_mut(helmet)
        .remove::<GridPos>()
        .insert(WornBy(victim));
    app.world_mut()
        .get_mut::<Equipment>(victim)
        .unwrap()
        .0
        .insert(Slot::Head, helmet);
    let grab = hold_to(&mut app, attacker, victim, Hold::Aggressive);
    click(&mut app, attacker, victim);
    assert_eq!(
        app.world().get::<Grab>(grab).unwrap().hold,
        Hold::Aggressive,
        "it's in the way"
    );

    app.world_mut()
        .get_mut::<Equipment>(victim)
        .unwrap()
        .0
        .remove(&Slot::Head);
    click(&mut app, attacker, victim);
    assert_eq!(app.world().get::<Grab>(grab).unwrap().hold, Hold::Neck);
    ticks(&mut app, 1);
    assert_eq!(at(&app, victim), at(&app, attacker));
}

#[test]
fn choking_takes_their_breath_and_can_be_stopped() {
    let (mut app, attacker, victim) = setup();
    let grab = hold_to(&mut app, attacker, victim, Hold::Neck);
    click(&mut app, attacker, grab);
    assert!(app.world().get::<Grab>(grab).unwrap().choking);
    let mut breathless = false;
    let mut down = false;
    for _ in 0..600 {
        ticks(&mut app, 1);
        breathless |= app.world().get::<Lungs>(victim).unwrap().losebreath > 0;
        down |= app.world().get::<Weakened>(victim).is_some();
    }
    assert!(breathless);
    assert!(down, "45% each second");
    click(&mut app, attacker, grab);
    assert!(!app.world().get::<Grab>(grab).unwrap().choking);
}

#[test]
fn a_table_takes_them_and_the_grab_ends() {
    let (mut app, attacker, victim) = setup();
    click(&mut app, attacker, victim);
    let mut tables = app.world_mut().query::<(Entity, &rs13_world::Appearance)>();
    let table = tables
        .iter(app.world())
        .find(|(_, a)| a.sprite == "table")
        .unwrap()
        .0;
    click(&mut app, attacker, table);
    assert!(the_grab(&mut app).is_none());
    assert_eq!(at(&app, victim), TilePos::new(1, 2, 0));
    assert!(app.world().get::<Weakened>(victim).is_some());
}

#[test]
fn it_ends_when_you_go_down() {
    let (mut app, attacker, victim) = setup();
    click(&mut app, attacker, victim);
    app.world_mut().entity_mut(attacker).insert(Weakened(1000.0));
    ticks(&mut app, 2);
    assert!(the_grab(&mut app).is_none());
}
