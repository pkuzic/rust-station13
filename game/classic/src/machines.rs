//! Atmospherics machinery: pipe networks, pumps, valves, vents, scrubbers, tanks and
//! canisters. Spec and tuning: `docs/reference/pipes.md`.

use std::collections::{BTreeMap, HashMap, VecDeque};

use bevy_ecs::prelude::*;
use rs13_simulation::atmos::gas::{ONE_ATMOSPHERE, R, T20C, TILE_VOLUME};
use rs13_simulation::atmos::{Gas, GasMix};
use rs13_world::{Appearance, Dir, GridPos, TilePos};

use crate::atmos::StationAtmos;
use crate::power::PowerUser;
use crate::time::WorldTime;

/// Machines run every this many ticks (2 s), like SS13's machinery cycle.
pub const MACHINE_TICKS: u64 = 40;
/// Litres of one pipe segment (SS13 value).
pub const PIPE_VOLUME: f64 = 70.0;
/// Litres of a three-way manifold (Luna `pipe/manifold`).
pub const MANIFOLD_VOLUME: f64 = 105.0;
/// Litres of a large air storage tank.
pub const TANK_VOLUME: f64 = 10_000.0;
/// Starting pressure of an air storage tank, kPa (about 90 atm).
pub const TANK_PRESSURE: f64 = 9_000.0;
/// Litres of a portable canister.
pub const CANISTER_VOLUME: f64 = 1_000.0;
/// Starting pressure of an air canister, kPa.
pub const CANISTER_PRESSURE: f64 = 4_500.0;
/// Litres of one of Luna's gas pressure tanks (`pipe/tank`: 0.9 m by 0.9 m by 2 m).
pub const GAS_TANK_VOLUME: f64 = 2_620.0;
/// Starting pressure of a gas pressure tank, kPa (Luna: 2 × 35 atm).
pub const GAS_TANK_PRESSURE: f64 = 70.0 * ONE_ATMOSPHERE;
/// Luna's gas pressure tanks, and the gas each is filled with.
pub const GAS_TANKS: [(&str, Gas); 4] = [
    ("oxygen-pressure-tank", Gas::Oxygen),
    ("nitrogen-pressure-tank", Gas::Nitrogen),
    ("carbon-dioxide-pressure-tank", Gas::CarbonDioxide),
    ("plasma-pressure-tank", Gas::Plasma),
];
/// Luna's canisters of one gas, and the gas each is filled with. The plain `canister`
/// holds air.
pub const GAS_CANISTERS: [(&str, Gas); 5] = [
    ("oxygen-canister", Gas::Oxygen),
    ("nitrogen-canister", Gas::Nitrogen),
    ("carbon-dioxide-canister", Gas::CarbonDioxide),
    ("plasma-canister", Gas::Plasma),
    ("nitrous-oxide-canister", Gas::NitrousOxide),
];
/// Default output pressure of a gas pump, kPa.
pub const PUMP_TARGET: f64 = 300.0;
/// Litres of room air a scrubber processes per cycle (SS13 `volume_rate`).
pub const SCRUBBER_VOLUME_RATE: f64 = 120.0;
/// The share of its room a siphoning vent pumps out each cycle.
pub const SIPHON_SHARE: f64 = 0.3;
/// Default output pressure of a gas filter, kPa (Luna `target_pressure`).
pub const FILTER_TARGET: f64 = ONE_ATMOSPHERE;
/// Default litres an outlet injector lets out per cycle (Luna `volume_rate`).
pub const INJECTOR_VOLUME_RATE: f64 = 50.0;
/// Litres of gas a unary device holds as part of its network (Luna `air_contents`, 200 L):
/// generator inputs and thermal plates.
pub const DEVICE_VOLUME: f64 = 200.0;
/// Share of its own gas, and of the room's, a thermal plate evens out each cycle (Luna 0.25).
pub const PLATE_SHARE: f64 = 0.25;
/// With fewer moles than this around it, a thermal plate radiates instead (Luna).
pub const PLATE_MIN_MOLES: f64 = 10.0;
/// Heat capacity of what a radiating plate gives its heat to, J/K, and that sink's
/// temperature, K (Luna `RADIATION_CAPACITY` and 6.4 K).
pub const RADIATION_CAPACITY: f64 = 30_000.0;
pub const RADIATION_TEMPERATURE: f64 = 6.4;
/// Litres of an open pipe end (Luna `pipe/vent` volume), and of pipe gas it mixes with
/// the room each cycle (`mingle_with_turf(loc, 250)`).
pub const PIPE_VENT_VOLUME: f64 = 250.0;
/// Gases scrubbers remove.
pub const SCRUBBED: [Gas; 3] = [Gas::CarbonDioxide, Gas::Plasma, Gas::NitrousOxide];

/// Part of a pipe network: it holds `volume` litres of the network's gas while `open`.
/// It connects on the sides in `connects` (N 1, S 2, E 4, W 8, as Luna's
/// `initialize_directions`): to a member on the neighbouring tile that connects back, and
/// to a member on its own tile that shares a side.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct PipeMember {
    pub volume: f64,
    pub open: bool,
    pub connects: u8,
}

/// Every side: hand-drawn maps place pipes that join whatever is next to them.
pub const ALL_SIDES: u8 = 0b1111;

/// The side bit of a direction; diagonals give both of their sides.
pub fn side_bits(dir: Dir) -> u8 {
    let (dx, dy) = dir.offset();
    let vertical = match dy {
        1 => 1,
        -1 => 2,
        _ => 0,
    };
    let horizontal = match dx {
        1 => 4,
        -1 => 8,
        _ => 0,
    };
    vertical | horizontal
}

