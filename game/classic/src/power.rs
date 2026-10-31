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

/// A length of power cable. Its two ends point at neighbouring tiles; `None` is a knot in
/// the middle of its own tile (Luna's `D1-D2` with 0). A knot comes first, otherwise the
/// end with the lower Luna direction number.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cable {
    pub a: Option<Dir>,
    pub b: Dir,
}

impl Cable {
    /// A cable between two ends, in either order. Two knots, or the same end twice, make
    /// no cable.
    pub fn new(x: Option<Dir>, y: Option<Dir>) -> Option<Self> {
        let (a, b) = match (x, y) {
            (None, None) => return None,
            (Some(x), Some(y)) if x == y => return None,
            (None, Some(d)) | (Some(d), None) => (None, d),
            (Some(x), Some(y)) if luna_dir(x) <= luna_dir(y) => (Some(x), y),
            (Some(x), Some(y)) => (Some(y), x),
        };
        Some(Self { a, b })
    }

    /// From map ends: one direction is a knot plus that direction. A cable placed without
    /// any is Luna's default, a knot pointing north.
    pub fn from_ends(ends: &[Dir]) -> Option<Self> {
        match ends {
            [] => Self::new(None, Some(Dir::North)),
            [d] => Self::new(None, Some(*d)),
            [x, y] => Self::new(Some(*x), Some(*y)),
            _ => None,
        }
    }

    pub fn ends(self) -> [Option<Dir>; 2] {
        [self.a, Some(self.b)]
    }

    pub fn has_end(self, end: Option<Dir>) -> bool {
        self.a == end || Some(self.b) == end
    }

    /// Sprite state, Luna's icon state: "0-4" (a knot to the east), "1-2" (north-south).
    pub fn state(self) -> String {
        format!("{}-{}", self.a.map_or(0, luna_dir), luna_dir(self.b))
    }

    /// How many lengths of cable it gives back when cut (Luna: 2, or 1 for a knot).
    pub fn lengths(self) -> u32 {
        if self.a.is_some() { 2 } else { 1 }
    }
}

/// BYOND's direction numbers: N 1, S 2, E 4, W 8, and their sums for diagonals.
pub fn luna_dir(d: Dir) -> u8 {
    match d {
        Dir::North => 1,
        Dir::South => 2,
        Dir::East => 4,
        Dir::West => 8,
        Dir::NorthEast => 5,
        Dir::SouthEast => 6,
        Dir::NorthWest => 9,
        Dir::SouthWest => 10,
    }
}

/// A big battery feeding its network.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Smes {
    pub charge: f64,
    pub capacity: f64,
    pub output: f64,
    pub online: bool,
}

/// A generator feeding its network, W this cycle. Unlike a SMES it stores nothing: what
/// isn't used is lost (Luna `AddPower`).
#[derive(Component, Debug, Default, Clone, Copy, PartialEq)]
pub struct PowerSupply {
    pub watts: f64,
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
    /// The interface is locked; an ID card with access unlocks it (Luna `locked`).
    pub locked: bool,
    /// The cover is locked shut (Luna `coverlocked`), set from the interface.
    pub cover_locked: bool,
    /// The cover is open: the cell can be taken out or put in (Luna `opened`).
    pub opened: bool,
    /// The cell charges from the grid's spare power (Luna `chargemode`).
    pub charge_auto: bool,
    /// Grid power last cycle: 0 none, 1 low, 2 good (Luna `main_status`).
    pub main_status: u8,
    /// 0 not charging, 1 charging, 2 full (Luna `charging`).
    pub charging: u8,
    /// Each channel's load last cycle, W.
    pub last_used: [f64; 3],
}

