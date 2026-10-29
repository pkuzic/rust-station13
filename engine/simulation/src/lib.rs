//! Fixed-tick schedule ordering and simulation solver contracts.
//!
//! Solvers (atmos, powernet, ...) live in their own modules and talk to each other only
//! through the contracts defined here, never through each other's internal storage.
//! See `docs/ARCHITECTURE.md`.

use bevy_ecs::schedule::SystemSet;

pub mod atmos;

/// Ordering of work inside one server tick.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TickSet {
    /// Apply validated player and NPC commands.
    Commands,
    /// Run interactions and machinery.
    Gameplay,
    /// Run simulation solvers (atmos, power, heat).
    Simulation,
    /// Collect changes for replication and persistence.
    Publish,
}