/// Which sides a placed piece connects on (Luna's pipe, manifold and device
/// `initialize_directions`). Without a direction it connects everywhere.
pub fn connections(proto: &str, dir: Option<Dir>) -> u8 {
    let Some(dir) = dir else { return ALL_SIDES };
    let bits = side_bits(dir);
    match proto {
        // A straight piece runs along its direction; a diagonal one is a bend.
        "pipe" | "valve" => match bits {
            1 | 2 => 1 | 2,
            4 | 8 => 4 | 8,
            bend => bend,
        },
        // A manifold joins every side but the one it faces.
        "manifold" => ALL_SIDES & !bits,
        // Vents, scrubbers, ports and tanks connect on the side they face.
        _ => bits,
    }
}

/// The sides a piece connects on, as its sprite state: "1-4" (north and east).
pub fn side_names(connects: u8) -> String {
    [1u8, 2, 4, 8]
        .into_iter()
        .filter(|b| connects & b != 0)
        .map(|b| b.to_string())
        .collect::<Vec<_>>()
        .join("-")
}

/// Gas a member holds while the networks are being rebuilt, or while it is cut off (a
/// closed valve, a disconnected canister).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct GasStore(pub GasMix);

/// On/off. For valves: open/closed. For canisters: the release valve.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Switch(pub bool);

/// Moves gas from the pipe behind it to the pipe in front, up to `target` kPa.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Pump {
    pub dir: Dir,
    pub target: f64,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Valve;

/// Lets the network under it out into the room, up to `target` kPa.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Vent {
    pub target: f64,
    /// Panic fill, set from the air alarm: refill the whole room at once.
    pub panic: bool,
    /// Pumping the room's air into the pipes instead, for an airlock console (Luna `purge`).
    pub siphon: bool,
}

/// Pulls CO2, plasma and N2O out of the room into the network under it.
#[derive(Component, Debug, Clone, Copy)]
pub struct Scrubber;

/// An open end of a pipe network: each cycle some of the network's gas evens out with
/// the room it opens into, or is lost to space (Luna `pipe/vent`).
#[derive(Component, Debug, Clone, Copy)]
pub struct PipeVent;

/// A gas filter: gas from the pipe behind it goes to the pipe ahead, up to `target` kPa
/// there, with the filtered gas taken out to the pipe on its side, a quarter turn
/// clockwise (Luna `trinary/filter`: node1 behind, node2 the side, node3 ahead).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct GasFilter {
    pub dir: Dir,
    /// What it takes out; `None` passes everything through.
    pub gas: Option<Gas>,
    pub target: f64,
}

/// A gas mixer: gas from the pipe behind it and the pipe on its side (a quarter turn
/// clockwise) goes to the pipe ahead, in the shares `node1` and `node2`, up to `target` kPa
/// there (Luna `trinary/mixer`: node1 behind, node2 the side, node3 ahead).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct GasMixer {
    pub dir: Dir,
    pub target: f64,
    pub node1: f64,
    pub node2: f64,
}

/// Luna's `filter_type`: 0 plasma, 1 oxygen, 2 nitrogen, 3 carbon dioxide, 4 N2O. Any other
/// value takes nothing out (Luna's "Plasma Toxins Filter" is set to 5, so it doesn't filter).
pub fn filter_gas(filter_type: i64) -> Option<Gas> {
    match filter_type {
        0 => Some(Gas::Plasma),
        1 => Some(Gas::Oxygen),
        2 => Some(Gas::Nitrogen),
        3 => Some(Gas::CarbonDioxide),
        4 => Some(Gas::NitrousOxide),
        _ => None,
    }
}

/// BYOND `turn(dir, -90)`: a quarter turn clockwise.
pub fn clockwise(dir: Dir) -> Dir {
    match dir {
        Dir::North => Dir::East,
        Dir::East => Dir::South,
        Dir::South => Dir::West,
        _ => Dir::North,
    }
}

/// An outlet injector: while on, lets `volume_rate` litres of its network's gas out into
/// the room each cycle (Luna `unary/outlet_injector`).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Injector {
    pub volume_rate: f64,
}

/// A thermo-electric generator's pipe connection (Luna `unary/generator_input`).
#[derive(Component, Debug, Clone, Copy)]
pub struct GeneratorInput;

/// Trades heat between its pipe network and the room it stands in, or radiates it away
/// into space (Luna `unary/thermal_plate`).
#[derive(Component, Debug, Clone, Copy)]
pub struct ThermalPlate;

/// A port a canister can be wrenched onto.
#[derive(Component, Debug, Clone, Copy)]
pub struct Connector;

/// A portable gas canister. Released into the room up to `release_target` kPa while its
/// valve (`Switch`) is open and it is not connected to a port.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Canister {
    pub release_target: f64,
}

/// Something wheeled about that a wrench fastens onto the connector port under it:
/// canisters, portable pumps and scrubbers.
#[derive(Component, Debug, Clone, Copy)]
pub struct Portable;

/// One connected pipe network.
#[derive(Debug, Clone, PartialEq)]
pub struct PipeNet {
    pub members: Vec<Entity>,
    pub volume: f64,
    pub mix: GasMix,
}

