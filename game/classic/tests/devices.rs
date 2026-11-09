//! Devices, assemblies, tank bombs and the transfer valve. Spec: docs/reference/devices.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::atmos::StationAtmos;
use rs13_classic::devices::{self, Assembly, IgniterDevice, PartOf, ProxSensor, Signaler, Timer, Valve};
use rs13_classic::equipment::TankGas;
use rs13_classic::explosions::PendingBlasts;
use rs13_classic::filing::PendingPicks;
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::machines::MACHINE_TICKS;
use rs13_classic::movement::MoveIntent;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_simulation::atmos::{Gas, GasMix};
use rs13_world::{Appearance, Dir, GridPos, Hand, HumanLook, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
    },
    rows: [
        "WWWWWWWWWWWW",
        "W..........W",
        "W..........W",
        "W..........W",
        "W..........W",
        "W..........W",
        "WWWWWWWWWWWW",
    ],
)"#;

fn at(x: i32, y: i32) -> TilePos {
    TilePos::new(x, y, 0)
}

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    tick(&mut app, 1);
    app
}

fn tick(app: &mut App, n: u32) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn cycles(app: &mut App, n: u32) {
    tick(app, n * MACHINE_TICKS as u32);
}

fn person(app: &mut App, pos: TilePos) -> Entity {
    let mob = spawn_player_mob(app.world_mut(), pos);
    app.world_mut().entity_mut(mob).insert(HumanLook::default());
    mob
}

fn act(app: &mut App, mob: Entity, action: Action) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
    tick(app, 1);
}

fn pick(app: &mut App, mob: Entity, target: Entity, line: usize) {
    app.world_mut()
        .get_mut::<PendingPicks>(mob)
        .unwrap()
        .0
        .push((target, line));
    tick(app, 1);
}

fn spawn(app: &mut App, id: &str, pos: TilePos) -> Entity {
    spawn_object(app.world_mut(), &ObjectSpec::new(id), pos).unwrap()
}

/// Put an item in one of a mob's hands.
fn give(app: &mut App, mob: Entity, hand: Hand, item: Entity) {
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    *app.world_mut().get_mut::<Hands>(mob).unwrap().slot_mut(hand) = Some(item);
}

fn active_hand(app: &App, mob: Entity) -> Hand {
    app.world().get::<Hands>(mob).unwrap().active
}

fn other(hand: Hand) -> Hand {
    if hand == Hand::Left {
        Hand::Right
    } else {
        Hand::Left
    }
}

/// Use what's in the active hand on something.
fn use_on(app: &mut App, mob: Entity, tool: Entity, target: Entity) {
    let hand = active_hand(app, mob);
    give(app, mob, hand, tool);
    act(app, mob, Action::UseOn(target));
}

/// An open signaler on an attachable igniter: a radio-igniter assembly, secured.
fn radio_igniter(app: &mut App, mob: Entity) -> Entity {
    let hand = active_hand(app, mob);
    let signaler = spawn(app, devices::SIGNALER, at(1, 1));
    let igniter = spawn(app, devices::IGNITER, at(1, 1));
    let screwdriver = spawn(app, "screwdriver", at(1, 1));
    give(app, mob, other(hand), signaler);
    use_on(app, mob, screwdriver, signaler);
    give(app, mob, other(hand), igniter);
    use_on(app, mob, screwdriver, igniter);
    assert!(!app.world().get::<IgniterDevice>(igniter).unwrap().ready);
    give(app, mob, other(hand), igniter);
    use_on(app, mob, signaler, igniter);
    let assembly = app.world().get::<Hands>(mob).unwrap().get(hand).unwrap();
    assert_eq!(
        app.world().get::<Appearance>(assembly).unwrap().sprite,
        "radio-igniter"
    );
    give(app, mob, other(hand), assembly);
    use_on(app, mob, screwdriver, assembly);
    assert!(app.world().get::<Assembly>(assembly).unwrap().secured);
    assert!(app.world().get::<IgniterDevice>(igniter).unwrap().ready);
    *app.world_mut().get_mut::<Hands>(mob).unwrap().slot_mut(hand) = None;
    app.world_mut().entity_mut(screwdriver).despawn();
    assembly
}

fn add_gas(app: &mut App, tile: TilePos, gas: Gas, moles: f64) {
    let mut mix = GasMix::default();
    mix.set(gas, moles);
    mix.temperature = 293.15;
    app.world_mut()
        .resource_mut::<StationAtmos>()
        .0
        .add_at(tile, &mix);
}

fn burning(app: &App, tile: TilePos) -> bool {
    app.world().resource::<StationAtmos>().0.burning_at(tile)
}

