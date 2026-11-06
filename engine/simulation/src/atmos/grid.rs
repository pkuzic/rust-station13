//! Zones of air on one deck, the openings between them, and gas flow through openings.
//! Design: `docs/ATMOSPHERICS.md`, `docs/adr/0006-hybrid-atmos.md`.

use std::collections::{BTreeMap, VecDeque};

use rs13_world::{Dir, TileMap, TilePos};

use super::gas::{Gas, GasMix, R, T_SPACE, TILE_VOLUME, VACUUM_MOLES};

mod fire_cells;
pub use fire_cells::FireCell;

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
/// The coldest water leaves burning air, K: Luna allows 0, and the cosmic background is
/// the floor everywhere else here.
pub const DOUSED_FLOOR: f64 = T_SPACE;

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
    /// Plasma burnt last step, mol/s: the zone is on fire while this is above zero
    /// (`fire.rs`).
    pub burn_rate: f64,
    /// Wall, floor and ceiling area its air touches, m²: where it gains or loses heat.
    pub surface: f64,
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

/// Where one deck's cells sit among a grid's: its size and first index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DeckSpan {
    z: i32,
    width: i32,
    height: i32,
    first: usize,
}

impl DeckSpan {
    fn len(&self) -> usize {
        (self.width * self.height) as usize
    }
}

/// The air of a station: one or more decks. Zones never cross decks.
#[derive(Debug, Clone)]
pub struct AtmosGrid {
    decks: Vec<DeckSpan>,
    cells: Vec<Cell>,
    portal_open: Vec<bool>,
    /// Edge barriers (firedoors) between two neighbouring tiles, by their indices (lower
    /// first): open or closed.
    edges: BTreeMap<(usize, usize), bool>,
    zone_of: Vec<Option<usize>>,
    zones: Vec<Zone>,
    openings: Vec<Opening>,
    wind: Vec<Option<Wind>>,
    /// Tiles on fire with air of their own, by tile index (`fire_cells.rs`).
    cells_on_fire: BTreeMap<usize, FireCell>,
    /// Tiles with no floor (`open-space`): air passes between them and the tile below.
    open_below: Vec<bool>,
    zones_dirty: bool,
    openings_dirty: bool,
    pub flow_scale: f64,
}

impl AtmosGrid {
    /// Build from a map. Space tiles are `space`; airtight tiles are walls (a grille is
    /// solid but lets air through: `TileDef::is_airtight`); `portals` are door
    /// tiles (closed until told otherwise). Every zone starts with standard air.
    pub fn new(map: &TileMap, portals: &[TilePos]) -> Self {
        Self::new_decks(&[map], portals)
    }