impl PipeNet {
    /// A network of devices alone (no pipe) has no volume and no pressure.
    pub fn pressure(&self) -> f64 {
        if self.volume > 0.0 {
            self.mix.pressure(self.volume)
        } else {
            0.0
        }
    }
}

/// All pipe networks, and which member belongs to which.
#[derive(Resource, Debug, Default)]
pub struct PipeNets {
    pub nets: Vec<PipeNet>,
    /// The network of each open member.
    pub net_of: HashMap<Entity, usize>,
    /// The network of the first member on each tile (pipes can cross on one tile).
    pub net_at: HashMap<TilePos, usize>,
    /// Open members on each tile, with the sides they connect on.
    pub on_tile: HashMap<TilePos, Vec<(Entity, u8)>>,
    /// Set when the shape of the networks changes (valves, canisters, new pipes).
    pub dirty: bool,
}

impl PipeNets {
    pub fn at(&self, pos: TilePos) -> Option<&PipeNet> {
        self.net_at.get(&pos).map(|i| &self.nets[*i])
    }

    /// The network of the member on `pos` that connects on `side` (a side bit): what a
    /// pump next to it reaches.
    pub fn net_toward(&self, pos: TilePos, side: u8) -> Option<usize> {
        self.on_tile
            .get(&pos)?
            .iter()
            .find(|(_, connects)| connects & side != 0)
            .and_then(|(e, _)| self.net_of.get(e).copied())
    }

    /// Pressure a gauge on `entity` shows, kPa: its network, or its own store if cut off.
    pub fn gauge(&self, entity: Entity, store: Option<&GasStore>, volume: f64) -> f64 {
        match self.nets.iter().find(|n| n.members.contains(&entity)) {
            Some(net) => net.pressure(),
            None => store.map_or(0.0, |s| s.0.pressure(volume)),
        }
    }
}

/// Air at `pressure` kPa and 20 °C in `volume` litres.
pub fn air_at_pressure(volume: f64, pressure: f64) -> GasMix {
    GasMix::standard_air(volume).scaled(pressure / ONE_ATMOSPHERE)
}

/// One gas alone at `pressure` kPa and 20 °C in `volume` litres.
pub fn gas_at_pressure(gas: Gas, volume: f64, pressure: f64) -> GasMix {
    let mut mix = GasMix::default();
    mix.temperature = T20C;
    mix.set(gas, pressure * volume / (R * T20C));
    mix
}

/// Is this sprite a pipe tank (air or one gas)? Tanks are pipe pieces meters sit on.
pub fn is_pipe_tank(sprite: &str) -> bool {
    sprite == "air-tank" || GAS_TANKS.iter().any(|(id, _)| *id == sprite)
}

/// Is this sprite a canister (air or one gas)?
pub fn is_canister(sprite: &str) -> bool {
    sprite == "canister" || sprite == "empty-canister" || GAS_CANISTERS.iter().any(|(id, _)| *id == sprite)
}

/// Luna's canister gauge light: `can-o0` below 10 kPa, `can-o1` below one atmosphere,
/// `can-o2` below 15 atm, `can-o3` above.
pub fn canister_light(kpa: f64) -> u8 {
    if kpa < 10.0 {
        0
    } else if kpa < ONE_ATMOSPHERE {
        1
    } else if kpa < 15.0 * ONE_ATMOSPHERE {
        2
    } else {
        3
    }
}

/// SS13's pump formula: moles to move from gas at `source_temperature` into `volume`
/// litres to raise its pressure by `delta` kPa.
pub fn moles_for(delta: f64, volume: f64, source_temperature: f64) -> f64 {
    if delta <= 0.0 || source_temperature <= 0.0 {
        return 0.0;
    }
    delta * volume / (source_temperature * R)
}

/// Rebuild the networks when their shape has changed. Gas is handed out to members by
/// volume, then pooled again per new network, so nothing is created or lost.
pub fn rebuild_networks(
    mut nets: ResMut<PipeNets>,
    added: Query<(), Added<PipeMember>>,
    mut members: Query<(Entity, &GridPos, &PipeMember, &mut GasStore)>,
) {
    if !nets.dirty && added.is_empty() {
        return;
    }
    // Hand the current gas back to the members.
    for net in std::mem::take(&mut nets.nets) {
        for member in &net.members {
            if let Ok((_, _, m, mut store)) = members.get_mut(*member)
                && net.volume > 0.0
            {
                store.0 = net.mix.scaled(m.volume / net.volume);
            }
        }
    }
    nets.net_at.clear();

    nets.net_of.clear();

    // Open members by tile, in a stable order.
    let mut by_tile: BTreeMap<(i32, i32, i32), Vec<(Entity, u8)>> = BTreeMap::new();
    for (entity, pos, member, _) in &members {
        if member.open {
            by_tile
                .entry((pos.0.x, pos.0.y, pos.0.z))
                .or_default()
                .push((entity, member.connects));
        }
    }
    for list in by_tile.values_mut() {
        list.sort();
    }
    let tile_of = |key: (i32, i32, i32)| TilePos::new(key.0, key.1, key.2);

    // Flood-fill along connections.
    let starts: Vec<((i32, i32, i32), Entity, u8)> = by_tile
        .iter()
        .flat_map(|(k, list)| list.iter().map(move |(e, c)| (*k, *e, *c)))
        .collect();
    for start in starts {
        if nets.net_of.contains_key(&start.1) {
            continue;
        }
        let id = nets.nets.len();
        let mut net = PipeNet {
            members: Vec::new(),
            volume: 0.0,
            mix: GasMix::default(),
        };
        let mut queue = VecDeque::from([start]);
        nets.net_of.insert(start.1, id);
        while let Some((key, entity, connects)) = queue.pop_front() {
            let pos = tile_of(key);
            nets.net_at.entry(pos).or_insert(id);
            if let Ok((_, _, member, mut store)) = members.get_mut(entity) {
                net.members.push(entity);
                net.volume += member.volume;
                net.mix.merge(&store.0);
                store.0 = GasMix::default();
            }
            // Members on the same tile that share a side.
            let mut next: Vec<((i32, i32, i32), Entity, u8)> = by_tile[&key]
                .iter()
                .filter(|(e, c)| *e != entity && c & connects != 0)
                .map(|(e, c)| (key, *e, *c))
                .collect();
            // Members on a neighbouring tile that connect back.
            for d in [Dir::North, Dir::South, Dir::East, Dir::West] {
                if connects & side_bits(d) == 0 {
                    continue;
                }
                let there = pos.step(d);
                let there_key = (there.x, there.y, there.z);
                let back = side_bits(d.opposite());
                if let Some(list) = by_tile.get(&there_key) {
                    next.extend(
                        list.iter()
                            .filter(|(_, c)| c & back != 0)
                            .map(|(e, c)| (there_key, *e, *c)),
                    );
                }
            }
            for item in next {
                if let std::collections::hash_map::Entry::Vacant(slot) = nets.net_of.entry(item.1) {
                    slot.insert(id);
                    queue.push_back(item);
                }
            }
        }
        nets.nets.push(net);
    }
    nets.on_tile = by_tile.into_iter().map(|(k, list)| (tile_of(k), list)).collect();
    nets.dirty = false;
}

