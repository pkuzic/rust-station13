//! Gas mixtures and the ideal gas law.
//!
//! Units: moles, kelvin, litres, kilopascals, joules. With volume in litres the ideal
//! gas law gives pressure directly in kPa: `P = n·R·T / V`.

/// Molar gas constant, J/(mol·K). The real value, not Luna's doubled one.
pub const R: f64 = 8.314;
/// Temperature of space (cosmic microwave background), K.
pub const T_SPACE: f64 = 2.7;
/// 20 °C, K.
pub const T20C: f64 = 293.15;
/// 0 °C, K.
pub const T0C: f64 = 273.15;
/// One standard atmosphere, kPa.
pub const ONE_ATMOSPHERE: f64 = 101.325;
/// Volume of one tile of air: 1 m × 1 m × 2.5 m, in litres.
pub const TILE_VOLUME: f64 = 2500.0;
/// Below this many moles a mixture counts as vacuum.
pub const VACUUM_MOLES: f64 = 1e-6;

/// The gases tracked by the simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Gas {
    Oxygen,
    Nitrogen,
    CarbonDioxide,
    Plasma,
    NitrousOxide,
}

impl Gas {
    pub const ALL: [Gas; 5] = [
        Gas::Oxygen,
        Gas::Nitrogen,
        Gas::CarbonDioxide,
        Gas::Plasma,
        Gas::NitrousOxide,
    ];

    /// Molar heat capacity at constant volume, J/(mol·K). SS13's classic values, which are
    /// close to real for O2/N2 (20.8) and CO2 (28.5); plasma is fictional.
    pub fn heat_capacity(self) -> f64 {
        match self {
            Gas::Oxygen | Gas::Nitrogen => 20.0,
            Gas::CarbonDioxide => 30.0,
            Gas::Plasma => 200.0,
            Gas::NitrousOxide => 40.0,
        }
    }

    /// Molar mass, kg/mol. Plasma is fictional; heavy, like a big organic molecule.
    pub fn molar_mass(self) -> f64 {
        match self {
            Gas::Oxygen => 0.032,
            Gas::Nitrogen => 0.028,
            Gas::CarbonDioxide => 0.044,
            Gas::Plasma => 0.150,
            Gas::NitrousOxide => 0.044,
        }
    }

    fn index(self) -> usize {
        self as usize
    }
}

/// An amount of gas at one temperature.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GasMix {
    pub(crate) moles: [f64; 5],
    pub temperature: f64,
}

impl Default for GasMix {
    /// Vacuum.
    fn default() -> Self {
        Self {
            moles: [0.0; 5],
            temperature: T_SPACE,
        }
    }
}

impl GasMix {
    /// Standard breathable air (21% O2, 79% N2) at 101.325 kPa and 20 °C in `volume` litres.
    pub fn standard_air(volume: f64) -> Self {
        let total = ONE_ATMOSPHERE * volume / (R * T20C);
        let mut mix = GasMix {
            moles: [0.0; 5],
            temperature: T20C,
        };
        mix.set(Gas::Oxygen, total * 0.21);
        mix.set(Gas::Nitrogen, total * 0.79);
        mix
    }

    pub fn get(&self, gas: Gas) -> f64 {
        self.moles[gas.index()]
    }

    pub fn set(&mut self, gas: Gas, moles: f64) {
        self.moles[gas.index()] = moles.max(0.0);
    }

    pub fn total_moles(&self) -> f64 {
        self.moles.iter().sum()
    }

    pub fn is_vacuum(&self) -> bool {
        self.total_moles() < VACUUM_MOLES
    }

    /// J/K.
    pub fn heat_capacity(&self) -> f64 {
        Gas::ALL.iter().map(|g| self.get(*g) * g.heat_capacity()).sum()
    }

    /// Internal energy, J.
    pub fn thermal_energy(&self) -> f64 {
        self.heat_capacity() * self.temperature
    }

    /// kPa, for a mixture filling `volume` litres.
    pub fn pressure(&self, volume: f64) -> f64 {
        if volume <= 0.0 {
            return 0.0;
        }
        self.total_moles() * R * self.temperature / volume
    }

    /// Partial pressure of one gas, kPa.
    pub fn partial_pressure(&self, gas: Gas, volume: f64) -> f64 {
        if volume <= 0.0 {
            return 0.0;
        }
        self.get(gas) * R * self.temperature / volume
    }

