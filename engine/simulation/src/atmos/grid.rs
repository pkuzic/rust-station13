//! Zones of air on one deck, the openings between them, and gas flow through openings.
//! Design: `docs/ATMOSPHERICS.md`, `docs/adr/0006-hybrid-atmos.md`.

use std::collections::{BTreeMap, VecDeque};

use rs13_world::{Dir, TileMap, TilePos};

use super::gas::{GasMix, R, T_SPACE, TILE_VOLUME, VACUUM_MOLES};

/// Open area of a doorway, m² (1 m × 2 m).
pub const DOOR_AREA: f64 = 2.0;
/// Open area of one breached tile face, m² (1 m × 2.5 m).
pub const BREACH_AREA: f64 = 2.5;
/// Discharge coefficient of a sharp-edged opening.
pub const DISCHARGE_COEFFICIENT: f64 = 0.6;
/// Gameplay scale on physical flow rates. Real decompression empties a room in about a
/// second; this keeps it violent but gives players a few seconds to react. Tune here.
pub const DEFAULT_FLOW_SCALE: f64 = 0.05;
/// Openings with a smaller pressure difference are skipped (sleeping regions).
pub const MIN_PRESSURE_DELTA: f64 = 0.001;

/// What a tile is, for air.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    /// Vacuum, an infinite sink.
    Space,
    /// Walls: no air, no passage.
    Solid,
    /// Floor: part of a zone.
    Open,
    /// A door tile: never part of a zone; when open it connects the sides around it.
    Portal,
}

/// One side of an opening.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Side {
    Zone(usize),
    Space,
}

/// A connection through which gas can flow between a zone and another zone or space.
#[derive(Debug, Clone, PartialEq)]
pub struct Opening {
    pub zone: usize,
    pub other: Side,
    /// The zone-side tile next to the opening.
    pub at: TilePos,
    /// The tile the gas passes into: a portal or a space tile.
    pub through: TilePos,
    pub area: f64,
    /// Flow in the last step, mol/s; positive from `zone` towards `other`.
    pub flow: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Zone {
    pub tiles: Vec<TilePos>,
    /// Litres.
    pub volume: f64,
    pub mix: GasMix,
}

impl Zone {
    pub fn pressure(&self) -> f64 {
        self.mix.pressure(self.volume)
    }
}

/// Gas wind at a tile: which way it blows and how fast, m/s.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wind {
    pub dir: Dir,
    pub speed: f64,
}

/// The air of one deck.
#[derive(Debug, Clone)]
pub struct AtmosGrid {
    width: i32,
    height: i32,
    z: i32,
    cells: Vec<Cell>,
    portal_open: Vec<bool>,
    zone_of: Vec<Option<usize>>,
    zones: Vec<Zone>,
    openings: Vec<Opening>,
    wind: Vec<Option<Wind>>,
    zones_dirty: bool,
    openings_dirty: bool,
    pub flow_scale: f64,
}

impl AtmosGrid {
    /// Build from a map. Space tiles are `space`; solid tiles are walls; `portals` are door
    /// tiles (closed until told otherwise). Every zone starts with standard air.
    pub fn new(map: &TileMap, portals: &[TilePos]) -> Self {
        let (width, height) = (map.width as i32, map.height as i32);
        let mut cells = Vec::with_capacity((width * height) as usize);
        for y in 0..height {
            for x in 0..width {
                let pos = TilePos::new(x, y, map.z);
                let tile = map.tile(pos);
                cells.push(match tile {
                    Some(t) if t.id == "space" => Cell::Space,
                    Some(t) if t.solid => Cell::Solid,
                    Some(_) => Cell::Open,
                    None => Cell::Solid,
                });
            }
        }
        let mut grid = AtmosGrid {
            width,
            height,
            z: map.z,
            portal_open: vec![false; cells.len()],
            zone_of: vec![None; cells.len()],
            cells,
            zones: Vec::new(),
            openings: Vec::new(),
            wind: Vec::new(),
            zones_dirty: true,
            openings_dirty: true,
            flow_scale: DEFAULT_FLOW_SCALE,
        };
        for p in portals {
            if let Some(i) = grid.index(*p) {
                grid.cells[i] = Cell::Portal;
            }
        }
        grid.rebuild_zones();
        for zone in &mut grid.zones {
            zone.mix = GasMix::standard_air(zone.volume);
        }
        grid
    }

