//! The windows of canisters, portable pumps and scrubbers, and the gas tank they hold.
//! Spec: docs/reference/portables.md, docs/reference/gas-tanks.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, PendingActions};
use rs13_classic::atmos::StationAtmos;
use rs13_classic::equipment::TankGas;
use rs13_classic::gas_panel::{GasOp, GasPanelOut, HeldTank, PendingGasOps, RELEASE_MAX, RELEASE_MIN};
use rs13_classic::hands::Hands;
use rs13_classic::machines::{Canister, GasStore, MACHINE_TICKS, Switch};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::portables::{PUMP_VOLUME, PortablePump};
use rs13_simulation::atmos::gas::ONE_ATMOSPHERE;
use rs13_world::{GasMachine, GridPos, ObjectSpec, TileMap, TilePos};

/// A closed room with an oxygen canister and a portable pump.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'o': (tile: (id: "floor"), objects: ["oxygen-canister"]),
        'p': (tile: (id: "floor"), objects: ["portable-pump"]),
    },
    rows: [
        "WWWWWWW",
        "W.o.p.W",
        "W.....W",
        "WWWWWWW",
    ],
)"#;

const BESIDE: TilePos = TilePos::new(3, 1, 0);

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().run_schedule(FixedUpdate);
    app
}

fn cycles(app: &mut App, n: u64) {
    for _ in 0..n * MACHINE_TICKS {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn one<C: Component>(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<C>>()
        .single(app.world())
        .unwrap()
}

fn click(app: &mut App, mob: Entity, target: Entity) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    app.world_mut().run_schedule(FixedUpdate);
}

fn press(app: &mut App, mob: Entity, target: Entity, op: GasOp) {
    app.world_mut()
        .get_mut::<PendingGasOps>(mob)
        .unwrap()
        .0
        .push((target, op));
    app.world_mut().run_schedule(FixedUpdate);
}

fn window(app: &App, mob: Entity) -> Option<rs13_world::GasPanelView> {
    app.world().get::<GasPanelOut>(mob).unwrap().0.clone()
}

#[test]
fn clicking_a_canister_opens_its_window_and_the_buttons_work() {
    let mut app = setup();
    let canister = one::<Canister>(&mut app);
    let mob = spawn_player_mob(app.world_mut(), BESIDE);
    click(&mut app, mob, canister);
    app.world_mut().run_schedule(FixedUpdate);
    let view = window(&app, mob).expect("the window opened");
    assert_eq!(view.kind, GasMachine::Canister);
    assert!(!view.on && !view.connected && view.tank_dkpa.is_none());
    assert!(view.pressure_dkpa > 40_000, "full: {}", view.pressure_dkpa);
    assert_eq!(view.setting, ONE_ATMOSPHERE.round() as u32);

    press(&mut app, mob, canister, GasOp::Toggle);
    assert_eq!(app.world().get::<Switch>(canister), Some(&Switch(true)));
    // The release pressure stays between a tenth of an atmosphere and ten.
    press(&mut app, mob, canister, GasOp::Adjust(5000.0));
    assert_eq!(
        app.world().get::<Canister>(canister).unwrap().release_target,
        RELEASE_MAX
    );
    press(&mut app, mob, canister, GasOp::Adjust(-5000.0));
    assert_eq!(
        app.world().get::<Canister>(canister).unwrap().release_target,
        RELEASE_MIN
    );

    // Walking away closes it.
    app.world_mut()
        .entity_mut(mob)
        .insert(GridPos(TilePos::new(5, 2, 0)));
    app.world_mut().run_schedule(FixedUpdate);
    assert!(window(&app, mob).is_none());
}

#[test]
fn a_canister_fills_a_tank_put_in_it_up_to_its_release_pressure() {
    let mut app = setup();
    let canister = one::<Canister>(&mut app);
    let mob = spawn_player_mob(app.world_mut(), BESIDE);
    let tank = spawn_object(app.world_mut(), &ObjectSpec::new("oxygen-tank"), BESIDE).unwrap();
    app.world_mut().get_mut::<TankGas>(tank).unwrap().mix = rs13_simulation::atmos::GasMix::default();
    click(&mut app, mob, tank);
    click(&mut app, mob, canister);
    assert_eq!(app.world().get::<HeldTank>(canister), Some(&HeldTank(tank)));
    assert_eq!(app.world().get::<Hands>(mob).unwrap().active_item(), None);
    let room = app.world().resource::<StationAtmos>().0.total_moles();

    // 500 kPa into the tank, not the room.
    click(&mut app, mob, canister);
    press(&mut app, mob, canister, GasOp::Adjust(400.0));
    press(&mut app, mob, canister, GasOp::Toggle);
    cycles(&mut app, 20);
    let tank_gas = app.world().get::<TankGas>(tank).unwrap();
    let kpa = tank_gas.mix.pressure(tank_gas.volume);
    assert!((kpa - 501.0).abs() < 5.0, "tank at {kpa:.1} kPa");
    let after = app.world().resource::<StationAtmos>().0.total_moles();
    assert!((after - room).abs() < 1.0, "the room got nothing");
    assert!(window(&app, mob).unwrap().tank_dkpa.is_some());

    // Out it comes, onto the floor.
    press(&mut app, mob, canister, GasOp::RemoveTank);
    app.world_mut().run_schedule(FixedUpdate);
    assert!(app.world().get::<HeldTank>(canister).is_none());
    assert_eq!(app.world().get::<GridPos>(tank).unwrap().0, TilePos::new(2, 2, 0));
}

#[test]
fn a_pump_turned_out_releases_its_tank_into_the_room() {
    let mut app = setup();
    let pump = one::<PortablePump>(&mut app);
    let mob = spawn_player_mob(app.world_mut(), BESIDE);
    click(&mut app, mob, pump);
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(window(&app, mob).unwrap().kind, GasMachine::Pump);
    // Siphon first, up to 500 kPa in its tank.
    press(&mut app, mob, pump, GasOp::Adjust(400.0));
    press(&mut app, mob, pump, GasOp::Toggle);
    cycles(&mut app, 10);
    let full = app.world().get::<GasStore>(pump).unwrap().0;
    assert!((full.pressure(PUMP_VOLUME) - 500.0).abs() < 10.0);

    // Then out, toward 110 kPa in the room.
    press(&mut app, mob, pump, GasOp::Direction);
    assert!(app.world().get::<PortablePump>(pump).unwrap().out);
    press(&mut app, mob, pump, GasOp::Adjust(-390.0));
    let before = app
        .world()
        .resource::<StationAtmos>()
        .0
        .pressure_at(TilePos::new(4, 2, 0));
    cycles(&mut app, 10);
    let after = app
        .world()
        .resource::<StationAtmos>()
        .0
        .pressure_at(TilePos::new(4, 2, 0));
    assert!(after > before, "the room filled: {before:.1} -> {after:.1}");
    assert!(window(&app, mob).unwrap().out);
}
