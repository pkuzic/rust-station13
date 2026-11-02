//! The station's shield generator. Spec: docs/reference/shields.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::atmos::StationAtmos;
use rs13_classic::machines::MACHINE_TICKS;
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_classic::shields::{
    CHARGE_PER_CYCLE, ConverterMode, MANUAL_RATE, MAX_CHARGE, START_CHARGE, ShieldCapacitor, ShieldConverter,
    ShieldNetwork, ShieldTile, ShieldsButton,
};
use rs13_world::{Appearance, Dense, GridPos, Hidden, TileMap, TilePos};

/// A room with a hole in its west side onto space, three capacitors with converters east of
/// them, a plate in the floor and the button. Far east, an airless closet out of the plate's
/// reach of space.
const MAP: &str = r#"(
    legend: {
        ' ': (tile: (id: "space")),
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'p': (tile: (id: "plating"), objects: ["shield-plate"]),
        'f': (tile: (id: "floor"), objects: ["shield-plate"]),
        'C': (tile: (id: "floor"), objects: ["shield-capacitor"]),
        'E': (tile: (id: "floor"), objects: ["shield-converter"]),
        'b': (tile: (id: "floor"), objects: ["shields-button"]),
    },
    rows: [
        "   WWWWWWWW",
        "   WCECECEW",
        "   ......bW",
        "   WpfW...W",
        "   WWWWWWWW",
    ],
)"#;

/// The hole in the hull, and the space just west of it.
const INSIDE: TilePos = TilePos::new(3, 2, 0);
const OUTSIDE: TilePos = TilePos::new(2, 2, 0);
/// The start of the room, by the button.
const STANDING: TilePos = TilePos::new(8, 2, 0);

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), STANDING);
    app.world_mut().run_schedule(FixedUpdate);
    (app, mob)
}