#[test]
fn a_signal_reaches_only_the_same_frequency_and_code() {
    let mut app = setup();
    let mob = person(&mut app, at(2, 2));
    let sender = spawn(&mut app, devices::SIGNALER, at(1, 1));
    give(&mut app, mob, Hand::Left, sender);
    let listening = spawn(&mut app, devices::VALVE, at(9, 4));
    let deaf = spawn(&mut app, devices::VALVE, at(9, 2));
    for (valve, code) in [(listening, 30), (deaf, 31)] {
        let s = spawn(&mut app, devices::SIGNALER, at(1, 1));
        app.world_mut().get_mut::<Signaler>(s).unwrap().code = code;
        app.world_mut()
            .entity_mut(s)
            .remove::<GridPos>()
            .insert(PartOf(valve));
        app.world_mut().get_mut::<Valve>(valve).unwrap().device = Some(s);
    }
    app.world_mut().get_mut::<Hands>(mob).unwrap().active = Hand::Left;
    act(&mut app, mob, Action::UseOn(sender));
    // Send.
    pick(&mut app, mob, sender, 0);
    assert!(app.world().get::<Valve>(listening).unwrap().open);
    assert!(!app.world().get::<Valve>(deaf).unwrap().open);
}

#[test]
fn a_signaler_window_changes_frequency_by_odd_steps() {
    let mut app = setup();
    let mob = person(&mut app, at(2, 2));
    let s = spawn(&mut app, devices::SIGNALER, at(1, 1));
    let hand = active_hand(&app, mob);
    give(&mut app, mob, hand, s);
    act(&mut app, mob, Action::UseOn(s));
    // Send, frequency, - -, -, +, + +, code, - -, -, +, + +.
    pick(&mut app, mob, s, 5);
    pick(&mut app, mob, s, 5);
    pick(&mut app, mob, s, 5);
    pick(&mut app, mob, s, 4);
    pick(&mut app, mob, s, 10);
    let sig = *app.world().get::<Signaler>(s).unwrap();
    assert_eq!(sig.frequency, 1489, "capped at 148.9");
    assert_eq!(sig.code, 35);
}

#[test]
fn a_secured_timer_igniter_lights_plasma_when_it_runs_out() {
    let mut app = setup();
    let mob = person(&mut app, at(2, 2));
    // Luna's mapped kind, timer and igniter already inside.
    let assembly = spawn(&mut app, devices::TIMER_IGNITER, at(6, 3));
    let a = *app.world().get::<Assembly>(assembly).unwrap();
    assert!(app.world().get::<Timer>(a.trigger).is_some());
    let screwdriver = spawn(&mut app, "screwdriver", at(1, 1));
    app.world_mut().entity_mut(mob).insert(GridPos(at(5, 3)));
    use_on(&mut app, mob, screwdriver, assembly);
    assert!(app.world().get::<Assembly>(assembly).unwrap().secured);
    add_gas(&mut app, at(6, 3), Gas::Plasma, 30.0);
    // Start it at 0: it goes off on the next cycle.
    let hand = active_hand(&app, mob);
    *app.world_mut().get_mut::<Hands>(mob).unwrap().slot_mut(hand) = Some(screwdriver);
    app.world_mut().get_mut::<Timer>(a.trigger).unwrap().timing = true;
    assert!(!burning(&app, at(6, 3)));
    cycles(&mut app, 2);
    assert!(burning(&app, at(6, 3)));
    assert!(!app.world().get::<Timer>(a.trigger).unwrap().timing);
}

#[test]
fn an_unsecured_assembly_comes_apart_with_a_wrench() {
    let mut app = setup();
    let mob = person(&mut app, at(2, 2));
    let assembly = spawn(&mut app, devices::TIMER_IGNITER, at(3, 2));
    let a = *app.world().get::<Assembly>(assembly).unwrap();
    let wrench = spawn(&mut app, "wrench", at(1, 1));
    use_on(&mut app, mob, wrench, assembly);
    assert!(app.world().get_entity(assembly).is_err());
    for part in [a.trigger, a.other] {
        assert_eq!(app.world().get::<GridPos>(part).unwrap().0, at(3, 2));
        assert!(app.world().get::<PartOf>(part).is_none());
    }
}

#[test]
fn a_radio_bomb_unwelded_lets_its_plasma_out() {
    let mut app = setup();
    let mob = person(&mut app, at(2, 2));
    let assembly = radio_igniter(&mut app, mob);
    let tank = spawn(&mut app, devices::PLASMA_TANK, at(1, 1));
    let h = other(active_hand(&app, mob));
    give(&mut app, mob, h, tank);
    use_on(&mut app, mob, assembly, tank);
    let bomb = app.world().get::<Assembly>(assembly).unwrap();
    assert_eq!(bomb.tank, Some(tank));
    assert_eq!(
        app.world().get::<Appearance>(assembly).unwrap().sprite,
        "radio-igniter-tank"
    );
    // Down on the floor, then signalled.
    let hand = active_hand(&app, mob);
    *app.world_mut().get_mut::<Hands>(mob).unwrap().slot_mut(hand) = None;
    app.world_mut()
        .entity_mut(assembly)
        .remove::<HeldBy>()
        .insert(GridPos(at(6, 4)));
    let sender = spawn(&mut app, devices::SIGNALER, at(1, 1));
    give(&mut app, mob, hand, sender);
    act(&mut app, mob, Action::UseOn(sender));
    pick(&mut app, mob, sender, 0);
    assert!(app.world().get::<TankGas>(tank).unwrap().mix.is_vacuum());
    assert!(
        app.world().get_entity(assembly).is_ok(),
        "no bang without the hole"
    );
}

