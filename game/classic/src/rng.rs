//! Game randomness. One seeded generator per world, so a test or a replay with the same
//! seed and inputs gives the same results.

use bevy_ecs::prelude::*;

/// xorshift64*: small, fast and good enough for game rolls. Not for security.
#[derive(Resource, Debug, Clone)]
pub struct GameRng(u64);

impl Default for GameRng {
    fn default() -> Self {
        Self::seeded(0x5253_3133)
    }
}

impl GameRng {
    pub fn seeded(seed: u64) -> Self {
        // Zero would stay zero forever.
        Self(seed | 1)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// BYOND `prob(percent)`.
    pub fn prob(&mut self, percent: f32) -> bool {
        let roll = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64 * 100.0;
        roll < f64::from(percent)
    }

    /// BYOND `rand(low, high)`: an integer in `low..=high`.
    pub fn range(&mut self, low: i32, high: i32) -> i32 {
        let span = (high - low + 1).max(1) as u64;
        low + (self.next_u64() % span) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_rolls() {
        let mut a = GameRng::seeded(42);
        let mut b = GameRng::seeded(42);
        for _ in 0..100 {
            assert_eq!(a.range(1, 9), b.range(1, 9));
        }
    }

    #[test]
    fn range_is_inclusive_and_covers_everything() {
        let mut rng = GameRng::seeded(1);
        let mut seen = [false; 9];
        for _ in 0..1000 {
            let r = rng.range(1, 9);
            assert!((1..=9).contains(&r));
            seen[(r - 1) as usize] = true;
        }
        assert!(seen.iter().all(|s| *s));
    }
}
