//! Shared identifiers, time and tick constants used by every engine crate.

/// Authoritative server simulation rate. Benchmark before changing; see `docs/TESTING.md`.
pub const TICK_RATE_HZ: u32 = 20;

/// A count of fixed simulation ticks since the round started.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Tick(pub u64);

impl Tick {
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

/// Stable entity identifier that survives saves, reloads and network replication,
/// unlike the ECS-internal entity id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityUid(pub u64);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_advances() {
        assert_eq!(Tick::default().next(), Tick(1));
    }
}