    fn index(&self, pos: TilePos) -> Option<usize> {
        if pos.z != self.z || pos.x < 0 || pos.y < 0 || pos.x >= self.width || pos.y >= self.height {
            return None;
        }
        Some((pos.y * self.width + pos.x) as usize)
    }

    fn pos(&self, index: usize) -> TilePos {
        let i = index as i32;
        TilePos::new(i % self.width, i / self.width, self.z)
    }

    /// Outside the grid counts as solid.
    pub fn cell(&self, pos: TilePos) -> Cell {
        self.index(pos).map_or(Cell::Solid, |i| self.cells[i])
    }

    pub fn zones(&self) -> &[Zone] {
        &self.zones
    }

    pub fn openings(&self) -> &[Opening] {
        &self.openings
    }

    pub fn zone_at(&self, pos: TilePos) -> Option<usize> {
        self.index(pos).and_then(|i| self.zone_of[i])
    }

    /// Change a tile, e.g. a wall breached into floor. Gas is redistributed on the next step.
    pub fn set_cell(&mut self, pos: TilePos, cell: Cell) {
        if let Some(i) = self.index(pos)
            && self.cells[i] != cell
        {
            self.cells[i] = cell;
            self.zones_dirty = true;
        }
    }

    /// Open or close a door tile.
    pub fn set_portal_open(&mut self, pos: TilePos, open: bool) {
        if let Some(i) = self.index(pos)
            && self.portal_open[i] != open
        {
            self.portal_open[i] = open;
            self.openings_dirty = true;
        }
    }

    /// The zone a tile draws air from: its own, or for a door tile the first zone next to
    /// it (north, south, east, west).
    fn air_zone(&self, pos: TilePos) -> Option<usize> {
        match self.cell(pos) {
            Cell::Open => self.zone_at(pos),
            Cell::Portal => [Dir::North, Dir::South, Dir::East, Dir::West]
                .into_iter()
                .find_map(|d| self.zone_at(pos.step(d))),
            Cell::Space | Cell::Solid => None,
        }
    }

    /// The air one tile holds: its share of its zone. Vacuum for space and walls.
    pub fn air_at(&self, pos: TilePos) -> GasMix {
        match self.air_zone(pos) {
            Some(z) => {
                let zone = &self.zones[z];
                zone.mix.scaled(TILE_VOLUME / zone.volume)
            }
            None => GasMix::default(),
        }
    }

    /// Pressure at a tile, kPa.
    pub fn pressure_at(&self, pos: TilePos) -> f64 {
        self.air_at(pos).pressure(TILE_VOLUME)
    }

    /// Take `moles` of gas from a tile's zone, in proportion to its composition.
    pub fn remove_at(&mut self, pos: TilePos, moles: f64) -> GasMix {
        match self.air_zone(pos) {
            Some(z) => self.zones[z].mix.remove_moles(moles),
            None => GasMix::default(),
        }
    }

    /// Release gas at a tile. Into space or a wall, it is lost.
    pub fn add_at(&mut self, pos: TilePos, gas: &GasMix) {
        if let Some(z) = self.air_zone(pos) {
            self.zones[z].mix.merge(gas);
        }
    }

    pub fn wind_at(&self, pos: TilePos) -> Option<Wind> {
        self.index(pos).and_then(|i| self.wind.get(i).copied().flatten())
    }

    /// Total moles in all zones (for conservation checks).
    pub fn total_moles(&self) -> f64 {
        self.zones.iter().map(|z| z.mix.total_moles()).sum()
    }

    /// Total thermal energy in all zones, J.
    pub fn total_energy(&self) -> f64 {
        self.zones.iter().map(|z| z.mix.thermal_energy()).sum()
    }