/// Run every machine once per machine cycle.
pub fn run_machines(
    time: Res<WorldTime>,
    atmos: Option<ResMut<StationAtmos>>,
    mut nets: ResMut<PipeNets>,
    pumps: Query<(&GridPos, &Pump, &Switch, &PowerUser)>,
    mut vents: Query<(Entity, &GridPos, &mut Vent, &Switch, &PowerUser)>,
    scrubbers: Query<(Entity, &GridPos, &Switch, &PowerUser), With<Scrubber>>,
    mut canisters: Query<(Entity, &GridPos, &Canister, &Switch, &PipeMember, &mut GasStore)>,
    pipe_vents: Query<(Entity, &GridPos), With<PipeVent>>,
    plates: Query<(Entity, &GridPos), With<ThermalPlate>>,
    (filters, mixers, injectors, held, mut tanks): (
        Query<(&GridPos, &GasFilter, &Switch, &PowerUser)>,
        Query<(&GridPos, &GasMixer, &Switch)>,
        Query<(Entity, &GridPos, &Injector, &Switch)>,
        Query<&crate::gas_panel::HeldTank>,
        Query<&mut crate::equipment::TankGas>,
    ),
) {
    let Some(mut atmos) = atmos else { return };
    if !time.tick.0.is_multiple_of(MACHINE_TICKS) {
        return;
    }
    let nets = &mut *nets;

    for (pos, pump, on, power) in &pumps {
        if !on.0 || !power.powered {
            continue;
        }
        let behind = pos.0.step(pump.dir.opposite());
        let ahead = pos.0.step(pump.dir);
        // The pipe behind must connect toward the pump, and the one ahead back to it.
        let (Some(from), Some(to)) = (
            nets.net_toward(behind, side_bits(pump.dir)),
            nets.net_toward(ahead, side_bits(pump.dir.opposite())),
        ) else {
            continue;
        };
        if from == to {
            continue;
        }
        let out = &nets.nets[to];
        let moles = moles_for(
            pump.target - out.pressure(),
            out.volume,
            nets.nets[from].mix.temperature,
        );
        let moved = nets.nets[from].mix.remove_moles(moles);
        nets.nets[to].mix.merge(&moved);
    }

    for (entity, pos, mut vent, on, power) in &mut vents {
        if !on.0 || !power.powered {
            continue;
        }
        if vent.siphon {
            let Some(&id) = nets.net_of.get(&entity) else {
                continue;
            };
            let moles = atmos
                .0
                .zone_at(pos.0)
                .map_or(0.0, |z| atmos.0.zones()[z].mix.total_moles() * SIPHON_SHARE);
            let taken = atmos.0.remove_at(pos.0, moles);
            nets.nets[id].mix.merge(&taken);
            continue;
        }
        // Panic fill (Luna `panic_fill`): while the room is below 95 kPa, pour in enough
        // to bring the whole zone to one atmosphere, breach or not.
        if vent.panic {
            let zone = atmos.0.zone_at(pos.0).map(|z| &atmos.0.zones()[z]);
            if let (Some(zone), Some(&id)) = (zone, nets.net_of.get(&entity))
                && zone.pressure() < crate::alarms::PANIC_FILL_UNTIL * ONE_ATMOSPHERE
            {
                let net = &mut nets.nets[id];
                let moles = moles_for(ONE_ATMOSPHERE - zone.pressure(), zone.volume, net.mix.temperature);
                let moved = net.mix.remove_moles(moles);
                atmos.0.add_at(pos.0, &moved);
                continue;
            }
            vent.panic = false;
        }
        // Air alarm behaviour: never feed a breach.
        if atmos.0.venting_to_space(pos.0) {
            continue;
        }
        let Some(&id) = nets.net_of.get(&entity) else {
            continue;
        };
        let room = atmos.0.pressure_at(pos.0);
        let net = &mut nets.nets[id];
        let delta = (vent.target - room).min(net.pressure() - room);
        let moles = moles_for(delta, TILE_VOLUME, net.mix.temperature);
        let moved = net.mix.remove_moles(moles);
        atmos.0.add_at(pos.0, &moved);
    }

    for (entity, pos, on, power) in &scrubbers {
        if !on.0 || !power.powered {
            continue;
        }
        let Some(&id) = nets.net_of.get(&entity) else {
            continue;
        };
        let filtered = atmos
            .0
            .remove_gases_at(pos.0, &SCRUBBED, SCRUBBER_VOLUME_RATE / TILE_VOLUME);
        nets.nets[id].mix.merge(&filtered);
    }

    // Gas filters (Luna `trinary/filter/process`). All three pipes must be there.
    for (pos, filter, on, power) in &filters {
        if !on.0 || !power.powered {
            continue;
        }
        let side_dir = clockwise(filter.dir);
        let (Some(from), Some(to), Some(side)) = (
            nets.net_toward(pos.0.step(filter.dir.opposite()), side_bits(filter.dir)),
            nets.net_toward(pos.0.step(filter.dir), side_bits(filter.dir.opposite())),
            nets.net_toward(pos.0.step(side_dir), side_bits(side_dir.opposite())),
        ) else {
            continue;
        };
        if from == to || from == side {
            continue;
        }
        let out = &nets.nets[to];
        let moles = moles_for(
            filter.target - out.pressure(),
            out.volume,
            nets.nets[from].mix.temperature,
        );
        let mut moved = nets.nets[from].mix.remove_moles(moles);
        if let Some(gas) = filter.gas {
            let mut taken = GasMix::default();
            taken.temperature = moved.temperature;
            taken.set(gas, moved.get(gas));
            moved.set(gas, 0.0);
            nets.nets[side].mix.merge(&taken);
        }
        nets.nets[to].mix.merge(&moved);
    }

    // Gas mixers (Luna `trinary/mixer/process`): each intake gives its share of the gas
    // the outlet still needs; if either runs short, both give that much less. Luna doesn't
    // check power. All three pipes must be there.
    for (pos, mixer, on) in &mixers {
        if !on.0 {
            continue;
        }
        let side_dir = clockwise(mixer.dir);
        let (Some(first), Some(second), Some(to)) = (
            nets.net_toward(pos.0.step(mixer.dir.opposite()), side_bits(mixer.dir)),
            nets.net_toward(pos.0.step(side_dir), side_bits(side_dir.opposite())),
            nets.net_toward(pos.0.step(mixer.dir), side_bits(mixer.dir.opposite())),
        ) else {
            continue;
        };
        if first == to || second == to || first == second {
            continue;
        }
        let out = &nets.nets[to];
        let delta = mixer.target - out.pressure();
        if delta <= 0.0 {
            continue;
        }
        let volume = out.volume;
        let want = |share: f64, temperature: f64| {
            if temperature > 0.0 {
                moles_for(share * delta, volume, temperature)
            } else {
                0.0
            }
        };
        let mut moles1 = want(mixer.node1, nets.nets[first].mix.temperature);
        let mut moles2 = want(mixer.node2, nets.nets[second].mix.temperature);
        let have1 = nets.nets[first].mix.total_moles();
        let have2 = nets.nets[second].mix.total_moles();
        if have1 < moles1 || have2 < moles2 {
            let ratio = |have: f64, want: f64| if want > 0.0 { have / want } else { f64::INFINITY };
            let ratio = ratio(have1, moles1).min(ratio(have2, moles2));
            moles1 *= ratio;
            moles2 *= ratio;
        }
        if moles1 > 0.0 {
            let moved = nets.nets[first].mix.remove_moles(moles1);
            nets.nets[to].mix.merge(&moved);
        }
        if moles2 > 0.0 {
            let moved = nets.nets[second].mix.remove_moles(moles2);
            nets.nets[to].mix.merge(&moved);
        }
    }

    // Outlet injectors (Luna `outlet_injector/process`): the gas that fills `volume_rate`
    // litres at the network's pressure goes into the room.
    for (entity, pos, injector, on) in &injectors {
        let Some(&id) = nets.net_of.get(&entity).filter(|_| on.0) else {
            continue;
        };
        let net = &mut nets.nets[id];
        let moles = moles_for(net.pressure(), injector.volume_rate, net.mix.temperature);
        let moved = net.mix.remove_moles(moles);
        atmos.0.add_at(pos.0, &moved);
    }

    // Thermal plates (Luna `thermal_plate/process`): a quarter of the plate's 200 L and a
    // quarter of the tile's air even out their temperatures; with almost no air around, a
    // quarter of the plate's gas evens out with a cold radiation sink instead.
    for (entity, pos) in &plates {
        let Some(&id) = nets.net_of.get(&entity) else {
            continue;
        };
        let net = &mut nets.nets[id];
        if net.volume <= 0.0 {
            continue;
        }
        let mut inside = net.mix.remove_fraction(PLATE_SHARE * DEVICE_VOLUME / net.volume);
        let tile_moles = atmos.0.air_at(pos.0).total_moles();
        if tile_moles < PLATE_MIN_MOLES {
            let capacity = inside.heat_capacity();
            if capacity > 0.0 {
                inside.temperature = (inside.thermal_energy() + RADIATION_CAPACITY * RADIATION_TEMPERATURE)
                    / (capacity + RADIATION_CAPACITY);
            }
        } else {
            let mut outside = atmos.0.remove_at(pos.0, PLATE_SHARE * tile_moles);
            let capacity = inside.heat_capacity() + outside.heat_capacity();
            if capacity > 0.0 {
                let temperature = (inside.thermal_energy() + outside.thermal_energy()) / capacity;
                inside.temperature = temperature;
                outside.temperature = temperature;
            }
            atmos.0.add_at(pos.0, &outside);
        }
        net.mix.merge(&inside);
    }

    // Pipe vents (Luna `mingle_with_turf`): 250 L of the network's gas evens out with the
    // room's whole zone by volume; in space it is simply lost.
    for (entity, pos) in &pipe_vents {
        let Some(&id) = nets.net_of.get(&entity) else {
            continue;
        };
        let net = &mut nets.nets[id];
        if net.volume <= 0.0 {
            continue;
        }
        let mut sample = net.mix.remove_fraction(PIPE_VENT_VOLUME / net.volume);
        let zone = atmos.0.zone_at(pos.0).map(|z| &atmos.0.zones()[z]);
        match zone.map(|z| (z.volume, z.mix.total_moles())) {
            Some((zone_volume, zone_moles)) => {
                let even =
                    (sample.total_moles() + zone_moles) * PIPE_VENT_VOLUME / (PIPE_VENT_VOLUME + zone_volume);
                let excess = sample.total_moles() - even;
                if excess > 0.0 {
                    let out = sample.remove_moles(excess);
                    atmos.0.add_at(pos.0, &out);
                } else {
                    let back = atmos.0.remove_at(pos.0, -excess);
                    sample.merge(&back);
                }
            }
            None => {
                atmos.0.add_at(pos.0, &sample);
                sample = GasMix::default();
            }
        }
        net.mix.merge(&sample);
    }

    // An open canister releases into the gas tank held in it, or else into the room, up to
    // its release pressure and never more than half the gap at a time (Luna `process`).
    for (entity, pos, canister, release, member, mut store) in &mut canisters {
        // On a port its gas is the pipes'.
        if !release.0 || member.open {
            continue;
        }
        let own = store.0.pressure(CANISTER_VOLUME);
        if let Some(mut tank) = held.get(entity).ok().and_then(|h| tanks.get_mut(h.0).ok()) {
            let env = tank.mix.pressure(tank.volume);
            let delta = (canister.release_target - env).min((own - env) / 2.0);
            let moles = moles_for(delta, tank.volume, store.0.temperature);
            let moved = store.0.remove_moles(moles);
            tank.mix.merge(&moved);
            continue;
        }
        let room = atmos.0.pressure_at(pos.0);
        let delta = (canister.release_target - room).min((own - room) / 2.0);
        let moles = moles_for(delta, TILE_VOLUME, store.0.temperature);
        let moved = store.0.remove_moles(moles);
        atmos.0.add_at(pos.0, &moved);
    }
}

