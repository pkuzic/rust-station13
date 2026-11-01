//! Fire cells: where a fire starts, the tile becomes a cell with its own air, taken out of
//! its zone, so a fire can start in one corner of any room and spread across it tile by
//! tile (`docs/ATMOSPHERICS.md`, active local cells; `docs/reference/fire.md`).
//!
//! - A cell holds one tile's air. Its zone's volume shrinks by a tile while it exists, so
//!   gas is conserved exactly.
//! - Every step a cell burns its own air with Luna's reaction, and trades gas and heat with
//!   each neighbouring cell and with its zone through each side that faces the zone's
//!   tiles, never across walls, closed doors or panes.
//! - A burning cell lights every neighbouring tile of open floor whose air has enough
//!   plasma and oxygen, once it has burnt for [`SPREAD_AFTER`]; that tile's air warms by
//!   [`SPREAD_HEAT`], as Luna's new flame warms its tile.
//! - A cell that has stopped burning and come near its zone's temperature goes back into
//!   the zone, and so does every cell of a zone venting to space: the rush of air out of
//!   the room takes the tile's air with it.

use std::collections::BTreeMap;

use rs13_world::{Dir, TilePos};

use super::{AtmosGrid, Cell};
use crate::atmos::fire::{IGNITION_MOLES, MIN_BURN, PLASMA_BURN_TEMPERATURE};
use crate::atmos::gas::{Gas, GasMix, T0C, TILE_VOLUME};

/// How fast neighbouring cells, or a cell and the zone beside it, even out their gas and
/// heat, per second and per side they share.
pub const CELL_MIXING: f64 = 0.4;
/// How long a cell burns before it lights its neighbours, s.
pub const SPREAD_AFTER: f64 = 0.5;
/// How much a tile the fire spreads to warms, K: Luna's new `obj/fire` adds its
/// `FIRE_MINIMUM_TEMPERATURE_TO_EXIST` (100 °C) to the tile's air.
pub const SPREAD_HEAT: f64 = T0C + 100.0;
/// A cell goes back into its zone once it has not burnt for this long, s, and is within
/// [`SETTLE_DELTA`] of the zone's temperature (or has not burnt for [`SETTLE_MAX`]).
pub const SETTLE_AFTER: f64 = 2.0;
pub const SETTLE_DELTA: f64 = 30.0;
pub const SETTLE_MAX: f64 = 30.0;

/// A tile on fire, or cooling from one, with its own air.
#[derive(Debug, Clone, PartialEq)]
pub struct FireCell {
    pub pos: TilePos,
    pub zone: usize,
    pub mix: GasMix,
    /// Plasma burnt last step, mol/s.
    pub burn_rate: f64,
    /// How long it has been burning, s.
    pub burning_for: f64,
    /// How long since it last burnt, s.
    pub idle_for: f64,
}

/// Gas and heat between two parcels of one tile each: each gives the other `fraction` of
/// itself.
fn mix_parcels(a: &mut GasMix, b: &mut GasMix, fraction: f64) {
    let from_a = a.remove_fraction(fraction);
    let from_b = b.remove_fraction(fraction);
    a.merge(&from_b);
    b.merge(&from_a);
}

impl AtmosGrid {
    /// The fire cells, by tile index.
    pub fn fire_cells(&self) -> impl Iterator<Item = &FireCell> {
        self.cells_on_fire.values()
    }

    /// Make the tile at `index`, in zone `z`, a fire cell holding its share of the zone's
    /// air. `None` if it already is one or is the zone's last tile.
    pub(super) fn open_cell(&mut self, index: usize, z: usize) -> Option<&mut FireCell> {
        if self.cells_on_fire.contains_key(&index) || self.zones[z].volume <= TILE_VOLUME {
            return None;
        }
        let zone = &mut self.zones[z];
        let mix = zone.mix.remove_fraction(TILE_VOLUME / zone.volume);
        zone.volume -= TILE_VOLUME;
        let pos = self.pos(index);
        Some(self.cells_on_fire.entry(index).or_insert(FireCell {
            pos,
            zone: z,
            mix,
            burn_rate: 0.0,
            burning_for: 0.0,
            idle_for: 0.0,
        }))
    }

    /// Put every cell's air back into its zone (before zones are rebuilt).
    pub(super) fn close_all_cells(&mut self) {
        for (_, cell) in std::mem::take(&mut self.cells_on_fire) {
            if let Some(zone) = self.zones.get_mut(cell.zone) {
                zone.mix.merge(&cell.mix);
                zone.volume += TILE_VOLUME;
            }
        }
    }

    /// Whether a tile's air holds enough plasma and oxygen for a fire to start in it.
    pub(super) fn can_catch(mix: &GasMix) -> bool {
        mix.get(Gas::Plasma) > IGNITION_MOLES && mix.get(Gas::Oxygen) > IGNITION_MOLES
    }

