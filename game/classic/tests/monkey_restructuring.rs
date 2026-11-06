//! The monkey and restructuring rounds. Spec: docs/reference/monkey-restructuring.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::combat::CombatIntent;
use rs13_classic::equipment::{Equipment, WornBy};
use rs13_classic::health::{Body, Zone};
use rs13_classic::monkey_round::{self, Monkeyized, how_many};
use rs13_classic::monkeys::MONKEY;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::restructuring::{self, Restructuring};
use rs13_classic::round::{Phase, Round};
use rs13_world::{Appearance, DisplayName, GridPos, HumanLook, Intent, ObjectSpec, Slot, TileMap, TilePos};

const MAP: &str = r#"(
    legend: { 'W': (tile: (id: "wall", solid: true)), '.': (tile: (id: "floor")) },
    rows: ["WWWWWWWW", "W......W", "W......W", "WWWWWWWW"],
)"#;

fn setup(mode: &'static str) -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().insert_resource(Round {
        phase: Phase::Playing { mode },
        forced_mode: None,
    });
    app
}

fn person(app: &mut App, x: i32, name: &str) -> Entity {
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(x, 1, 0));
    app.world_mut()
        .entity_mut(mob)
        .insert((HumanLook::default(), DisplayName(name.to_owned())));
    mob
}

fn tick(app: &mut App, n: u32) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn heard(app: &App, mob: Entity, id: &str) -> bool {
    app.world()
        .get::<Outbox>(mob)
        .unwrap()
        .0
        .iter()
        .any(|m| m.id == id)
}

#[test]
fn a_third_of_the_crew_turn_with_three_or_more_at_most_four() {
    assert_eq!(how_many(2), 0);
    assert_eq!(how_many(3), 1);
    assert_eq!(how_many(4), 2);
    assert_eq!(how_many(7), 3);
    assert_eq!(how_many(30), 4);
}

#[test]
fn in_a_monkey_round_some_crew_turn_after_five_seconds_and_drop_what_they_had() {
    let mut app = setup("monkey");
    let crew: Vec<Entity> = (1..=4)
        .map(|x| person(&mut app, x, &format!("Crew {x}")))
        .collect();
    monkey_round::setup_round(app.world_mut());
    tick(&mut app, 2);
    let world = app.world_mut();
    assert_eq!(world.query::<&Monkeyized>().iter(world).count(), 0, "not yet");
    tick(&mut app, 120);
    let turned: Vec<Entity> = crew
        .iter()
        .copied()
        .filter(|m| app.world().get::<Monkeyized>(*m).is_some())
        .collect();
    assert_eq!(turned.len(), 2);
    for mob in turned {
        let world = app.world();
        assert_eq!(world.get::<Appearance>(mob).unwrap().sprite, MONKEY);
        assert_eq!(world.get::<DisplayName>(mob).unwrap().0, MONKEY);
        assert!(world.get::<HumanLook>(mob).is_none());
        assert!(
            world.get::<Equipment>(mob).unwrap().0.is_empty(),
            "clothes fell off"
        );
        assert!(heard(&app, mob, "monkey-you-are"));
        assert!(
            app.world()
                .get::<Monkeyized>(mob)
                .unwrap()
                .was
                .starts_with("Crew ")
        );
    }
}

#[test]
fn no_one_turns_in_other_rounds() {
    let mut app = setup("extended");
    for x in 1..=4 {
        person(&mut app, x, "Crew");
    }
    tick(&mut app, 200);
    let world = app.world_mut();
    assert_eq!(world.query::<&Monkeyized>().iter(world).count(), 0);
}

fn bite_many(app: &mut App, monkey: Entity, victim: Entity) -> f32 {
    app.world_mut().get_mut::<CombatIntent>(monkey).unwrap().0 = Intent::Harm;
    for _ in 0..20 {
        app.world_mut()
            .get_mut::<PendingActions>(monkey)
            .unwrap()
            .0
            .push(Action::UseOn(victim));
        app.world_mut().run_schedule(FixedUpdate);
        app.world_mut().get_mut::<Outbox>(victim).unwrap().0.clear();
    }
    let body = app.world().get::<Body>(victim).unwrap();
    Zone::ALL.iter().map(|z| body.organ(*z).brute).sum()
}

#[test]
fn a_monkey_bites_for_a_little_and_a_heavy_suit_keeps_its_teeth_out() {
    let mut app = setup("monkey");
    let monkey = person(&mut app, 1, "Bobo");
    let victim = person(&mut app, 2, "Victim");
    monkey_round::monkeyize(app.world_mut(), monkey);
    tick(&mut app, 1);
    let bitten = bite_many(&mut app, monkey, victim);
    assert!(
        (20.0..=60.0).contains(&bitten),
        "1 to 3 a bite, every time: {bitten}"
    );

    let mut app = setup("monkey");
    let monkey = person(&mut app, 1, "Bobo");
    let victim = person(&mut app, 2, "Victim");
    let suit = spawn_object(
        app.world_mut(),
        &ObjectSpec::new("space-suit"),
        TilePos::new(2, 1, 0),
    )
    .unwrap();
    app.world_mut()
        .entity_mut(suit)
        .remove::<GridPos>()
        .insert(WornBy(victim));
    app.world_mut()
        .get_mut::<Equipment>(victim)
        .unwrap()
        .0
        .insert(Slot::Suit, suit);
    monkey_round::monkeyize(app.world_mut(), monkey);
    tick(&mut app, 1);
    assert_eq!(bite_many(&mut app, monkey, victim), 0.0);
}

#[test]
fn monkeys_win_alone_on_the_pods_and_lose_otherwise() {
    let mut app = setup("monkey");
    let monkey = person(&mut app, 1, "Bobo");
    person(&mut app, 2, "Human");
    monkey_round::setup_round(app.world_mut());
    monkey_round::monkeyize(app.world_mut(), monkey);
    // Nobody is in a pod on this map.
    let said = monkey_round::report(app.world_mut());
    assert_eq!(said.len(), 1);
    assert_eq!(said[0].id, "monkey-lost");
}

#[test]
fn head_office_names_someone_after_twelve_seconds() {
    let mut app = setup("restructuring");
    let a = person(&mut app, 1, "Alice");
    let b = person(&mut app, 2, "Bob");
    restructuring::setup_round(app.world_mut());
    tick(&mut app, 100);
    assert!(app.world().resource::<Restructuring>().target.is_none());
    tick(&mut app, 200);
    let target = app.world().resource::<Restructuring>().target.unwrap();
    assert!(target == a || target == b);
    for mob in [a, b] {
        assert!(heard(&app, mob, "restructuring-target"));
    }
    assert!(heard(&app, target, "restructuring-you"));
    assert_eq!(restructuring::announcement().len(), 2);
}