    /// Advance the simulation by `dt` seconds.
    pub fn step(&mut self, dt: f64) {
        if self.zones_dirty {
            self.rebuild_zones();
        }
        if self.openings_dirty {
            self.rebuild_openings();
        }
        for i in 0..self.openings.len() {
            self.flow_through(i, dt);
        }
        self.update_wind();
    }

    /// Flood-fill zones over open tiles, redistributing existing gas: every old zone's gas
    /// is split among the new zones in proportion to how many of its tiles each received.
    fn rebuild_zones(&mut self) {
        let old_zone_of = std::mem::take(&mut self.zone_of);
        let old_zones = std::mem::take(&mut self.zones);
        self.zone_of = vec![None; self.cells.len()];

        for start in 0..self.cells.len() {
            if self.cells[start] != Cell::Open || self.zone_of[start].is_some() {
                continue;
            }
            let id = self.zones.len();
            let mut tiles = Vec::new();
            let mut queue = VecDeque::from([start]);
            self.zone_of[start] = Some(id);
            while let Some(i) = queue.pop_front() {
                let pos = self.pos(i);
                tiles.push(pos);
                for d in [Dir::North, Dir::East, Dir::South, Dir::West] {
                    if let Some(j) = self.index(pos.step(d))
                        && self.cells[j] == Cell::Open
                        && self.zone_of[j].is_none()
                    {
                        self.zone_of[j] = Some(id);
                        queue.push_back(j);
                    }
                }
            }
            let volume = tiles.len() as f64 * TILE_VOLUME;
            self.zones.push(Zone {
                tiles,
                volume,
                mix: GasMix::default(),
            });
        }

        // Conservative hand-over from the old zones.
        for (old_id, old) in old_zones.iter().enumerate() {
            let mut received: BTreeMap<usize, usize> = BTreeMap::new();
            for pos in &old.tiles {
                if let Some(i) = self.index(*pos)
                    && old_zone_of[i] == Some(old_id)
                    && let Some(new_id) = self.zone_of[i]
                {
                    *received.entry(new_id).or_default() += 1;
                }
            }
            let kept: usize = received.values().sum();
            if kept == 0 {
                continue;
            }
            for (new_id, count) in received {
                let share = old.mix.scaled(count as f64 / kept as f64);
                self.zones[new_id].mix.merge(&share);
            }
        }
        self.zones_dirty = false;
        self.openings_dirty = true;
    }

    /// Find every zone tile facing space, and every open door, and what they connect.
    fn rebuild_openings(&mut self) {
        let mut openings = Vec::new();
        // Breaches: zone tiles next to space.
        for (z, zone) in self.zones.iter().enumerate() {
            for pos in &zone.tiles {
                for d in [Dir::North, Dir::East, Dir::South, Dir::West] {
                    let next = pos.step(d);
                    if self.cell(next) == Cell::Space {
                        openings.push(Opening {
                            zone: z,
                            other: Side::Space,
                            at: *pos,
                            through: next,
                            area: BREACH_AREA,
                            flow: 0.0,
                        });
                    }
                }
            }
        }
        // Doors: an open portal (or a run of adjacent open portals) joins every side around it.
        let mut seen = vec![false; self.cells.len()];
        for start in 0..self.cells.len() {
            if self.cells[start] != Cell::Portal || !self.portal_open[start] || seen[start] {
                continue;
            }
            let mut sides: Vec<(Side, TilePos, TilePos)> = Vec::new();
            let mut queue = VecDeque::from([start]);
            seen[start] = true;
            while let Some(i) = queue.pop_front() {
                let pos = self.pos(i);
                for d in [Dir::North, Dir::East, Dir::South, Dir::West] {
                    let next = pos.step(d);
                    let Some(j) = self.index(next) else { continue };
                    match self.cells[j] {
                        Cell::Portal if self.portal_open[j] && !seen[j] => {
                            seen[j] = true;
                            queue.push_back(j);
                        }
                        Cell::Open => {
                            let side = Side::Zone(self.zone_of[j].expect("open tiles have zones"));
                            if !sides.iter().any(|(s, ..)| *s == side) {
                                sides.push((side, next, pos));
                            }
                        }
                        Cell::Space if !sides.iter().any(|(s, ..)| *s == Side::Space) => {
                            sides.push((Side::Space, next, pos));
                        }
                        _ => {}
                    }
                }
            }
            sides.sort_by_key(|(s, ..)| *s);
            for a in 0..sides.len() {
                let (Side::Zone(zone), at, portal) = sides[a] else {
                    continue;
                };
                for (other, other_at, _) in sides.iter().skip(a + 1) {
                    openings.push(Opening {
                        zone,
                        other: *other,
                        at,
                        // Gas passes through the door tile; towards space, on into space.
                        through: if *other == Side::Space { *other_at } else { portal },
                        area: DOOR_AREA,
                        flow: 0.0,
                    });
                }
            }
        }
        self.openings = openings;
        self.openings_dirty = false;
    }