    /// Ratio of heat capacities γ = Cp/Cv of the mixture (1.4 for air).
    pub fn gamma(&self) -> f64 {
        let n = self.total_moles();
        if n < VACUUM_MOLES {
            return 1.4;
        }
        let cv = self.heat_capacity() / n;
        (cv + R) / cv
    }

    /// Mean molar mass, kg/mol.
    pub fn molar_mass(&self) -> f64 {
        let n = self.total_moles();
        if n < VACUUM_MOLES {
            return 0.029;
        }
        Gas::ALL
            .iter()
            .map(|g| self.get(*g) * g.molar_mass())
            .sum::<f64>()
            / n
    }

    /// Take `fraction` (0..=1) of every gas out of this mixture, at this temperature.
    pub fn remove_fraction(&mut self, fraction: f64) -> GasMix {
        let fraction = fraction.clamp(0.0, 1.0);
        let mut out = GasMix {
            moles: [0.0; 5],
            temperature: self.temperature,
        };
        for i in 0..self.moles.len() {
            let taken = self.moles[i] * fraction;
            out.moles[i] = taken;
            self.moles[i] -= taken;
        }
        out
    }

    /// Take `moles` out, in proportion to the composition.
    pub fn remove_moles(&mut self, moles: f64) -> GasMix {
        let total = self.total_moles();
        if total <= 0.0 {
            return GasMix::default();
        }
        self.remove_fraction(moles / total)
    }

    /// Add `other`, keeping total thermal energy (heat-capacity-weighted temperature).
    pub fn merge(&mut self, other: &GasMix) {
        let energy = self.thermal_energy() + other.thermal_energy();
        for i in 0..self.moles.len() {
            self.moles[i] += other.moles[i];
        }
        let capacity = self.heat_capacity();
        if capacity > 0.0 {
            self.temperature = energy / capacity;
        }
    }

    /// Add `energy` joules of heat (negative to remove), never below absolute zero.
    pub fn add_energy(&mut self, energy: f64) {
        let capacity = self.heat_capacity();
        if capacity > 0.0 {
            self.temperature = ((self.thermal_energy() + energy) / capacity).max(0.0);
        }
    }

    /// Multiply every amount by `factor` (e.g. to scale a per-tile share).
    pub fn scaled(&self, factor: f64) -> GasMix {
        let mut out = *self;
        for m in &mut out.moles {
            *m *= factor;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_air_is_one_atmosphere() {
        let air = GasMix::standard_air(TILE_VOLUME);
        assert!((air.pressure(TILE_VOLUME) - ONE_ATMOSPHERE).abs() < 1e-9);
        assert!((air.partial_pressure(Gas::Oxygen, TILE_VOLUME) - 21.278).abs() < 0.01);
        // About 104 mol in a 2.5 m³ tile.
        assert!((air.total_moles() - 103.94).abs() < 0.01);
        assert!((air.gamma() - 1.4157).abs() < 1e-3);
    }

    #[test]
    fn heating_a_sealed_volume_raises_pressure() {
        let mut air = GasMix::standard_air(TILE_VOLUME);
        air.add_energy(air.heat_capacity() * T20C);
        assert!((air.pressure(TILE_VOLUME) - 2.0 * ONE_ATMOSPHERE).abs() < 1e-9);
    }

    #[test]
    fn merge_conserves_moles_and_energy() {
        let mut cold = GasMix::standard_air(TILE_VOLUME);
        cold.temperature = 200.0;
        let mut hot = GasMix::standard_air(TILE_VOLUME);
        hot.set(Gas::Plasma, 5.0);
        hot.temperature = 600.0;
        let (n, e) = (
            cold.total_moles() + hot.total_moles(),
            cold.thermal_energy() + hot.thermal_energy(),
        );
        cold.merge(&hot);
        assert!((cold.total_moles() - n).abs() < 1e-9);
        assert!((cold.thermal_energy() - e).abs() < 1e-6);
    }

    #[test]
    fn removing_keeps_composition() {
        let mut air = GasMix::standard_air(TILE_VOLUME);
        let taken = air.remove_moles(10.0);
        assert!((taken.total_moles() - 10.0).abs() < 1e-9);
        assert!((taken.get(Gas::Oxygen) / taken.total_moles() - 0.21).abs() < 1e-9);
        assert_eq!(taken.temperature, air.temperature);
    }
}