/// Keep machine sprites in step with their switches. Canisters show their gauge instead.
pub fn update_machine_looks(
    mut machines: Query<
        (&Switch, &mut Appearance, Has<Valve>),
        (
            Changed<Switch>,
            Without<Canister>,
            Without<GasFilter>,
            Without<Pump>,
            Without<GasMixer>,
        ),
    >,
    mut facing: Query<
        (
            Option<&GasFilter>,
            Option<&Pump>,
            Option<&GasMixer>,
            &Switch,
            &mut Appearance,
        ),
        (
            Changed<Switch>,
            Without<Canister>,
            Or<(With<GasFilter>, With<Pump>, With<GasMixer>)>,
        ),
    >,
) {
    for (filter, pump, mixer, switch, mut look) in &mut facing {
        let dir = filter
            .map(|f| f.dir)
            .or(pump.map(|p| p.dir))
            .or(mixer.map(|m| m.dir))
            .unwrap_or(Dir::South);
        let state = facing_look(dir, switch.0);
        if look.state != state {
            look.state = state;
        }
    }
    for (switch, mut appearance, is_valve) in &mut machines {
        let state = match (is_valve, switch.0) {
            (true, true) => "open",
            (true, false) => "closed",
            (false, true) => "on",
            (false, false) => "off",
        };
        if appearance.state != state {
            appearance.state = state.to_owned();
        }
    }
}

