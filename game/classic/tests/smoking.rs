//! Cigarettes, matches and lighters. Spec: docs/reference/smoking.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::atmos::StationAtmos;
use rs13_classic::environment::LIFE_TICKS;
use rs13_classic::hands::Hands;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::smoking::{CIGARETTE_CYCLES, Cigarette, CigarettePacket, Lighter, Match};
use rs13_classic::storage::Storage;
use rs13_simulation::atmos::gas::T20C;
use rs13_simulation::atmos::{Gas, GasMix};
use rs13_world::{Appearance, GridPos, Hand, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
    },
    rows: [
        "WWWWW",
        "W...W",
        "W...W",
        "WWWWW",
    ],
)"#;

const HERE: TilePos = TilePos::new(1, 1, 0);

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().run_schedule(FixedUpdate);
    app
}

fn act(app: &mut App, mob: Entity, action: Action) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
    app.world_mut().run_schedule(FixedUpdate);
}

fn pick_up(app: &mut App, mob: Entity, proto: &str) -> Entity {
    let item = spawn_object(app.world_mut(), &ObjectSpec::new(proto), HERE).unwrap();
    act(app, mob, Action::UseOn(item));
    item
}

fn cycles(app: &mut App, n: u64) {
    for _ in 0..n * LIFE_TICKS {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

#[test]
fn a_cigarette_comes_out_of_its_packet_into_the_empty_hand() {
    let mut app = setup();
    let mob = spawn_player_mob(app.world_mut(), HERE);
    let packet = pick_up(&mut app, mob, "cigarette-packet");
    act(&mut app, mob, Action::SwapHands);
    act(&mut app, mob, Action::UseOn(packet));
    app.world_mut().run_schedule(FixedUpdate);
    let cig = app
        .world()
        .get::<Hands>(mob)
        .unwrap()
        .active_item()
        .expect("a cigarette");
    assert!(app.world().get::<Cigarette>(cig).is_some());
    assert_eq!(app.world().get::<CigarettePacket>(packet).unwrap().0, 5);
    assert_eq!(app.world().get::<Appearance>(packet).unwrap().state, "5");
}

#[test]
fn a_struck_match_lights_a_cigarette_which_burns_down_to_a_butt() {
    let mut app = setup();
    let mob = spawn_player_mob(app.world_mut(), HERE);
    let cig = pick_up(&mut app, mob, "cigarette");
    act(&mut app, mob, Action::SwapHands);
    let matchbox = spawn_object(app.world_mut(), &ObjectSpec::new("matchbox"), HERE).unwrap();
    let first = app.world().get::<Storage>(matchbox).unwrap().items[0];
    assert_eq!(app.world().get::<Storage>(matchbox).unwrap().items.len(), 7);
    // Take a match out of the box and strike it on the box.
    app.world_mut()
        .entity_mut(first)
        .remove::<rs13_classic::storage::InStorage>()
        .insert(GridPos(HERE));
    app.world_mut()
        .get_mut::<Storage>(matchbox)
        .unwrap()
        .items
        .retain(|e| *e != first);
    act(&mut app, mob, Action::UseOn(first));
    act(&mut app, mob, Action::UseOn(matchbox));
    assert!(matches!(app.world().get::<Match>(first), Some(Match::Lit(_))));
    assert_eq!(
        app.world().get::<Storage>(matchbox).unwrap().items.len(),
        6,
        "not put back"
    );

    // The lit match lights the cigarette in the other hand.
    act(&mut app, mob, Action::UseOn(cig));
    assert!(app.world().get::<Cigarette>(cig).unwrap().lit.is_some());
    assert_eq!(app.world().get::<Appearance>(cig).unwrap().state, "lit");

    // It burns down, and a butt is left.
    cycles(&mut app, u64::from(CIGARETTE_CYCLES) + 1);
    assert!(app.world().get_entity(cig).is_err(), "burnt out");
    assert!(app.world().get::<Hands>(mob).unwrap().get(Hand::Left) != Some(cig));
    let mut q = app.world_mut().query::<&Appearance>();
    assert!(q.iter(app.world()).any(|a| a.sprite == "cigarette-butt"));
}

#[test]
fn a_lit_lighter_sets_plasma_alight() {
    let mut app = setup();
    let mut plasma = GasMix::default();
    plasma.temperature = T20C;
    plasma.set(Gas::Plasma, 20.0);
    app.world_mut()
        .resource_mut::<StationAtmos>()
        .0
        .add_at(HERE, &plasma);
    let mob = spawn_player_mob(app.world_mut(), HERE);
    let lighter = pick_up(&mut app, mob, "lighter");
    // A cheap lighter lights, sometimes burning a finger.
    act(&mut app, mob, Action::UseOn(lighter));
    assert!(app.world().get::<Lighter>(lighter).unwrap().lit);
    assert!(
        app.world()
            .get::<Appearance>(lighter)
            .unwrap()
            .state
            .ends_with("-on")
    );
    cycles(&mut app, 2);
    assert!(
        !app.world()
            .resource::<StationAtmos>()
            .0
            .burning_tiles()
            .is_empty(),
        "700 K is enough for plasma"
    );
    act(&mut app, mob, Action::UseOn(lighter));
    assert!(!app.world().get::<Lighter>(lighter).unwrap().lit);
}