impl Apc {
    pub fn new(area: impl Into<String>) -> Self {
        Self {
            area: area.into(),
            cell: Some((APC_CELL_CAPACITY * APC_START_CHARGE, APC_CELL_CAPACITY)),
            operating: true,
            channels: [ChannelMode::AutoOn; 3],
            locked: true,
            cover_locked: true,
            opened: false,
            charge_auto: true,
            main_status: 0,
            charging: 0,
            last_used: [0.0; 3],
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

/// Power networks: which cables and machines connect to which.
#[derive(Resource, Debug, Default)]
pub struct Powernets {
    /// The network of each cable and connected machine.
    pub net_of: HashMap<Entity, usize>,
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

/// What the grid offered and what was drawn from it last cycle, W (Luna's
/// `TotalSupply` and `OldDraw`, for power monitors).
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq)]
pub struct GridTotals {
    pub supply: f64,
    pub load: f64,
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

/// Rebuild networks when cables or machines appear, go or change. Networks follow the
/// cables' ends (Luna's unified cable network): a cable end joins the cables on the tile it
/// points to whose end points back, and the machines on that tile. A knot joins knots and
/// machines on its own tile. Machines never pass power between cables, and each belongs to
/// at most one network.
pub fn rebuild_powernets(
    mut nets: ResMut<Powernets>,
    added: Query<(), Added<PowerNode>>,
    changed: Query<(), Changed<Cable>>,
    mut removed: RemovedComponents<PowerNode>,
    cables: Query<(Entity, &GridPos, &Cable)>,
    machines: Query<(Entity, &GridPos), (With<PowerNode>, Without<Cable>)>,
) {
    let removed = removed.read().count() > 0;
    if !nets.dirty && added.is_empty() && changed.is_empty() && !removed {
        return;
    }
    let mut cables_at: BTreeMap<TilePos, Vec<(Entity, Cable)>> = BTreeMap::new();
    for (entity, pos, cable) in &cables {
        cables_at.entry(pos.0).or_default().push((entity, *cable));
    }
    for list in cables_at.values_mut() {
        list.sort_by_key(|(e, _)| *e);
    }
    let mut machines_at: HashMap<TilePos, Vec<Entity>> = HashMap::new();
    for (entity, pos) in &machines {
        machines_at.entry(pos.0).or_default().push(entity);
    }
    for list in machines_at.values_mut() {
        list.sort();
    }

    nets.net_of.clear();
    let mut count = 0;
    for (start_pos, list) in &cables_at {
        for (start, _) in list {
            if nets.net_of.contains_key(start) {
                continue;
            }
            let id = count;
            count += 1;
            nets.net_of.insert(*start, id);
            let mut queue = VecDeque::from([(*start_pos, *start)]);
            while let Some((pos, entity)) = queue.pop_front() {
                let Some(cable) = cables_at[&pos]
                    .iter()
                    .find(|(e, _)| *e == entity)
                    .map(|(_, c)| *c)
                else {
                    continue;
                };
                for end in cable.ends() {
                    let there = end.map_or(pos, |d| pos.step(d));
                    let back = end.map(Dir::opposite);
                    for (other, other_cable) in cables_at.get(&there).into_iter().flatten() {
                        if other_cable.has_end(back) && !nets.net_of.contains_key(other) {
                            nets.net_of.insert(*other, id);
                            queue.push_back((there, *other));
                        }
                    }
                    for machine in machines_at.get(&there).into_iter().flatten() {
                        nets.net_of.entry(*machine).or_insert(id);
                    }
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
    mut totals: ResMut<GridTotals>,
    mut users: Query<(&GridPos, &mut PowerUser, Option<&Switch>, Option<&mut Airlock>)>,
    mut apcs: Query<(Entity, &mut Apc)>,
    mut batteries: Query<(Entity, &mut Smes)>,
    generators: Query<(Entity, &PowerSupply)>,
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

    // What each network's generators and SMES units can give this cycle. Generators are
    // drawn on first, since what they make is lost if unused.
    let mut generated = vec![0.0; nets.count];
    for (entity, supply) in &generators {
        if let Some(&net) = nets.net_of.get(&entity) {
            generated[net] += supply.watts * POWER_DT;
        }
    }
    let mut stored = vec![0.0; nets.count];
    for (entity, smes) in &batteries {
        if let (true, Some(&net)) = (smes.online, nets.net_of.get(&entity)) {
            stored[net] += smes.charge.min(smes.output * POWER_DT);
        }
    }
    let offered: Vec<f64> = generated.iter().zip(&stored).map(|(g, s)| g + s).collect();
    let mut remaining = offered.clone();

    let mut order: Vec<Entity> = apcs.iter().map(|(e, ..)| e).collect();
    order.sort();
    for entity in order {
        let Ok((_, mut apc)) = apcs.get_mut(entity) else {
            continue;
        };
        let state = areas.0.entry(apc.area.clone()).or_default();
        let usage: f64 = state.used.iter().sum();
        apc.last_used = state.used.map(|j| j / POWER_DT);
        state.used = [0.0; 3];
        let grid = nets.net_of.get(&entity).map(|n| &mut remaining[*n]);
        let mut spare = grid.as_deref().copied().unwrap_or(0.0);
        apc.main_status = if spare <= 0.0 {
            0
        } else if spare < usage {
            1
        } else {
            2
        };

        match apc.cell {
            Some((mut charge, capacity)) => {
                let trickle = if apc.charge_auto {
                    (capacity * APC_TRICKLE).min(capacity - charge)
                } else {
                    0.0
                };
                let mut can_cover = true;
                let mut charged = false;
                if spare >= usage + trickle {
                    spare -= usage + trickle;
                    charge += trickle;
                    charged = trickle > 0.0;
                } else if spare + charge >= usage {
                    let from_grid = spare.min(usage);
                    spare -= from_grid;
                    charge -= usage - from_grid;
                } else {
                    can_cover = false;
                }
                apc.cell = Some((charge, capacity));
                apc.charging = if charge >= capacity {
                    2
                } else if charged {
                    1
                } else {
                    0
                };
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
            None => {
                apc.charging = 0;
                apc.autoset_all([0, 0, 0]);
            }
        }
        if let Some(grid) = grid {
            *grid = spare;
        }
        let operating = apc.operating && apc.cell.is_some();
        state.on = apc.channels.map(|c| operating && c.is_on());
    }

    let supply: f64 = offered.iter().sum();
    let drawn: f64 = offered.iter().zip(&remaining).map(|(o, r)| o - r).sum();
    *totals = GridTotals {
        supply: supply / POWER_DT,
        load: drawn / POWER_DT,
    };

    // SMES units lose only what was drawn, in proportion to what they offered.
    for (entity, mut smes) in &mut batteries {
        let Some(&net) = nets.net_of.get(&entity) else {
            continue;
        };
        if !smes.online || stored[net] <= 0.0 {
            continue;
        }
        let share = smes.charge.min(smes.output * POWER_DT) / stored[net];
        let from_storage = (offered[net] - remaining[net] - generated[net]).max(0.0);
        let drawn = from_storage * share;
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

/// APC sprite: the cover open, with or without its cell; else off with the breaker open,
/// "low" once equipment has been cut, else on.
pub fn update_power_looks(mut apcs: Query<(&Apc, &mut rs13_world::Appearance), Changed<Apc>>) {
    for (apc, mut appearance) in &mut apcs {
        let state = if apc.opened {
            if apc.cell.is_some() { "open" } else { "open-empty" }
        } else if !apc.operating {
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