/// A filter's, pump's or mixer's sprite: on or off, and the way it faces ("on-north").
pub fn facing_look(dir: Dir, on: bool) -> String {
    let facing = match dir {
        Dir::North => "north",
        Dir::East => "east",
        Dir::West => "west",
        _ => "south",
    };
    format!("{}-{facing}", if on { "on" } else { "off" })
}

/// A canister shows its gauge light, and the connector when it is on a port (Luna
/// `canister/update_icon`).
pub fn update_canister_looks(
    nets: Res<PipeNets>,
    mut canisters: Query<(Entity, &PipeMember, &GasStore, &mut Appearance), With<Canister>>,
) {
    for (entity, member, store, mut look) in &mut canisters {
        let kpa = nets.gauge(entity, Some(store), member.volume);
        let mut state = format!("+can-o{}", canister_light(kpa));
        if member.open {
            state.push_str("+can-connector");
        }
        if look.state != state {
            look.state = state;
        }
    }
}

/// A canister of air or of one gas, released into the room toward one atmosphere.
fn canister(sprite: &'static str, gas: GasMix) -> MachineSpawn {
    MachineSpawn::new(sprite, "+can-o3")
        .switch(false)
        // Starts connected if spawned on a port (`spawn_map` checks).
        .member(PipeMember {
            volume: CANISTER_VOLUME,
            open: false,
            connects: ALL_SIDES,
        })
        .gas(gas)
        .canister(Canister {
            release_target: ONE_ATMOSPHERE,
        })
        .dense()
}