#[test]
fn a_welded_bomb_with_hot_plasma_blows_up() {
    let mut app = setup();
    let mob = person(&mut app, at(2, 2));
    let assembly = spawn(&mut app, devices::TIMER_IGNITER, at(1, 1));
    let screwdriver = spawn(&mut app, "screwdriver", at(1, 1));
    let h = other(active_hand(&app, mob));
    give(&mut app, mob, h, assembly);
    use_on(&mut app, mob, screwdriver, assembly);
    let tank = spawn(&mut app, devices::PLASMA_TANK, at(1, 1));
    let h = other(active_hand(&app, mob));
    give(&mut app, mob, h, tank);
    use_on(&mut app, mob, assembly, tank);
    let welder = spawn(&mut app, "welder", at(1, 1));
    let h = other(active_hand(&app, mob));
    give(&mut app, mob, h, assembly);
    use_on(&mut app, mob, welder, assembly);
    assert!(app.world().get::<Assembly>(assembly).unwrap().welded);
    // Over 400 °C.
    app.world_mut().get_mut::<TankGas>(tank).unwrap().mix.temperature = 800.0;
    let timer = app.world().get::<Assembly>(assembly).unwrap().trigger;
    // Nine times in ten it goes; give it a few tries.
    for _ in 0..6 {
        if app.world().get_entity(assembly).is_err() {
            break;
        }
        app.world_mut().get_mut::<Timer>(timer).unwrap().timing = true;
        cycles(&mut app, 2);
    }
    assert!(app.world().get_entity(assembly).is_err());
    assert!(app.world().get_entity(tank).is_err());
    assert!(app.world().get::<Hands>(mob).unwrap().left != Some(assembly));
    let _ = app.world().resource::<PendingBlasts>();
}

#[test]
fn the_transfer_valve_pours_the_first_tank_into_the_second() {
    let mut app = setup();
    let mob = person(&mut app, at(2, 2));
    let valve = spawn(&mut app, devices::VALVE, at(1, 1));
    let hand = active_hand(&app, mob);
    let one = spawn(&mut app, devices::PLASMA_TANK, at(1, 1));
    let two = spawn(&mut app, "oxygen-tank", at(1, 1));
    for tank in [one, two] {
        give(&mut app, mob, other(hand), valve);
        use_on(&mut app, mob, tank, valve);
    }
    let v = *app.world().get::<Valve>(valve).unwrap();
    assert_eq!(v.tanks, [Some(one), Some(two)]);
    let (moles_one, volume_two) = {
        let w = app.world();
        (
            w.get::<TankGas>(one).unwrap().mix.get(Gas::Plasma),
            w.get::<TankGas>(two).unwrap().volume,
        )
    };
    // The valve in the active hand, used: its window; the fourth line opens it.
    give(&mut app, mob, hand, valve);
    *app.world_mut()
        .get_mut::<Hands>(mob)
        .unwrap()
        .slot_mut(other(hand)) = None;
    act(&mut app, mob, Action::UseOn(valve));
    pick(&mut app, mob, valve, 3);
    assert!(app.world().get::<Valve>(valve).unwrap().open);
    let w = app.world();
    assert!(w.get::<TankGas>(one).unwrap().mix.is_vacuum());
    let two_gas = w.get::<TankGas>(two).unwrap();
    assert!((two_gas.mix.get(Gas::Plasma) - moles_one).abs() < 1e-6);
    assert!(two_gas.volume > volume_two);
}

#[test]
fn an_armed_sensor_feels_a_step_beside_it() {
    let mut app = setup();
    let valve = spawn(&mut app, devices::VALVE, at(6, 3));
    let sensor = spawn(&mut app, devices::PROX, at(1, 1));
    app.world_mut().get_mut::<ProxSensor>(sensor).unwrap().armed = true;
    app.world_mut()
        .entity_mut(sensor)
        .remove::<GridPos>()
        .insert(PartOf(valve));
    app.world_mut().get_mut::<Valve>(valve).unwrap().device = Some(sensor);
    let mob = person(&mut app, at(4, 3));
    tick(&mut app, 2);
    assert!(!app.world().get::<Valve>(valve).unwrap().open, "too far");
    app.world_mut().get_mut::<MoveIntent>(mob).unwrap().dir = Some(Dir::East);
    tick(&mut app, 10);
    app.world_mut().get_mut::<MoveIntent>(mob).unwrap().dir = None;
    assert!(app.world().get::<Valve>(valve).unwrap().open);
}
