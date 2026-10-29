//! Atmospherics machinery: pipe networks, pumps, valves, vents, scrubbers, tanks and
//! canisters. Spec and tuning: `docs/reference/pipes.md`.

use std::collections::{BTreeMap, HashMap, VecDeque};

use bevy_ecs::prelude::*;
use rs13_simulation::atmos::gas::{ONE_ATMOSPHERE, R, TILE_VOLUME};
use rs13_simulation::atmos::{Gas, GasMix};
use rs13_world::{Appearance, Dir, GridPos, TilePos};

use crate::atmos::StationAtmos;
use crate::time::WorldTime;

/// Machines run every this many ticks (2 s), like SS13's machinery cycle.
pub const MACHINE_TICKS: u64 = 40;
/// Litres of one pipe segment (SS13 value).
pub const PIPE_VOLUME: f64 = 70.0;
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

/// Part of a pipe network: it holds `volume` litres of the network's gas and connects to
/// members on its own and neighbouring tiles, while `open`.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct PipeMember {
    pub volume: f64,
    pub open: bool,
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
    pub fn pressure(&self) -> f64 {
        self.mix.pressure(self.volume)
    }
}

/// All pipe networks, and which tile belongs to which.
#[derive(Resource, Debug, Default)]
pub struct PipeNets {
    pub nets: Vec<PipeNet>,
    pub net_at: HashMap<TilePos, usize>,
    /// Set when the shape of the networks changes (valves, canisters, new pipes).
    pub dirty: bool,
}

impl PipeNets {
    pub fn at(&self, pos: TilePos) -> Option<&PipeNet> {
        self.net_at.get(&pos).map(|i| &self.nets[*i])
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

    // Flood-fill connected tiles of open members.
    let mut by_tile: BTreeMap<(i32, i32, i32), Vec<Entity>> = BTreeMap::new();
    for (entity, pos, member, _) in &members {
        if member.open {
            by_tile
                .entry((pos.0.x, pos.0.y, pos.0.z))
                .or_default()
                .push(entity);
        }
    }
    let mut visited: HashMap<(i32, i32, i32), usize> = HashMap::new();
    let tiles: Vec<(i32, i32, i32)> = by_tile.keys().copied().collect();
    for start in tiles {
        if visited.contains_key(&start) {
            continue;
        }
        let id = nets.nets.len();
        let mut net = PipeNet {
            members: Vec::new(),
            volume: 0.0,
            mix: GasMix::default(),
        };
        let mut queue = VecDeque::from([start]);
        visited.insert(start, id);
        while let Some(tile) = queue.pop_front() {
            let pos = TilePos::new(tile.0, tile.1, tile.2);
            nets.net_at.insert(pos, id);
            for entity in &by_tile[&tile] {
                let Ok((_, _, member, mut store)) = members.get_mut(*entity) else {
                    continue;
                };
                net.members.push(*entity);
                net.volume += member.volume;
                net.mix.merge(&store.0);
                store.0 = GasMix::default();
            }
            for d in [Dir::North, Dir::East, Dir::South, Dir::West] {
                let next = pos.step(d);
                let key = (next.x, next.y, next.z);
                if by_tile.contains_key(&key) && !visited.contains_key(&key) {
                    visited.insert(key, id);
                    queue.push_back(key);
                }
            }
        }
        nets.nets.push(net);
    }
    nets.dirty = false;
}

/// Run every machine once per machine cycle.
pub fn run_machines(
    time: Res<WorldTime>,
    atmos: Option<ResMut<StationAtmos>>,
    mut nets: ResMut<PipeNets>,
    pumps: Query<(&GridPos, &Pump, &Switch)>,
    vents: Query<(&GridPos, &Vent, &Switch)>,
    scrubbers: Query<(&GridPos, &Switch), With<Scrubber>>,
    mut canisters: Query<(&GridPos, &Canister, &Switch, &PipeMember, &mut GasStore)>,
) {
    let Some(mut atmos) = atmos else { return };
    if !time.tick.0.is_multiple_of(MACHINE_TICKS) {
        return;
    }
    let nets = &mut *nets;

    for (pos, pump, on) in &pumps {
        if !on.0 {
            continue;
        }
        let behind = pos.0.step(opposite(pump.dir));
        let ahead = pos.0.step(pump.dir);
        let (Some(&from), Some(&to)) = (nets.net_at.get(&behind), nets.net_at.get(&ahead)) else {
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

    for (pos, vent, on) in &vents {
        if !on.0 {
            continue;
        }
        // Air alarm behaviour: never feed a breach.
        if atmos.0.venting_to_space(pos.0) {
            continue;
        }
        let Some(&id) = nets.net_at.get(&pos.0) else {
            continue;
        };
        let room = atmos.0.pressure_at(pos.0);
        let net = &mut nets.nets[id];
        let delta = (vent.target - room).min(net.pressure() - room);
        let moles = moles_for(delta, TILE_VOLUME, net.mix.temperature);
        let moved = net.mix.remove_moles(moles);
        atmos.0.add_at(pos.0, &moved);
    }

    for (pos, on) in &scrubbers {
        if !on.0 {
            continue;
        }
        let Some(&id) = nets.net_at.get(&pos.0) else {
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
    let member = |volume| PipeMember { volume, open: true };
    Some(match proto {
        "pipe" => MachineSpawn::new("pipe", "").member(member(PIPE_VOLUME)),
        "air-tank" => MachineSpawn::new("air-tank", "")
            .member(member(TANK_VOLUME))
            .gas(air_at_pressure(TANK_VOLUME, TANK_PRESSURE))
            .dense(),
        "pump" => MachineSpawn::new("pump", "on").switch(true).pump(Pump {
            dir: dir.unwrap_or(Dir::East),
            target: PUMP_TARGET,
        }),
        "valve" => MachineSpawn::new("valve", "open")
            .switch(true)
            .member(member(PIPE_VOLUME))
            .valve(),
        "vent" => MachineSpawn::new("vent", "on").switch(true).vent(Vent {
            target: ONE_ATMOSPHERE,
        }),
        "scrubber" => MachineSpawn::new("scrubber", "on").switch(true).scrubber(),
        "connector" => MachineSpawn::new("connector", "").connector(),
        "canister" => MachineSpawn::new("canister", "off")
            .switch(false)
            // Starts connected if spawned on a port (`spawn_object` checks).
            .member(PipeMember {
                volume: CANISTER_VOLUME,
                open: false,
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
    pub state: &'static str,
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
}

impl MachineSpawn {
    fn new(sprite: &'static str, state: &'static str) -> Self {
        Self {
            sprite,
            state,
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
        }
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