/// The components a machine prototype spawns with, or `None` if `proto` is not a machine.
pub fn machine_bundle(
    proto: &str,
    dir: Option<Dir>,
    vars: &BTreeMap<String, String>,
) -> Option<MachineSpawn> {
    let number = |name: &str| vars.get(name).and_then(|v| v.trim().parse::<f64>().ok());
    let on = number("on").is_some_and(|v| v != 0.0);
    let connects = connections(proto, dir);
    let member = |volume| PipeMember {
        volume,
        open: true,
        connects,
    };
    // Devices hold no gas of their own here; they only join the pipe they sit on.
    let device = member(0.0);
    Some(match proto {
        "pipe" => {
            let mut pipe = MachineSpawn::new("pipe", "").member(member(PIPE_VOLUME));
            // Placed with a direction, a piece shows its own shape.
            if dir.is_some() {
                pipe.state = side_names(connects);
            }
            pipe
        }
        "manifold" => {
            let mut manifold = MachineSpawn::new("manifold", "").member(member(MANIFOLD_VOLUME));
            manifold.state = side_names(connects);
            manifold
        }
        "air-tank" => MachineSpawn::new("air-tank", "")
            .member(member(TANK_VOLUME))
            .gas(air_at_pressure(TANK_VOLUME, TANK_PRESSURE))
            .dense(),
        "pump" => {
            // Hand-drawn maps leave it on at 300 kPa; Luna's map says (`pipes.md`).
            let on = vars.get("on").is_none_or(|_| on);
            let pump = Pump {
                dir: dir.unwrap_or(Dir::East),
                target: number("target_pressure").unwrap_or(PUMP_TARGET),
            };
            let mut spawn = MachineSpawn::new("pump", "").switch(on).powered().pump(pump);
            spawn.state = facing_look(pump.dir, on);
            spawn
        }
        "gas-mixer" => {
            let mixer = GasMixer {
                dir: dir.unwrap_or(Dir::South),
                target: number("target_pressure").unwrap_or(ONE_ATMOSPHERE),
                node1: number("node1_concentration").unwrap_or(0.5),
                node2: number("node2_concentration").unwrap_or(0.5),
            };
            let mut spawn = MachineSpawn::new("gas-mixer", "").switch(on).dense();
            spawn.state = facing_look(mixer.dir, on);
            spawn.mixer = Some(mixer);
            spawn
        }
        "valve" => MachineSpawn::new("valve", "open")
            .switch(true)
            .member(member(PIPE_VOLUME))
            .valve(),
        "vent" => MachineSpawn::new("vent", "on")
            .switch(true)
            .powered()
            .member(device)
            .vent(Vent {
                target: ONE_ATMOSPHERE,
                panic: false,
                siphon: false,
            }),
        "air-alarm" => {
            let mut alarm = MachineSpawn::new("air-alarm", "alarm0");
            alarm.power = Some(crate::power::PowerUser::new(
                crate::power::Channel::Environment,
                crate::alarms::ALARM_POWER,
            ));
            alarm.alarm = true;
            alarm
        }
        "scrubber" => MachineSpawn::new("scrubber", "on")
            .switch(true)
            .powered()
            .member(device)
            .scrubber(),
        "connector" => MachineSpawn::new("connector", "").member(device).connector(),
        "generator-input" => {
            let mut spawn = MachineSpawn::new("generator-input", "")
                .member(member(DEVICE_VOLUME))
                .dense();
            spawn.generator_input = true;
            spawn
        }
        "freezer" => {
            let mut spawn = MachineSpawn::new("freezer", "freezer")
                .member(member(DEVICE_VOLUME))
                .dense();
            spawn.freezer = true;
            spawn
        }
        "thermal-plate" => {
            let mut spawn = MachineSpawn::new("thermal-plate", "")
                .member(member(DEVICE_VOLUME))
                .dense();
            spawn.thermal_plate = true;
            spawn
        }
        "pipe-vent" => MachineSpawn::new("pipe-vent", "")
            .member(member(PIPE_VENT_VOLUME))
            .pipe_vent(),
        "gas-filter" => {
            let filter = GasFilter {
                dir: dir.unwrap_or(Dir::South),
                gas: filter_gas(number("filter_type").unwrap_or(0.0) as i64),
                target: number("target_pressure").unwrap_or(FILTER_TARGET),
            };
            let mut spawn = MachineSpawn::new("gas-filter", "").switch(on).powered();
            spawn.state = facing_look(filter.dir, on);
            spawn.filter = Some(filter);
            spawn
        }
        "injector" => {
            let mut spawn = MachineSpawn::new("injector", if on { "on" } else { "off" })
                .switch(on)
                .member(device);
            spawn.injector = Some(Injector {
                volume_rate: number("volume_rate").unwrap_or(INJECTOR_VOLUME_RATE),
            });
            spawn
        }
        "canister" => canister("canister", air_at_pressure(CANISTER_VOLUME, CANISTER_PRESSURE)),
        // Luna's plain canister starts empty, for filling.
        "empty-canister" => {
            let mut empty = GasMix::default();
            empty.temperature = T20C;
            canister("empty-canister", empty)
        }
        _ => {
            if let Some(&(id, gas)) = GAS_TANKS.iter().find(|(id, _)| *id == proto) {
                MachineSpawn::new(id, "")
                    .member(member(GAS_TANK_VOLUME))
                    .gas(gas_at_pressure(gas, GAS_TANK_VOLUME, GAS_TANK_PRESSURE))
                    .dense()
            } else if let Some(&(id, gas)) = GAS_CANISTERS.iter().find(|(id, _)| *id == proto) {
                canister(id, gas_at_pressure(gas, CANISTER_VOLUME, CANISTER_PRESSURE))
            } else {
                return None;
            }
        }
    })
}

