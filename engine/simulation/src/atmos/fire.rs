//! Combustion: plasma burning in oxygen. Design: `docs/ATMOSPHERICS.md` (thermal and fire),
//! reference: `docs/reference/fire.md`.
//!
//! The reaction is Luna's (`gas_mixture/react`, FEA): above 250 °C plasma burns at a rate
//! that grows with temperature up to 1370 °C, using oxygen and making carbon dioxide and
//! 3 MJ of heat per mole. Luna applies it once per air cycle; here it is a rate per second.

use super::gas::{Gas, GasMix, T0C};

/// Plasma burns above this, K (Luna `PLASMA_MINIMUM_BURN_TEMPERATURE`, 250 °C).
pub const PLASMA_BURN_TEMPERATURE: f64 = 250.0 + T0C;
/// At and above this plasma burns as fast as it can, K (Luna `PLASMA_UPPER_TEMPERATURE`).
pub const PLASMA_UPPER_TEMPERATURE: f64 = 1370.0 + T0C;
/// Heat released per mole of plasma burnt, J (Luna `FIRE_PLASMA_ENERGY_RELEASED`).
pub const PLASMA_ENERGY: f64 = 3_000_000.0;
/// Oxygen per mole of plasma above which plasma, not oxygen, limits the burn (Luna
/// `PLASMA_OXYGEN_FULLBURN`).
pub const OXYGEN_FULLBURN: f64 = 10.0;
/// Below this many moles burnt per second nothing counts as burning.
pub const MIN_BURN: f64 = 1e-4;
/// Litres of gas a spark or flame heats when it touches the air (Luna's hotspot volume).
pub const HOTSPOT_VOLUME: f64 = 125.0;

impl GasMix {
    /// Burn plasma for `dt` seconds (Luna `react`). Returns the moles of plasma burnt.
    pub fn burn(&mut self, dt: f64) -> f64 {
        let plasma = self.get(Gas::Plasma);
        let oxygen = self.get(Gas::Oxygen);
        if plasma <= 0.0 || oxygen <= 0.0 || self.temperature <= PLASMA_BURN_TEMPERATURE {
            return 0.0;
        }
        let scale = ((self.temperature - PLASMA_BURN_TEMPERATURE)
            / (PLASMA_UPPER_TEMPERATURE - PLASMA_BURN_TEMPERATURE))
            .clamp(0.0, 1.0);
        // Oxygen used per mole of plasma: more at lower temperatures.
        let oxygen_per_plasma = 1.4 - scale;
        let rate = if oxygen > plasma * OXYGEN_FULLBURN {
            plasma * scale / 4.0
        } else {
            scale * (oxygen / OXYGEN_FULLBURN) / 4.0
        };
        let burnt = (rate * dt).min(plasma).min(oxygen / oxygen_per_plasma);
        if burnt <= MIN_BURN * dt {
            return 0.0;
        }
        let energy = self.thermal_energy() + PLASMA_ENERGY * burnt;
        self.set(Gas::Plasma, plasma - burnt);
        self.set(Gas::Oxygen, oxygen - burnt * oxygen_per_plasma);
        self.set(Gas::CarbonDioxide, self.get(Gas::CarbonDioxide) + burnt);
        let capacity = self.heat_capacity();
        if capacity > 0.0 {
            self.temperature = energy / capacity;
        }
        burnt
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atmos::gas::T20C;

    fn fuel_air(temperature: f64) -> GasMix {
        let mut mix = GasMix::standard_air(2500.0);
        mix.set(Gas::Plasma, 5.0);
        mix.temperature = temperature;
        mix
    }

    #[test]
    fn nothing_burns_below_250_c() {
        let mut mix = fuel_air(T20C);
        assert_eq!(mix.burn(1.0), 0.0);
        let mut mix = fuel_air(PLASMA_BURN_TEMPERATURE);
        assert_eq!(mix.burn(1.0), 0.0);
    }

    #[test]
    fn hot_plasma_burns_into_carbon_dioxide_and_heat() {
        let mut mix = fuel_air(800.0);
        let (oxygen, temperature) = (mix.get(Gas::Oxygen), mix.temperature);
        let burnt = mix.burn(1.0);
        assert!(burnt > 0.0);
        assert!((mix.get(Gas::Plasma) - (5.0 - burnt)).abs() < 1e-9);
        assert!(mix.get(Gas::Oxygen) < oxygen);
        assert!((mix.get(Gas::CarbonDioxide) - burnt).abs() < 1e-9);
        assert!(mix.temperature > temperature, "it heats up");
    }

    #[test]
    fn energy_released_is_three_megajoules_a_mole() {
        let mut mix = fuel_air(900.0);
        let before = mix.thermal_energy();
        let burnt = mix.burn(0.5);
        assert!((mix.thermal_energy() - before - PLASMA_ENERGY * burnt).abs() < 1e-3);
    }

    #[test]
    fn no_oxygen_no_fire() {
        let mut mix = fuel_air(1000.0);
        mix.set(Gas::Oxygen, 0.0);
        assert_eq!(mix.burn(1.0), 0.0);
    }
}
