//! Igniters and ignition switches. Spec: docs/reference/igniters.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::atmos::StationAtmos;
use rs13_classic::fires::Fire;
use rs13_classic::igniters::{Igniter, IgnitionSwitch};
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_classic::power::{AreaPower, AreaState};
use rs13_simulation::atmos::gas::T20C;
use rs13_simulation::atmos::{Gas, GasMix};
use rs13_world::{Appearance, TileMap, TilePos};

/// A closed 3 × 3 burn chamber with an igniter in the middle, and a room west of it where
/// the switch is. Outside any area, so everything has power.
const LEGEND: &str = r#"
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'x': (tile: (id: "floor"), objects: [(id: "ignition-switch", vars: {"id": "burn"})]),
        'i': (tile: (id: "floor"), objects: [(id: "igniter", vars: {"id": "burn", "on": "0"})]),
    },
    rows: [
        "WWWWWWWW",
        "W.xW...W",
        "W..W.i.W",
        "W..W...W",
        "WWWWWWWW",
    ],
"#;

/// The same, all in one area, whose power is cut.
const UNPOWERED_AREA: &str = r#"
    area_legend: { 'a': "lab" },
    area_rows: [
        "aaaaaaaa",
        "aaaaaaaa",
        "aaaaaaaa",
        "aaaaaaaa",
        "aaaaaaaa",
    ],
"#;

const CHAMBER: TilePos = TilePos::new(5, 2, 0);
/// Next to the switch.
const BY_SWITCH: TilePos = TilePos::new(1, 3, 0);

fn setup(plasma: f64, powered: bool) -> App {
    let map = format!("({LEGEND}{})", if powered { "" } else { UNPOWERED_AREA });
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(&map, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    spawn_player_mob(app.world_mut(), BY_SWITCH);
    if !powered {
        app.world_mut()
            .resource_mut::<AreaPower>()
            .0
            .insert("lab".to_owned(), AreaState::default());
    }
    // A machine cycle, so power is worked out.
    seconds(&mut app, 2);
    if plasma > 0.0 {
        let mut gas = GasMix::default();
        gas.temperature = T20C;
        gas.set(Gas::Plasma, plasma);
        app.world_mut()
            .resource_mut::<StationAtmos>()
            .0
            .add_at(CHAMBER, &gas);
    }
    app
}

fn seconds(app: &mut App, s: u64) {
    for _ in 0..s * 20 {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn one<C: Component>(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<C>>()
        .single(app.world())
        .unwrap()
}

fn igniter_on(app: &mut App) -> bool {
    let e = one::<Igniter>(app);
    app.world().get::<Igniter>(e).unwrap().on
}

fn look<C: Component>(app: &mut App) -> String {
    let e = one::<C>(app);
    app.world().get::<Appearance>(e).unwrap().state.clone()
}

fn fires(app: &mut App) -> usize {
    app.world_mut().query::<&Fire>().iter(app.world()).count()
}

/// The most tiles on fire at once over the next `s` seconds: the plasma burns out.
fn most_fires(app: &mut App, s: u64) -> usize {
    (0..s)
        .map(|_| {
            seconds(app, 1);
            fires(app)
        })
        .max()
        .unwrap_or(0)
}

/// The player standing by the switch presses it.
fn press_switch(app: &mut App) {
    let mob = one::<PendingActions>(app);
    let switch = one::<IgnitionSwitch>(app);
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(switch));
    app.world_mut().run_schedule(FixedUpdate);
}

#[test]
fn the_switch_turns_the_igniter_on_and_the_chamber_burns() {
    let mut app = setup(40.0, true);
    assert!(!igniter_on(&mut app));
    assert_eq!(look::<Igniter>(&mut app), "igniter0");
    seconds(&mut app, 10);
    assert_eq!(fires(&mut app), 0, "nothing burns while the igniter is off");

    press_switch(&mut app);
    assert!(igniter_on(&mut app));
    assert_eq!(look::<IgnitionSwitch>(&mut app), "launcheract");
    assert_eq!(look::<Igniter>(&mut app), "igniter1");

    seconds(&mut app, 6);
    assert_eq!(
        look::<IgnitionSwitch>(&mut app),
        "launcherbtt",
        "the switch is back up after five seconds"
    );
    assert_eq!(most_fires(&mut app, 60), 9, "fire on every tile of the chamber");
}

#[test]
fn a_pressed_switch_ignores_presses_until_it_is_back_up() {
    let mut app = setup(0.0, true);
    press_switch(&mut app);
    press_switch(&mut app);
    assert!(igniter_on(&mut app), "the second press did nothing");
    seconds(&mut app, 6);
    press_switch(&mut app);
    assert!(!igniter_on(&mut app), "back up, it flips the igniter off again");
}

#[test]
fn without_power_the_switch_does_nothing_and_the_igniter_does_not_spark() {
    let mut app = setup(40.0, false);
    press_switch(&mut app);
    assert!(!igniter_on(&mut app));
    assert_eq!(look::<IgnitionSwitch>(&mut app), "launcherbtt");

    // Turned on by hand, an igniter without power still doesn't spark.
    let igniter = one::<Igniter>(&mut app);
    app.world_mut().get_mut::<Igniter>(igniter).unwrap().on = true;
    seconds(&mut app, 60);
    assert_eq!(fires(&mut app), 0);
    assert_eq!(look::<Igniter>(&mut app), "igniter0");
}
