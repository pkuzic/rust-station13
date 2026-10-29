//! Atmospherics: gas mixtures, zones and flow. Design: `docs/ATMOSPHERICS.md`.

pub mod gas;
pub mod grid;

pub use gas::{Gas, GasMix};
pub use grid::{AtmosGrid, Cell, Opening, Side, Wind, Zone};

#[cfg(test)]
mod tests;