/// What `machine_bundle` asks `spawn_object` to attach.
#[derive(Debug, Clone)]
pub struct MachineSpawn {
    pub sprite: &'static str,
    pub state: String,
    pub member: Option<PipeMember>,
    pub gas: GasMix,
    pub switch: Option<bool>,
    pub pump: Option<Pump>,
    pub vent: Option<Vent>,
    pub canister: Option<Canister>,
    pub valve: bool,
    pub scrubber: bool,
    pub connector: bool,
    pub pipe_vent: bool,
    pub filter: Option<GasFilter>,
    pub mixer: Option<GasMixer>,
    pub generator_input: bool,
    pub thermal_plate: bool,
    pub freezer: bool,
    pub injector: Option<Injector>,
    pub dense: bool,
    pub power: Option<crate::power::PowerUser>,
    pub alarm: bool,
}

impl MachineSpawn {
    fn new(sprite: &'static str, state: &'static str) -> Self {
        Self {
            sprite,
            state: state.to_owned(),
            member: None,
            gas: air_at_pressure(1.0, 0.0),
            switch: None,
            pump: None,
            vent: None,
            canister: None,
            valve: false,
            scrubber: false,
            connector: false,
            pipe_vent: false,
            filter: None,
            mixer: None,
            generator_input: false,
            thermal_plate: false,
            freezer: false,
            injector: None,
            dense: false,
            power: None,
            alarm: false,
        }
    }
    fn powered(mut self) -> Self {
        self.power = Some(crate::power::PowerUser::new(
            crate::power::Channel::Environment,
            crate::power::ATMOS_MACHINE_POWER,
        ));
        self
    }
    fn member(mut self, m: PipeMember) -> Self {
        self.member = Some(m);
        self
    }
    fn gas(mut self, g: GasMix) -> Self {
        self.gas = g;
        self
    }
    fn switch(mut self, on: bool) -> Self {
        self.switch = Some(on);
        self
    }
    fn pump(mut self, p: Pump) -> Self {
        self.pump = Some(p);
        self
    }
    fn vent(mut self, v: Vent) -> Self {
        self.vent = Some(v);
        self
    }
    fn canister(mut self, c: Canister) -> Self {
        self.canister = Some(c);
        self
    }
    fn valve(mut self) -> Self {
        self.valve = true;
        self
    }
    fn scrubber(mut self) -> Self {
        self.scrubber = true;
        self
    }
    fn connector(mut self) -> Self {
        self.connector = true;
        self
    }
    fn pipe_vent(mut self) -> Self {
        self.pipe_vent = true;
        self
    }
    fn dense(mut self) -> Self {
        self.dense = true;
        self
    }

    /// Attach everything to a freshly spawned entity.
    pub fn apply(&self, entity: &mut EntityWorldMut) {
        if let Some(m) = self.member {
            entity.insert((m, GasStore(self.gas)));
        }
        if let Some(on) = self.switch {
            entity.insert(Switch(on));
        }
        if let Some(p) = self.pump {
            entity.insert(p);
        }
        if let Some(v) = self.vent {
            entity.insert(v);
        }
        if let Some(c) = self.canister {
            entity.insert((c, Portable));
        }
        if self.valve {
            entity.insert(Valve);
        }
        if self.scrubber {
            entity.insert(Scrubber);
        }
        if self.connector {
            entity.insert(Connector);
        }
        if self.pipe_vent {
            entity.insert(PipeVent);
        }
        if self.generator_input {
            entity.insert(GeneratorInput);
        }
        if self.thermal_plate {
            entity.insert(ThermalPlate);
        }
        if self.freezer {
            entity.insert(crate::freezers::Freezer::default());
        }
        if let Some(f) = self.filter {
            entity.insert(f);
        }
        if let Some(m) = self.mixer {
            entity.insert(m);
        }
        if let Some(i) = self.injector {
            entity.insert(i);
        }
        if self.dense {
            entity.insert(rs13_world::Dense);
        }
        if let Some(power) = self.power {
            entity.insert(power);
        }
        if self.alarm {
            entity.insert(crate::alarms::AirAlarm::default());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tank_and_canister_hold_what_they_say() {
        let tank = air_at_pressure(TANK_VOLUME, TANK_PRESSURE);
        assert!((tank.pressure(TANK_VOLUME) - TANK_PRESSURE).abs() < 1e-6);
        // Enough air to refill the whole engineering chamber twice over.
        assert!(tank.total_moles() > 36_000.0);
    }

    #[test]
    fn pump_formula_raises_pressure_by_delta() {
        let volume = 1000.0;
        let n = moles_for(50.0, volume, T20C);
        assert!((air_at_pressure(volume, 50.0).total_moles() - n).abs() < 1e-6);
        assert_eq!(moles_for(-1.0, volume, T20C), 0.0);
    }
}