    /// One step of every fire cell: burn, trade with neighbours and zones, spread, settle.
    pub(super) fn step_cells(&mut self, dt: f64) {
        if self.cells_on_fire.is_empty() {
            return;
        }
        let mixing = 1.0 - (-CELL_MIXING * dt).exp();
        let indices: Vec<usize> = self.cells_on_fire.keys().copied().collect();

        // Burn.
        for cell in self.cells_on_fire.values_mut() {
            cell.burn_rate = cell.mix.burn(dt) / dt;
            if cell.burn_rate > MIN_BURN {
                cell.burning_for += dt;
                cell.idle_for = 0.0;
            } else {
                cell.burning_for = 0.0;
                cell.idle_for += dt;
            }
        }

        // Trade with neighbours: each shared side once.
        for &i in &indices {
            let pos = self.pos(i);
            for d in [Dir::North, Dir::East, Dir::South, Dir::West] {
                let Some(j) = self.index(pos.step(d)) else {
                    continue;
                };
                if self.edge(i, j).is_some_and(|open| !open) || self.cells[j] != Cell::Open {
                    continue;
                }
                if self.cells_on_fire.contains_key(&j) {
                    if j > i {
                        let mut b = self.cells_on_fire.remove(&j).expect("checked");
                        let a = self.cells_on_fire.get_mut(&i).expect("a cell");
                        mix_parcels(&mut a.mix, &mut b.mix, mixing);
                        self.cells_on_fire.insert(j, b);
                    }
                    continue;
                }
                let Some(z) = self.zone_of[j] else { continue };
                let zone = &mut self.zones[z];
                if zone.volume <= 0.0 {
                    continue;
                }
                // A tile's worth of the zone beside the cell.
                let mut parcel = zone.mix.remove_fraction((TILE_VOLUME / zone.volume).min(1.0));
                let cell = self.cells_on_fire.get_mut(&i).expect("a cell");
                mix_parcels(&mut cell.mix, &mut parcel, mixing);
                zone.mix.merge(&parcel);
            }
        }

        // Spread from cells that have burnt long enough.
        let mut lit: Vec<(usize, usize)> = Vec::new();
        for &i in &indices {
            let cell = &self.cells_on_fire[&i];
            if cell.burning_for < SPREAD_AFTER || cell.mix.temperature <= PLASMA_BURN_TEMPERATURE {
                continue;
            }
            for d in [Dir::North, Dir::East, Dir::South, Dir::West] {
                let Some(j) = self.index(cell.pos.step(d)) else {
                    continue;
                };
                if self.edge(i, j).is_some_and(|open| !open)
                    || self.cells[j] != Cell::Open
                    || self.cells_on_fire.contains_key(&j)
                    || lit.iter().any(|(k, _)| *k == j)
                {
                    continue;
                }
                let Some(z) = self.zone_of[j] else { continue };
                let zone = &self.zones[z];
                if zone.volume > TILE_VOLUME && Self::can_catch(&zone.mix.scaled(TILE_VOLUME / zone.volume)) {
                    lit.push((j, z));
                }
            }
        }
        for (j, z) in lit {
            if let Some(cell) = self.open_cell(j, z) {
                cell.mix.temperature += SPREAD_HEAT;
            }
        }

        // Settle the ones that are done.
        let settled: Vec<usize> = self
            .cells_on_fire
            .iter()
            .filter(|(_, c)| {
                let zone_t = self
                    .zones
                    .get(c.zone)
                    .map_or(c.mix.temperature, |z| z.mix.temperature);
                c.idle_for >= SETTLE_MAX
                    || (c.idle_for >= SETTLE_AFTER && (c.mix.temperature - zone_t).abs() < SETTLE_DELTA)
                    || self.zone_leaks_to_space(c.zone)
            })
            .map(|(i, _)| *i)
            .collect();
        for i in settled {
            let cell = self.cells_on_fire.remove(&i).expect("listed");
            if let Some(zone) = self.zones.get_mut(cell.zone) {
                zone.mix.merge(&cell.mix);
                zone.volume += TILE_VOLUME;
            }
        }
    }

    /// Every burning tile and how fast it burns, mol/s: fire cells, and every other tile of
    /// a zone burning as a whole (its rate shared out).
    pub fn burning_tiles(&self) -> Vec<(TilePos, f64)> {
        let mut out: BTreeMap<TilePos, f64> = BTreeMap::new();
        for zone in &self.zones {
            if zone.burn_rate <= 0.0 {
                continue;
            }
            let per_tile = zone.burn_rate * TILE_VOLUME / zone.volume.max(TILE_VOLUME);
            for pos in &zone.tiles {
                out.insert(*pos, per_tile);
            }
        }
        for cell in self.cells_on_fire.values() {
            if cell.burn_rate > MIN_BURN {
                out.insert(cell.pos, cell.burn_rate);
            } else {
                out.remove(&cell.pos);
            }
        }
        out.into_iter().collect()
    }
}
