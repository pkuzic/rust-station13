//! Electrical power: cables, networks, SMES batteries, APCs and the machines they feed.
//! Spec: `docs/reference/power.md`.

use std::collections::{BTreeMap, HashMap, VecDeque};

use bevy_ecs::prelude::*;
use rs13_core::TICK_RATE_HZ;
use rs13_world::{Dir, GridPos, MapLayer, TilePos};

use crate::doors::Airlock;
use crate::machines::{MACHINE_TICKS, Switch};
use crate::time::WorldTime;

/// Seconds per power cycle (the machine cycle).
pub const POWER_DT: f64 = MACHINE_TICKS as f64 / TICK_RATE_HZ as f64;
/// SMES capacity, J (Luna: 10⁸ W-cycles).
pub const SMES_CAPACITY: f64 = 2.0e8;
/// SMES default output, W.
pub const SMES_OUTPUT: f64 = 30_000.0;
/// APC cell capacity, J (Luna: 2 500 units).
pub const APC_CELL_CAPACITY: f64 = 2.5e6;
/// APC cell charge at round start, fraction.
pub const APC_START_CHARGE: f64 = 0.95;
/// Fraction of cell capacity an APC may trickle-charge per cycle (Luna `CHARGELEVEL`).
pub const APC_TRICKLE: f64 = 0.001;
/// Energy one airlock open or close takes, J (Luna `use_power(50)` for one cycle).
pub const AIRLOCK_OPERATION_ENERGY: f64 = 50.0 * POWER_DT;
/// Draw of a running vent, scrubber or pump, W (/tg/-style; Luna's drew nothing).
pub const ATMOS_MACHINE_POWER: f64 = 150.0;
/// Access needed to work an APC's breaker.
pub const APC_ACCESS: &str = "engine_equip";

/// The three things an APC powers in its area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Channel {
    Equipment = 0,
    Lighting = 1,
    Environment = 2,
}

impl Channel {
    pub const ALL: [Channel; 3] = [Channel::Equipment, Channel::Lighting, Channel::Environment];
}

/// A channel's setting (Luna: 0 off, 1 auto-off, 2 on, 3 auto-on).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelMode {
    Off,
    AutoOff,
    On,
    AutoOn,
}

impl ChannelMode {
    pub fn is_on(self) -> bool {
        matches!(self, ChannelMode::On | ChannelMode::AutoOn)
    }

    /// Luna's `autoset(val, on)`: 0 forces off (even a manual "on"), 1 switches automatic
    /// channels back on, 2 switches automatic channels off but leaves a manual "on" alone.
    fn autoset(self, on: u8) -> Self {
        match (on, self) {
            (0, ChannelMode::On) => ChannelMode::Off,
            (0, ChannelMode::AutoOn) | (2, ChannelMode::AutoOn) => ChannelMode::AutoOff,
            (1, ChannelMode::AutoOff) => ChannelMode::AutoOn,
            (_, mode) => mode,
        }
    }
}

/// Joins power networks: cables, SMES units, APCs.
#[derive(Component, Debug, Clone, Copy)]
pub struct PowerNode;

#[derive(Component, Debug, Clone, Copy)]
pub struct Cable;

/// A big battery feeding its network.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Smes {
    pub charge: f64,
    pub capacity: f64,
    pub output: f64,
    pub online: bool,
}

/// An area power controller.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Apc {
    pub area: String,
    /// Cell charge and capacity, J; `None` with no cell fitted.
    pub cell: Option<(f64, f64)>,
    /// The main breaker.
    pub operating: bool,
    pub channels: [ChannelMode; 3],
}

impl Apc {
    pub fn new(area: impl Into<String>) -> Self {
        Self {
            area: area.into(),
            cell: Some((APC_CELL_CAPACITY * APC_START_CHARGE, APC_CELL_CAPACITY)),
            operating: true,
            channels: [ChannelMode::AutoOn; 3],
        }
    }

    pub fn cell_percent(&self) -> f64 {
        self.cell
            .map_or(0.0, |(charge, capacity)| 100.0 * charge / capacity)
    }

