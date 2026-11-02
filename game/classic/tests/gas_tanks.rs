//! Gas pressure tanks and canisters of one gas. Spec: docs/reference/gas-tanks.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::atmos::StationAtmos;
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::machines::{GAS_TANK_PRESSURE, GAS_TANKS, GasStore, PipeMember, PipeNets, canister_light};
use rs13_classic::objects::{spawn_map, spawn_player_mob};
use rs13_simulation::atmos::Gas;
use rs13_simulation::atmos::gas::ONE_ATMOSPHERE;
use rs13_world::{Appearance, GridPos, TileMap, TilePos};

/// A room with one pressure tank of each gas, an oxygen canister on a connector port, a
/// loose nitrogen canister and a wrench.
const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'o': (tile: (id: "floor"), objects: [(id: "oxygen-pressure-tank", dir: East)]),
        'n': (tile: (id: "floor"), objects: [(id: "nitrogen-pressure-tank", dir: East)]),
        'c': (tile: (id: "floor"), objects: [(id: "carbon-dioxide-pressure-tank", dir: East)]),
        'p': (tile: (id: "floor"), objects: [(id: "plasma-pressure-tank", dir: East)]),
        'k': (tile: (id: "floor"), objects: ["connector", "oxygen-canister"]),
        'N': (tile: (id: "floor"), objects: ["nitrogen-canister"]),
        'w': (tile: (id: "floor"), objects: ["wrench"]),
    },
    rows: [
        "WWWWWWWWW",
        "Wo.n.c.pW",
        "W.......W",
        "Wk..w..NW",
        "W.......W",
        "WWWWWWWWW",
    ],
)"#;

const PORT: TilePos = TilePos::new(1, 2, 0);
const LOOSE: TilePos = TilePos::new(7, 2, 0);
const WRENCH: TilePos = TilePos::new(4, 2, 0);

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    ticks(&mut app, 5);
    app
}

fn ticks(app: &mut App, n: usize) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn find(app: &mut App, sprite: &str) -> (Entity, TilePos) {
    let mut q = app.world_mut().query::<(Entity, &Appearance, &GridPos)>();
    q.iter(app.world())
        .find(|(_, a, _)| a.sprite == sprite)
        .map(|(e, _, p)| (e, p.0))
        .unwrap_or_else(|| panic!("no {sprite}"))
}

fn act(app: &mut App, mob: Entity, action: Action) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
    app.world_mut().run_schedule(FixedUpdate);
}

fn look(app: &App, entity: Entity) -> String {
    app.world().get::<Appearance>(entity).unwrap().state.clone()
}

#[test]
fn each_pressure_tank_holds_its_own_gas_at_seventy_atmospheres() {
    let mut app = setup();
    for (id, gas) in GAS_TANKS {
        let (tank, pos) = find(&mut app, id);
        let nets = app.world().resource::<PipeNets>();
        let (mix, pressure) = match nets.at(pos) {
            Some(net) => (net.mix, net.pressure()),
            None => {
                let store = app.world().get::<GasStore>(tank).unwrap().0;
                let volume = app.world().get::<PipeMember>(tank).unwrap().volume;
                (store, store.pressure(volume))
            }
        };
        assert!(
            (pressure - GAS_TANK_PRESSURE).abs() < 1.0,
            "{id} at {pressure:.1} kPa"
        );
        assert!(
            (mix.get(gas) - mix.total_moles()).abs() < 1e-6,
            "{id} holds only its gas"
        );
    }
}

#[test]
fn a_loose_canister_releases_only_its_own_gas() {
    let mut app = setup();
    let (canister, pos) = find(&mut app, "nitrogen-canister");
    assert_eq!(pos, LOOSE);
    assert_eq!(look(&app, canister), "+can-o3");

    // Let half the room's air out (7 by 4 tiles), so the canister has something to fill.
    {
        let mut atmos = app.world_mut().resource_mut::<StationAtmos>();
        let half = atmos.0.air_at(LOOSE).total_moles() * 14.0;
        atmos.0.remove_at(LOOSE, half);
    }
    let before = app.world().resource::<StationAtmos>().0.air_at(LOOSE);

    let mob = spawn_player_mob(app.world_mut(), LOOSE.step(rs13_world::Dir::South));
    // Open its release valve from its window.
    app.world_mut()
        .get_mut::<rs13_classic::gas_panel::PendingGasOps>(mob)
        .unwrap()
        .0
        .push((canister, rs13_classic::gas_panel::GasOp::Toggle));
    ticks(&mut app, 4 * rs13_classic::machines::MACHINE_TICKS as usize);

    let after = app.world().resource::<StationAtmos>().0.air_at(LOOSE);
    assert!(
        after.get(Gas::Nitrogen) > before.get(Gas::Nitrogen) + 1.0,
        "nitrogen came out: {before:?} -> {after:?}"
    );
    // The mob breathes some oxygen; none comes out of the canister.
    assert!(
        after.get(Gas::Oxygen) <= before.get(Gas::Oxygen) + 1e-9,
        "no oxygen came out"
    );
    let room = app.world().resource::<StationAtmos>().0.pressure_at(LOOSE);
    assert!(room <= ONE_ATMOSPHERE + 0.5, "it stops at one atmosphere: {room}");
}

#[test]
fn a_canister_shows_its_gauge_and_the_port_it_is_on() {
    let mut app = setup();
    let (canister, pos) = find(&mut app, "oxygen-canister");
    assert_eq!(pos, PORT);
    assert_eq!(look(&app, canister), "+can-o3+can-connector");

    // Wrench it off the port.
    let mob = spawn_player_mob(app.world_mut(), PORT.step(rs13_world::Dir::South));
    let (wrench, at) = find(&mut app, "wrench");
    assert_eq!(at, WRENCH);
    app.world_mut()
        .entity_mut(wrench)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    app.world_mut().get_mut::<Hands>(mob).unwrap().right = Some(wrench);
    act(&mut app, mob, Action::UseOn(canister));
    let said = app.world().get::<Outbox>(mob).unwrap().0.last().cloned().unwrap();
    assert_eq!(said.id, "canister-disconnected");
    assert!(format!("{said:?}").contains("oxygen-canister"), "{said:?}");
    ticks(&mut app, 1);
    assert_eq!(look(&app, canister), "+can-o3");
}

#[test]
fn the_gauge_light_follows_luna() {
    assert_eq!(canister_light(0.0), 0);
    assert_eq!(canister_light(50.0), 1);
    assert_eq!(canister_light(ONE_ATMOSPHERE), 2);
    assert_eq!(canister_light(15.0 * ONE_ATMOSPHERE), 3);
}