    /// Move gas through one opening for `dt` seconds: isentropic orifice flow, choked when
    /// the downstream pressure is low enough. Moles are conserved (except into space) and
    /// so is energy: the upstream gas cools as it expands, and the downstream gas receives
    /// exactly the energy the upstream lost.
    fn flow_through(&mut self, index: usize, dt: f64) {
        let opening = self.openings[index].clone();
        let (p_zone, v_zone) = {
            let z = &self.zones[opening.zone];
            (z.pressure(), z.volume)
        };
        let (p_other, v_other) = match opening.other {
            Side::Zone(o) => (self.zones[o].pressure(), self.zones[o].volume),
            Side::Space => (0.0, f64::INFINITY),
        };
        self.openings[index].flow = 0.0;
        if (p_zone - p_other).abs() < MIN_PRESSURE_DELTA {
            return;
        }
        let forward = p_zone > p_other;
        let (up, down) = match (forward, opening.other) {
            (true, other) => (Side::Zone(opening.zone), other),
            (false, other) => (other, Side::Zone(opening.zone)),
        };
        let Side::Zone(up_id) = up else { return };
        let (p_up, p_down, v_up, v_down) = if forward {
            (p_zone, p_other, v_zone, v_other)
        } else {
            (p_other, p_zone, v_other, v_zone)
        };

        let mix = self.zones[up_id].mix;
        let n_up = mix.total_moles();
        if n_up < VACUUM_MOLES {
            return;
        }
        let rate = orifice_flow(&mix, p_up, p_down, opening.area);
        let mut moles = rate * dt * self.flow_scale;
        // Never more than equalises the two sides, and never more than there is.
        let t = mix.temperature.max(T_SPACE);
        let equalise = (p_up - p_down) / (R * t * (1.0 / v_up + 1.0 / v_down));
        moles = moles.min(equalise).min(n_up);
        if moles <= 0.0 {
            return;
        }

        let up_zone = &mut self.zones[up_id];
        let energy_before = up_zone.mix.thermal_energy();
        let gamma = up_zone.mix.gamma();
        let mut moved = up_zone.mix.remove_moles(moles);
        let n_after = up_zone.mix.total_moles();
        if n_after < VACUUM_MOLES {
            moved.merge(&up_zone.mix.remove_fraction(1.0));
            up_zone.mix.temperature = T_SPACE;
        } else {
            // Isentropic expansion of the gas left behind.
            up_zone.mix.temperature = t * (n_after / n_up).powf(gamma - 1.0);
        }
        let energy_out = energy_before - up_zone.mix.thermal_energy();
        let capacity = moved.heat_capacity();
        if capacity > 0.0 {
            moved.temperature = energy_out / capacity;
        }
        if let Side::Zone(down_id) = down {
            self.zones[down_id].mix.merge(&moved);
        }
        let moved_moles = moved.total_moles();
        self.openings[index].flow = if forward {
            moved_moles / dt
        } else {
            -moved_moles / dt
        };
    }