    fn autoset_all(&mut self, settings: [u8; 3]) {
        for (mode, on) in self.channels.iter_mut().zip(settings) {
            *mode = mode.autoset(on);
        }
    }
}

/// A machine that needs power from its area's APC.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct PowerUser {
    pub channel: Channel,
    /// Continuous draw while running, W.
    pub watts: f64,
    /// Whether the area currently powers it.
    pub powered: bool,
}

impl PowerUser {
    pub fn new(channel: Channel, watts: f64) -> Self {
        Self {
            channel,
            watts,
            powered: true,
        }
    }
}

/// Power networks: which tiles connect to which.
#[derive(Resource, Debug, Default)]
pub struct Powernets {
    pub net_at: HashMap<TilePos, usize>,
    pub count: usize,
    pub dirty: bool,
}

/// Per-area power state.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AreaState {
    pub on: [bool; 3],
    /// Energy used this cycle, J, per channel.
    pub used: [f64; 3],
}

#[derive(Resource, Debug, Default)]
pub struct AreaPower(pub HashMap<String, AreaState>);

impl AreaPower {
    /// Areas without an APC are always powered (Luna).
    pub fn powered(&self, area: Option<&str>, channel: Channel) -> bool {
        match area.and_then(|a| self.0.get(a)) {
            Some(state) => state.on[channel as usize],
            None => true,
        }
    }
}

/// Rebuild networks when nodes appear or the shape changes: flood fill over tiles with
/// power nodes (cables, SMES, APCs), like pipes.
pub fn rebuild_powernets(
    mut nets: ResMut<Powernets>,
    added: Query<(), Added<PowerNode>>,
    nodes: Query<&GridPos, With<PowerNode>>,
) {
    if !nets.dirty && added.is_empty() {
        return;
    }
    let tiles: BTreeMap<(i32, i32, i32), ()> = nodes.iter().map(|p| ((p.0.x, p.0.y, p.0.z), ())).collect();
    nets.net_at.clear();
    let mut count = 0;
    for start in tiles.keys().copied() {
        let start_pos = TilePos::new(start.0, start.1, start.2);
        if nets.net_at.contains_key(&start_pos) {
            continue;
        }
        let id = count;
        count += 1;
        let mut queue = VecDeque::from([start_pos]);
        nets.net_at.insert(start_pos, id);
        while let Some(pos) = queue.pop_front() {
            for d in [Dir::North, Dir::East, Dir::South, Dir::West] {
                let next = pos.step(d);
                if tiles.contains_key(&(next.x, next.y, next.z)) && !nets.net_at.contains_key(&next) {
                    nets.net_at.insert(next, id);
                    queue.push_back(next);
                }
            }
        }
    }
    nets.count = count;
    nets.dirty = false;
}