fn cycles(app: &mut App, n: u64) {
    for _ in 0..n * MACHINE_TICKS {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn press(app: &mut App, mob: Entity, target: Entity) -> Vec<String> {
    let before = app.world().get::<Outbox>(mob).unwrap().0.len();
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    app.world_mut().run_schedule(FixedUpdate);
    app.world().get::<Outbox>(mob).unwrap().0[before..]
        .iter()
        .map(|m| m.id.clone())
        .collect()
}

fn button(app: &mut App) -> Entity {
    let mut q = app.world_mut().query_filtered::<Entity, With<ShieldsButton>>();
    q.single(app.world()).unwrap()
}

fn field_tile(app: &mut App, at: TilePos) -> Entity {
    let mut q = app
        .world_mut()
        .query_filtered::<(Entity, &GridPos), With<ShieldTile>>();
    q.iter(app.world())
        .find(|(_, p)| p.0 == at)
        .expect("a field tile")
        .0
}

fn charges(app: &mut App) -> Vec<f64> {
    let mut q = app.world_mut().query::<&ShieldCapacitor>();
    q.iter(app.world()).map(|c| c.charge).collect()
}

fn pressure(app: &App, at: TilePos) -> f64 {
    app.world().resource::<StationAtmos>().0.pressure_at(at)
}

#[test]
fn the_field_lies_only_along_the_hull_and_stays_down_until_raised() {
    let (mut app, _) = setup();
    let tile = field_tile(&mut app, OUTSIDE);
    assert!(app.world().get::<Hidden>(tile).is_some());
    assert!(app.world().get::<Dense>(tile).is_none());
    let mut q = app.world_mut().query_filtered::<&GridPos, With<ShieldTile>>();
    assert!(q.iter(app.world()).all(|p| p.0.x == 2), "only next to the hull");
}

#[test]
fn a_plate_under_floor_tiles_is_hidden_and_on_plating_is_not() {
    let (mut app, _) = setup();
    let mut q = app.world_mut().query::<(&GridPos, &Appearance, Has<Hidden>)>();
    let mut plate = |x| {
        q.iter(app.world())
            .find(|(p, a, _)| a.sprite == "shield-plate" && p.0.x == x)
            .unwrap()
            .2
    };
    assert!(!plate(4), "on plating");
    assert!(plate(5), "under floor tiles");
}

#[test]
fn raising_the_shields_stops_people_and_seals_a_breach() {
    let (mut app, mob) = setup();
    let button = button(&mut app);
    assert_eq!(press(&mut app, mob, button), ["shields-raised"]);
    cycles(&mut app, 1);
    assert!(app.world().resource::<ShieldNetwork>().up);
    let tile = field_tile(&mut app, OUTSIDE);
    assert!(app.world().get::<Dense>(tile).is_some());
    assert!(app.world().get::<Hidden>(tile).is_none());
    let mut plates = app.world_mut().query::<&Appearance>();
    assert!(
        plates
            .iter(app.world())
            .any(|a| a.sprite == "shield-plate" && a.state == "plate")
    );

    let sealed = pressure(&app, INSIDE);
    for _ in 0..100 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    assert!(
        pressure(&app, INSIDE) > sealed * 0.99,
        "{sealed} then {}",
        pressure(&app, INSIDE)
    );

    assert_eq!(press(&mut app, mob, button), ["shields-lowered"]);
    cycles(&mut app, 1);
    let tile = field_tile(&mut app, OUTSIDE);
    assert!(app.world().get::<Dense>(tile).is_none());
    for _ in 0..200 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    assert!(pressure(&app, INSIDE) < sealed * 0.5, "the breach vents again");
}

#[test]
fn running_shields_drain_capacitors_until_they_fall() {
    let (mut app, mob) = setup();
    let button = button(&mut app);
    press(&mut app, mob, button);
    cycles(&mut app, 2);
    // Manual converters can't keep up with what the field takes.
    let gained = MANUAL_RATE.powf(2.15).round();
    let once = charges(&mut app)[0];
    assert!(once < START_CHARGE);
    cycles(&mut app, 1);
    let twice = charges(&mut app)[0];
    assert!(
        (once - twice - (CHARGE_PER_CYCLE - gained)).abs() < 1.0,
        "{once} {twice}"
    );

    cycles(&mut app, 20);
    assert!(
        !app.world().resource::<ShieldNetwork>().up,
        "out of charge: {:?}",
        charges(&mut app)
    );
    let tile = field_tile(&mut app, OUTSIDE);
    assert!(app.world().get::<Hidden>(tile).is_some());
}

#[test]
fn shields_wont_rise_on_two_capacitors() {
    let (mut app, mob) = setup();
    let mut q = app.world_mut().query::<&mut ShieldCapacitor>();
    q.iter_mut(app.world_mut()).next().unwrap().charge = 0.0;
    let mut q = app.world_mut().query::<&mut ShieldConverter>();
    for mut converter in q.iter_mut(app.world_mut()) {
        converter.mode = ConverterMode::Off;
    }
    let button = button(&mut app);
    assert_eq!(press(&mut app, mob, button), ["shields-no-charge"]);
    cycles(&mut app, 1);
    assert!(!app.world().resource::<ShieldNetwork>().up);
}

#[test]
fn automatic_converters_keep_the_shields_up_and_clicking_changes_mode() {
    let (mut app, mob) = setup();
    let mut q = app.world_mut().query_filtered::<Entity, With<ShieldConverter>>();
    let converters: Vec<Entity> = q.iter(app.world()).collect();
    let mut positions = Vec::new();
    for converter in converters {
        positions.push(app.world().get::<GridPos>(converter).unwrap().0);
        app.world_mut().get_mut::<GridPos>(mob).unwrap().0 = TilePos::new(positions.last().unwrap().x, 2, 0);
        assert_eq!(press(&mut app, mob, converter), ["shield-converter-mode"]);
        assert_eq!(
            app.world().get::<ShieldConverter>(converter).unwrap().mode,
            ConverterMode::Auto
        );
    }
    app.world_mut().get_mut::<GridPos>(mob).unwrap().0 = STANDING;
    let button = button(&mut app);
    press(&mut app, mob, button);
    cycles(&mut app, 40);
    assert!(app.world().resource::<ShieldNetwork>().up);
    let charge = charges(&mut app)[0];
    assert!(charge > START_CHARGE && charge <= MAX_CHARGE, "{charge}");
}