    /// Wind: for every zone losing gas through an opening, each of its tiles blows one step
    /// towards that opening along the shortest path, with a speed that falls off with
    /// distance. The strongest opening wins.
    fn update_wind(&mut self) {
        let mut wind: Vec<Option<Wind>> = vec![None; self.cells.len()];
        for opening in &self.openings {
            // Gas leaves `source` through this opening.
            let (source, outflow) = match opening.other {
                _ if opening.flow > 0.0 => (opening.zone, opening.flow),
                Side::Zone(o) if opening.flow < 0.0 => (o, -opening.flow),
                _ => continue,
            };
            let zone = &self.zones[source];
            let n = zone.mix.total_moles();
            if n < VACUUM_MOLES {
                continue;
            }
            // Gas speed through the opening: molar flow / (molar density × area). Reported
            // at physical scale: `flow_scale` slows how fast rooms drain, not how hard the
            // gas blows.
            let density = n / (zone.volume / 1000.0);
            let v0 = outflow / self.flow_scale / (density * opening.area);
            // Where the gas leaves this zone, and where it goes from there.
            let (exit, toward) = if source == opening.zone {
                (opening.at, opening.through)
            } else {
                // Flowing into `opening.zone` from the other zone through a door.
                let Some(exit) = [Dir::North, Dir::East, Dir::South, Dir::West]
                    .into_iter()
                    .map(|d| opening.through.step(d))
                    .find(|p| self.zone_at(*p) == Some(source))
                else {
                    continue;
                };
                (exit, opening.through)
            };
            let mut set = |pos: TilePos, dir: Dir, speed: f64| {
                if let Some(i) = self.index(pos)
                    && wind[i].is_none_or(|w| w.speed < speed)
                {
                    wind[i] = Some(Wind { dir, speed });
                }
            };
            // The exit tile, and any door tiles between it and where the gas goes, blow
            // straight through.
            if let Some(dir) = direction(exit, toward) {
                let mut pos = exit;
                while pos != toward {
                    set(pos, dir, v0);
                    pos = pos.step(dir);
                    if self.cell(pos) != Cell::Portal {
                        break;
                    }
                }
            }
            // Everything else in the zone blows down the distance gradient.
            let mut dist: BTreeMap<(i32, i32), u32> = BTreeMap::new();
            dist.insert((exit.x, exit.y), 0);
            let mut queue = VecDeque::from([exit]);
            while let Some(pos) = queue.pop_front() {
                let d = dist[&(pos.x, pos.y)];
                for dir in [Dir::North, Dir::East, Dir::South, Dir::West] {
                    let next = pos.step(dir);
                    if self.zone_at(next) == Some(source) && !dist.contains_key(&(next.x, next.y)) {
                        dist.insert((next.x, next.y), d + 1);
                        queue.push_back(next);
                        let back = match dir {
                            Dir::North => Dir::South,
                            Dir::South => Dir::North,
                            Dir::East => Dir::West,
                            _ => Dir::East,
                        };
                        set(next, back, v0 / (1.0 + f64::from(d + 1)));
                    }
                }
            }
        }
        self.wind = wind;
    }
}

fn direction(from: TilePos, to: TilePos) -> Option<Dir> {
    Dir::from_offset(to.x - from.x, to.y - from.y)
}

/// Molar flow through an orifice of `area` m², mol/s, from `p_up` to `p_down` kPa.
fn orifice_flow(mix: &GasMix, p_up: f64, p_down: f64, area: f64) -> f64 {
    let gamma = mix.gamma();
    let molar_mass = mix.molar_mass();
    let t = mix.temperature.max(T_SPACE);
    let p0 = p_up * 1000.0; // Pa
    let r = (p_down / p_up).clamp(0.0, 1.0);
    let critical = (2.0 / (gamma + 1.0)).powf(gamma / (gamma - 1.0));
    let core = if r <= critical {
        // Choked: the flow no longer depends on the downstream pressure.
        (gamma / (molar_mass * R * t)).sqrt()
            * (2.0 / (gamma + 1.0)).powf((gamma + 1.0) / (2.0 * (gamma - 1.0)))
    } else {
        let term = r.powf(2.0 / gamma) - r.powf((gamma + 1.0) / gamma);
        (2.0 * gamma / ((gamma - 1.0) * molar_mass * R * t) * term.max(0.0)).sqrt()
    };
    DISCHARGE_COEFFICIENT * area * p0 * core
}