/// One power cycle: areas run up their bills, APCs pay them from the grid or their cells,
/// SMES units discharge by what was used, and channels switch with the cells' charge.
pub fn power_cycle(
    time: Res<WorldTime>,
    map: Single<&MapLayer>,
    nets: Res<Powernets>,
    mut areas: ResMut<AreaPower>,
    mut users: Query<(&GridPos, &mut PowerUser, Option<&Switch>, Option<&mut Airlock>)>,
    mut apcs: Query<(Entity, &GridPos, &mut Apc)>,
    mut batteries: Query<(&GridPos, &mut Smes)>,
) {
    if !time.tick.0.is_multiple_of(MACHINE_TICKS) {
        return;
    }
    let map = &map.0;

    // Usage, by the power each machine was given last cycle.
    for (pos, user, switch, airlock) in &mut users {
        let Some(area) = map.area(pos.0) else { continue };
        let mut energy = 0.0;
        if user.powered && switch.is_none_or(|s| s.0) {
            energy += user.watts * POWER_DT;
        }
        if let Some(mut airlock) = airlock {
            energy += f64::from(airlock.operations) * AIRLOCK_OPERATION_ENERGY;
            airlock.operations = 0;
        }
        if let Some(state) = areas.0.get_mut(area) {
            state.used[user.channel as usize] += energy;
        }
    }

    // What each network's SMES units can give this cycle.
    let mut offered = vec![0.0; nets.count];
    for (pos, smes) in &batteries {
        if let (true, Some(&net)) = (smes.online, nets.net_at.get(&pos.0)) {
            offered[net] += smes.charge.min(smes.output * POWER_DT);
        }
    }
    let mut remaining = offered.clone();

    let mut order: Vec<Entity> = apcs.iter().map(|(e, ..)| e).collect();
    order.sort();
    for entity in order {
        let Ok((_, pos, mut apc)) = apcs.get_mut(entity) else {
            continue;
        };
        let state = areas.0.entry(apc.area.clone()).or_default();
        let usage: f64 = state.used.iter().sum();
        state.used = [0.0; 3];
        let grid = nets.net_at.get(&pos.0).map(|n| &mut remaining[*n]);
        let mut spare = grid.as_deref().copied().unwrap_or(0.0);

        match apc.cell {
            Some((mut charge, capacity)) => {
                let trickle = (capacity * APC_TRICKLE).min(capacity - charge);
                let mut can_cover = true;
                if spare >= usage + trickle {
                    spare -= usage + trickle;
                    charge += trickle;
                } else if spare + charge >= usage {
                    let from_grid = spare.min(usage);
                    spare -= from_grid;
                    charge -= usage - from_grid;
                } else {
                    can_cover = false;
                }
                apc.cell = Some((charge, capacity));
                let percent = 100.0 * charge / capacity;
                if !can_cover || charge <= 0.0 {
                    apc.autoset_all([0, 0, 0]);
                } else if percent < 15.0 {
                    apc.autoset_all([2, 2, 1]);
                } else if percent < 30.0 {
                    apc.autoset_all([1, 2, 1]);
                } else {
                    apc.autoset_all([1, 1, 1]);
                }
            }
            None => apc.autoset_all([0, 0, 0]),
        }
        if let Some(grid) = grid {
            *grid = spare;
        }
        let operating = apc.operating && apc.cell.is_some();
        state.on = apc.channels.map(|c| operating && c.is_on());
    }

    // SMES units lose only what was drawn, in proportion to what they offered.
    for (pos, mut smes) in &mut batteries {
        let Some(&net) = nets.net_at.get(&pos.0) else {
            continue;
        };
        if !smes.online || offered[net] <= 0.0 {
            continue;
        }
        let share = smes.charge.min(smes.output * POWER_DT) / offered[net];
        let drawn = (offered[net] - remaining[net]) * share;
        smes.charge = (smes.charge - drawn).max(0.0);
    }

    // Tell every machine whether it has power now.
    for (pos, mut user, _, airlock) in &mut users {
        let powered = areas.powered(map.area(pos.0), user.channel);
        user.powered = powered;
        if let Some(mut airlock) = airlock
            && airlock.powered != powered
        {
            airlock.powered = powered;
        }
    }
}

/// APC sprite: off with the breaker open, "low" once equipment has been cut, else on.
pub fn update_power_looks(mut apcs: Query<(&Apc, &mut rs13_world::Appearance), Changed<Apc>>) {
    for (apc, mut appearance) in &mut apcs {
        let state = if !apc.operating {
            "off"
        } else if !apc.channels[Channel::Equipment as usize].is_on() {
            "low"
        } else {
            "on"
        };
        if appearance.state != state {
            appearance.state = state.to_owned();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autoset_follows_luna() {
        use ChannelMode::*;
        assert_eq!(On.autoset(0), Off, "0 forces even a manual on off");
        assert_eq!(AutoOn.autoset(0), AutoOff);
        assert_eq!(AutoOff.autoset(1), AutoOn);
        assert_eq!(Off.autoset(1), Off, "a manual off stays off");
        assert_eq!(On.autoset(2), On, "2 leaves a manual on alone");
        assert_eq!(AutoOn.autoset(2), AutoOff);
    }
}