    /// Build from every deck of a station, as [`Self::new`] does for one. Decks must have
    /// different `z`.
    pub fn new_decks(maps: &[&TileMap], portals: &[TilePos]) -> Self {
        let mut cells = Vec::new();
        let mut open_below = Vec::new();
        let mut decks = Vec::new();
        for map in maps {
            let (width, height) = (map.width as i32, map.height as i32);
            decks.push(DeckSpan {
                z: map.z,
                width,
                height,
                first: cells.len(),
            });
            for y in 0..height {
                for x in 0..width {
                    let pos = TilePos::new(x, y, map.z);
                    open_below.push(map.tile(pos).is_some_and(|t| t.id == "open-space"));
                    cells.push(match map.tile(pos) {
                        Some(t) if t.id == "space" => Cell::Space,
                        Some(t) if t.is_airtight() => Cell::Solid,
                        Some(_) => Cell::Open,
                        None => Cell::Solid,
                    });
                }
            }
        }
        let mut grid = AtmosGrid {
            decks,
            portal_open: vec![false; cells.len()],
            edges: BTreeMap::new(),
            zone_of: vec![None; cells.len()],
            cells,
            zones: Vec::new(),
            openings: Vec::new(),
            wind: Vec::new(),
            cells_on_fire: BTreeMap::new(),
            open_below,
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
        let deck = self.decks.iter().find(|d| d.z == pos.z)?;
        if pos.x < 0 || pos.y < 0 || pos.x >= deck.width || pos.y >= deck.height {
            return None;
        }
        Some(deck.first + (pos.y * deck.width + pos.x) as usize)
    }

    fn pos(&self, index: usize) -> TilePos {
        let deck = self
            .decks
            .iter()
            .find(|d| (d.first..d.first + d.len()).contains(&index))
            .expect("an index of this grid");
        let i = (index - deck.first) as i32;
        TilePos::new(i % deck.width, i / deck.width, deck.z)
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

    /// Open or close a hole in the floor of `pos` to the tile below (`open-space` built over,
    /// or broken open). Zones are rebuilt on the next step.
    pub fn set_open_below(&mut self, pos: TilePos, open: bool) {
        if let Some(i) = self.index(pos)
            && self.open_below[i] != open
        {
            self.open_below[i] = open;
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

    fn edge_key(&self, at: TilePos, dir: Dir) -> Option<(usize, usize)> {
        let (a, b) = (self.index(at)?, self.index(at.step(dir))?);
        Some((a.min(b), a.max(b)))
    }

    /// Put a barrier on the edge between `at` and the tile in `dir`, such as a firedoor
    /// that seals one side of its tile. It starts open. Zones never extend across an edge
    /// barrier; while it is open, gas flows through it as through a doorway.
    pub fn add_edge(&mut self, at: TilePos, dir: Dir) {
        if let Some(key) = self.edge_key(at, dir)
            && self.edges.insert(key, true).is_none()
        {
            self.zones_dirty = true;
        }
    }

    /// Open or close an edge barrier added with `add_edge`.
    pub fn set_edge_open(&mut self, at: TilePos, dir: Dir, open: bool) {
        if let Some(key) = self.edge_key(at, dir)
            && let Some(state) = self.edges.get_mut(&key)
            && *state != open
        {
            *state = open;
            self.openings_dirty = true;
        }
    }

    /// The edge barrier between two cells, if any: `Some(open)`.
    fn edge(&self, i: usize, j: usize) -> Option<bool> {
        self.edges.get(&(i.min(j), i.max(j))).copied()
    }

    /// Whether a closed edge barrier lies between two cells.
    fn edge_closed(&self, i: usize, j: usize) -> bool {
        self.edge(i, j) == Some(false)
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

    /// The air one tile holds: its fire cell's, or its share of its zone. Vacuum for space
    /// and walls.
    pub fn air_at(&self, pos: TilePos) -> GasMix {
        if let Some(cell) = self.index(pos).and_then(|i| self.cells_on_fire.get(&i)) {
            return cell.mix;
        }
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

    /// Take `moles` of gas from a tile's fire cell or zone, in proportion to its composition.
    pub fn remove_at(&mut self, pos: TilePos, moles: f64) -> GasMix {
        if let Some(cell) = self.index(pos).and_then(|i| self.cells_on_fire.get_mut(&i)) {
            return cell.mix.remove_moles(moles);
        }
        match self.air_zone(pos) {
            Some(z) => self.zones[z].mix.remove_moles(moles),
            None => GasMix::default(),
        }
    }

    /// Release gas at a tile: into its fire cell or zone. Into space or a wall, it is lost.
    pub fn add_at(&mut self, pos: TilePos, gas: &GasMix) {
        if let Some(cell) = self.index(pos).and_then(|i| self.cells_on_fire.get_mut(&i)) {
            cell.mix.merge(gas);
            return;
        }
        if let Some(z) = self.air_zone(pos) {
            self.zones[z].mix.merge(gas);
        }
    }

    /// Take a `fraction` of only the listed gases from a tile's share of its zone.
    pub fn remove_gases_at(&mut self, pos: TilePos, gases: &[Gas], fraction: f64) -> GasMix {
        let Some(z) = self.air_zone(pos) else {
            return GasMix::default();
        };
        let zone = &mut self.zones[z];
        let share = (TILE_VOLUME / zone.volume) * fraction.clamp(0.0, 1.0);
        let mut out = GasMix {
            temperature: zone.mix.temperature,
            ..GasMix::default()
        };
        for gas in gases {
            let have = zone.mix.get(*gas);
            let taken = have * share;
            zone.mix.set(*gas, have - taken);
            out.set(*gas, taken);
        }
        out
    }

    /// Whether the zone a tile belongs to is losing gas to space right now (a breach or an
    /// open airlock). Air alarms use this to lock vents out instead of feeding the leak.
    pub fn venting_to_space(&self, pos: TilePos) -> bool {
        let Some(z) = self.air_zone(pos) else { return false };
        if self.zone_leaks_to_space(z) {
            return true;
        }
        // One door away from a leaking zone, with gas flowing between them.
        self.openings.iter().any(|o| {
            let neighbour = match o.other {
                Side::Zone(other) if o.zone == z => Some(other),
                Side::Zone(other) if other == z => Some(o.zone),
                _ => None,
            };
            o.flow != 0.0 && neighbour.is_some_and(|n| self.zone_leaks_to_space(n))
        })
    }

    fn zone_leaks_to_space(&self, z: usize) -> bool {
        self.openings
            .iter()
            .any(|o| o.zone == z && o.other == Side::Space && o.flow > 0.0)
    }

    pub fn wind_at(&self, pos: TilePos) -> Option<Wind> {
        self.index(pos).and_then(|i| self.wind.get(i).copied().flatten())
    }

    /// Total moles in all zones and fire cells (for conservation checks).
    pub fn total_moles(&self) -> f64 {
        let cells: f64 = self.cells_on_fire.values().map(|c| c.mix.total_moles()).sum();
        self.zones.iter().map(|z| z.mix.total_moles()).sum::<f64>() + cells
    }

    /// Total thermal energy in all zones and fire cells, J.
    pub fn total_energy(&self) -> f64 {
        let cells: f64 = self.cells_on_fire.values().map(|c| c.mix.thermal_energy()).sum();
        self.zones.iter().map(|z| z.mix.thermal_energy()).sum::<f64>() + cells
    }

    /// A spark or flame at `temperature` K touches the air on `pos` (Luna `hotspot_expose`
    /// and `obj/fire/New`). If it is hotter than
    /// [`PLASMA_BURN_TEMPERATURE`](super::fire::PLASMA_BURN_TEMPERATURE), the tile's air
    /// holds more than [`IGNITION_MOLES`](super::fire::IGNITION_MOLES) of plasma and of
    /// oxygen, and it isn't burning already, a fire starts there: the tile becomes a fire
    /// cell (`fire_cells.rs`) and, as Luna adds the flame's temperature to the tile's air,
    /// its air warms by `temperature`. In a zone's last tile the zone itself warms instead.
    /// Returns how much warmer that air is, K (0 when nothing lit).
    pub fn expose(&mut self, pos: TilePos, temperature: f64) -> f64 {
        use super::fire::{MIN_BURN, PLASMA_BURN_TEMPERATURE};
        let (Some(i), Some(z)) = (self.index(pos), self.zone_at(pos)) else {
            return 0.0;
        };
        if temperature <= PLASMA_BURN_TEMPERATURE || self.zones[z].burn_rate > 0.0 {
            return 0.0;
        }
        if let Some(cell) = self.cells_on_fire.get_mut(&i) {
            if cell.burn_rate > MIN_BURN || !Self::can_catch(&cell.mix) {
                return 0.0;
            }
            cell.mix.temperature += temperature;
            return temperature;
        }
        let zone = &self.zones[z];
        if zone.volume <= 0.0 || !Self::can_catch(&zone.mix.scaled((TILE_VOLUME / zone.volume).min(1.0))) {
            return 0.0;
        }
        if let Some(cell) = self.open_cell(i, z) {
            cell.mix.temperature += temperature;
            return temperature;
        }
        // A one-tile zone: the zone is the tile.
        let zone = &mut self.zones[z];
        zone.mix.temperature += temperature;
        temperature
    }

    /// Water on `pos` (Luna's water reagent on a hotspot): if the air there is burning, it
    /// drops to max(min(T − 2000, T / 2), [`DOUSED_FLOOR`]) K and stops burning. A tile of
    /// a zone burning as a whole becomes a fire cell first, so only that tile is cooled; in
    /// a zone's last tile the zone itself is. Returns whether a fire was put out.
    pub fn douse(&mut self, pos: TilePos) -> bool {
        fn cooled(t: f64) -> f64 {
            (t - 2000.0).min(t / 2.0).max(DOUSED_FLOOR)
        }
        if !self.burning_at(pos) {
            return false;
        }
        let (Some(i), Some(z)) = (self.index(pos), self.zone_at(pos)) else {
            return false;
        };
        if !self.cells_on_fire.contains_key(&i) {
            self.open_cell(i, z);
        }
        if let Some(cell) = self.cells_on_fire.get_mut(&i) {
            cell.mix.temperature = cooled(cell.mix.temperature);
            cell.burn_rate = 0.0;
            cell.burning_for = 0.0;
            return true;
        }
        let zone = &mut self.zones[z];
        zone.mix.temperature = cooled(zone.mix.temperature);
        zone.burn_rate = 0.0;
        true
    }

    /// Air trades heat with its zone's walls, floor and ceiling for `dt` seconds: hot rooms
    /// cool, cold ones warm, toward [`WALL_TEMPERATURE`]. Kept apart from [`Self::step`] so the
    /// flow itself conserves energy exactly.
    pub fn exchange_heat(&mut self, dt: f64) {
        for zone in &mut self.zones {
            exchange_heat_with_walls(zone, dt);
        }
    }

    /// Whether the air on `pos` is burning: its fire cell if it is one, else its whole zone.
    pub fn burning_at(&self, pos: TilePos) -> bool {
        match self.index(pos).and_then(|i| self.cells_on_fire.get(&i)) {
            Some(cell) => cell.burn_rate > super::fire::MIN_BURN,
            None => self.zone_at(pos).is_some_and(|z| self.zones[z].burn_rate > 0.0),
        }
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
        for zone in &mut self.zones {
            zone.burn_rate = zone.mix.burn(dt) / dt;
        }
        self.step_cells(dt);
        self.update_wind();
    }

    /// Flood-fill zones over open tiles, redistributing existing gas: every old zone's gas
    /// is split among the new zones in proportion to how many of its tiles each received.
    fn rebuild_zones(&mut self) {
        // Fire cells go back into their zones first: the tiles may now belong elsewhere.
        self.close_all_cells();
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
                        && self.edge(i, j).is_none()
                    {
                        self.zone_of[j] = Some(id);
                        queue.push_back(j);
                    }
                }
                // Through a hole in this floor to the deck below, or in the ceiling above
                // (Luna's open floor shares its zone with the tile under it).
                let below = TilePos::new(pos.x, pos.y, pos.z - 1);
                let above = TilePos::new(pos.x, pos.y, pos.z + 1);
                let vertical = [
                    self.index(below).filter(|_| self.open_below[i]),
                    self.index(above).filter(|&k| self.open_below[k]),
                ];
                for j in vertical.into_iter().flatten() {
                    if self.cells[j] == Cell::Open && self.zone_of[j].is_none() {
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
                burn_rate: 0.0,
                surface: 0.0,
            });
        }

        // Each zone's surface: a wall face for every side that leaves the zone, and the floor
        // and ceiling of every tile.
        for z in 0..self.zones.len() {
            let mut faces = 0usize;
            for pos in &self.zones[z].tiles {
                for d in [Dir::North, Dir::East, Dir::South, Dir::West] {
                    let inside = self
                        .index(pos.step(d))
                        .is_some_and(|j| self.zone_of[j] == Some(z));
                    if !inside {
                        faces += 1;
                    }
                }
            }
            let tiles = self.zones[z].tiles.len();
            self.zones[z].surface = faces as f64 * WALL_FACE_AREA + tiles as f64 * 2.0 * FLOOR_AREA;
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
                    let sealed = match (self.index(*pos), self.index(next)) {
                        (Some(i), Some(j)) => self.edge_closed(i, j),
                        _ => false,
                    };
                    if self.cell(next) == Cell::Space && !sealed {
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
                    if self.edge_closed(i, j) {
                        continue;
                    }
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
        // Open edge barriers between two zones: a doorway on the edge.
        for (&(i, j), &open) in &self.edges {
            if !open || self.cells[i] != Cell::Open || self.cells[j] != Cell::Open {
                continue;
            }
            let (Some(a), Some(b)) = (self.zone_of[i], self.zone_of[j]) else {
                continue;
            };
            if a != b {
                openings.push(Opening {
                    zone: a,
                    other: Side::Zone(b),
                    at: self.pos(i),
                    through: self.pos(j),
                    area: DOOR_AREA,
                    flow: 0.0,
                });
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
            } else if self.zone_at(opening.through) == Some(source) {
                // Back across an open edge barrier.
                (opening.through, opening.at)
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

/// Area of one wall face of a tile, m² (1 m wide, 2.5 m high).
pub const WALL_FACE_AREA: f64 = 2.5;
/// Area of a tile's floor, and of its ceiling, m².
pub const FLOOR_AREA: f64 = 1.0;
/// How readily air and the station's structure trade heat, W/(m²·K): slow natural
/// convection.
pub const WALL_HEAT_TRANSFER: f64 = 5.0;
/// The structure's temperature. Walls are treated as a heat sink that never warms up
/// until materials get their own temperature (`docs/ATMOSPHERICS.md`, P1).
pub const WALL_TEMPERATURE: f64 = super::gas::T20C;

/// Air trades heat with the zone's walls, floor and ceiling (Newton's law of cooling),
/// never overshooting the walls' temperature.
fn exchange_heat_with_walls(zone: &mut Zone, dt: f64) {
    let capacity = zone.mix.heat_capacity();
    if capacity <= 0.0 || zone.mix.is_vacuum() {
        return;
    }
    let difference = zone.mix.temperature - WALL_TEMPERATURE;
    let fraction = (WALL_HEAT_TRANSFER * zone.surface * dt / capacity).min(1.0);
    zone.mix.temperature -= difference * fraction;
}
