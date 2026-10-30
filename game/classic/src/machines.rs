//! Atmospherics machinery: pipe networks, pumps, valves, vents, scrubbers, tanks and
//! canisters. Spec and tuning: `docs/reference/pipes.md`.

use std::collections::{BTreeMap, HashMap, VecDeque};

use bevy_ecs::prelude::*;
use rs13_simulation::atmos::gas::{ONE_ATMOSPHERE, R, TILE_VOLUME};
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
/// Default output pressure of a gas pump, kPa.
pub const PUMP_TARGET: f64 = 300.0;
/// Litres of room air a scrubber processes per cycle (SS13 `volume_rate`).
pub const SCRUBBER_VOLUME_RATE: f64 = 120.0;
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
}

/// Pulls CO2, plasma and N2O out of the room into the network under it.
#[derive(Component, Debug, Clone, Copy)]
pub struct Scrubber;

/// A port a canister can be wrenched onto.
#[derive(Component, Debug, Clone, Copy)]
pub struct Connector;

/// A portable gas canister. Released into the room up to `release_target` kPa while its
/// valve (`Switch`) is open and it is not connected to a port.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Canister {
    pub release_target: f64,
}

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
                let back = side_bits(opposite(d));
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
    mut canisters: Query<(&GridPos, &Canister, &Switch, &PipeMember, &mut GasStore)>,
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
        let behind = pos.0.step(opposite(pump.dir));
        let ahead = pos.0.step(pump.dir);
        // The pipe behind must connect toward the pump, and the one ahead back to it.
        let (Some(from), Some(to)) = (
            nets.net_toward(behind, side_bits(pump.dir)),
            nets.net_toward(ahead, side_bits(opposite(pump.dir))),
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

    for (pos, canister, release, member, mut store) in &mut canisters {
        if !release.0 || member.open {
            continue;
        }
        let room = atmos.0.pressure_at(pos.0);
        let delta = (canister.release_target - room).min(store.0.pressure(CANISTER_VOLUME) - room);
        let moles = moles_for(delta, TILE_VOLUME, store.0.temperature);
        let moved = store.0.remove_moles(moles);
        atmos.0.add_at(pos.0, &moved);
    }
}

/// Keep machine sprites in step with their switches.
pub fn update_machine_looks(mut machines: Query<(&Switch, &mut Appearance, Has<Valve>), Changed<Switch>>) {
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

fn opposite(dir: Dir) -> Dir {
    let (dx, dy) = dir.offset();
    Dir::from_offset(-dx, -dy).expect("non-zero")
}

/// The components a machine prototype spawns with, or `None` if `proto` is not a machine.
pub fn machine_bundle(proto: &str, dir: Option<Dir>) -> Option<MachineSpawn> {
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
        "pump" => MachineSpawn::new("pump", "on").switch(true).powered().pump(Pump {
            dir: dir.unwrap_or(Dir::East),
            target: PUMP_TARGET,
        }),
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
        "canister" => MachineSpawn::new("canister", "off")
            .switch(false)
            // Starts connected if spawned on a port (`spawn_object` checks).
            .member(PipeMember {
                volume: CANISTER_VOLUME,
                open: false,
                connects: ALL_SIDES,
            })
            .gas(air_at_pressure(CANISTER_VOLUME, CANISTER_PRESSURE))
            .canister(Canister {
                release_target: ONE_ATMOSPHERE,
            })
            .dense(),
        _ => return None,
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
            entity.insert(c);
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
    use rs13_simulation::atmos::gas::T20C;

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
