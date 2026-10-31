//! The thermo-electric generator: turns the difference between a hot and a cold pipe loop
//! into power. Spec: `docs/reference/engine.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, Dense, Dir, GridPos, NetId, ObjectSpec, TilePos};

use crate::machines::{DEVICE_VOLUME, GeneratorInput, MACHINE_TICKS, PipeNets};
use crate::power::{POWER_DT, PowerNode, PowerSupply};
use crate::time::WorldTime;

/// The generator's prototype.
pub const TEG: &str = "teg";
/// Share of Carnot efficiency it reaches (Luna: `(1 - Tc/Th) × 0.75`).
pub const CARNOT_SHARE: f64 = 0.75;
/// Output for the brightest of its eleven lights, W (Luna `round(11 × lastgen / 100000)`).
pub const FULL_LIGHTS: f64 = 100_000.0;

/// A thermo-electric generator, between its two generator inputs (east and west).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Teg {
    /// Share of the possible heat flow it lets through (Luna `transferpercent`, 70 on the
    /// map).
    pub transfer: f64,
    /// Multiplier on its output (Luna `outputpercent`, 1 on the map).
    pub output: f64,
    /// Power made last cycle, W.
    pub last: f64,
    /// Hot and cold loop temperature (K) and pressure (kPa) last cycle, for examining.
    pub loops: Option<[(f64, f64); 2]>,
}

pub fn spawn_teg(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    let number = |name: &str, default: f64| {
        spec.vars
            .get(name)
            .and_then(|v| v.trim().parse::<f64>().ok())
            .unwrap_or(default)
    };
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(spec.id.as_str(), ""),
            Teg {
                transfer: number("transferpercent", 100.0) / 100.0,
                output: number("outputpercent", 1.0),
                last: 0.0,
                loops: None,
            },
            PowerSupply::default(),
            PowerNode,
            Dense,
        ))
        .id()
}

/// Every machine cycle, before the power cycle (Luna `generator_type2/process`): heat flows
/// from the hotter loop to the colder through the generator, a share of it becomes power,
/// the rest warms the cold loop. Each loop is the network of the generator input beside it.
pub fn run_tegs(
    time: Res<WorldTime>,
    mut nets: ResMut<PipeNets>,
    inputs: Query<(Entity, &GridPos), With<GeneratorInput>>,
    mut tegs: Query<(&GridPos, &mut Teg, &mut PowerSupply, &mut Appearance)>,
) {
    if !time.tick.0.is_multiple_of(MACHINE_TICKS) {
        return;
    }
    let net_beside = |nets: &PipeNets, at: TilePos| {
        inputs
            .iter()
            .find(|(_, p)| p.0 == at)
            .and_then(|(e, _)| nets.net_of.get(&e).copied())
    };
    for (pos, mut teg, mut supply, mut look) in &mut tegs {
        let east = net_beside(&nets, pos.0.step(Dir::East));
        let west = net_beside(&nets, pos.0.step(Dir::West));
        let mut made = 0.0;
        teg.loops = None;
        if let (Some(a), Some(b)) = (east, west)
            && a != b
        {
            let (hot, cold) = if nets.nets[a].mix.temperature >= nets.nets[b].mix.temperature {
                (a, b)
            } else {
                (b, a)
            };
            // Each loop takes part through the input's own 200 L (Luna `air_contents`).
            let share = |net: usize| {
                let n = &nets.nets[net];
                if n.volume > 0.0 {
                    n.mix.scaled(DEVICE_VOLUME.min(n.volume) / n.volume)
                } else {
                    n.mix
                }
            };
            let (hot_part, cold_part) = (share(hot), share(cold));
            let (hc, cc) = (hot_part.heat_capacity(), cold_part.heat_capacity());
            let delta = hot_part.temperature - cold_part.temperature;
            if delta > 1.0 && hc > 0.01 && cc > 0.01 {
                let efficiency = (1.0 - cold_part.temperature / hot_part.temperature) * CARNOT_SHARE;
                let transferred = delta * hc * cc / (hc + cc) * teg.transfer;
                let electricity = transferred * efficiency * teg.output;
                nets.nets[hot].mix.add_energy(-transferred);
                nets.nets[cold].mix.add_energy(transferred * (1.0 - efficiency));
                made = electricity / POWER_DT;
            }
            let reading = |net: usize| (nets.nets[net].mix.temperature, nets.nets[net].pressure());
            teg.loops = Some([reading(hot), reading(cold)]);
        }
        teg.last = made;
        if supply.watts != made {
            supply.watts = made;
        }
        let lights = (11.0 * made / FULL_LIGHTS).round().clamp(0.0, 11.0) as u8;
        let state = if lights == 0 {
            String::new()
        } else {
            format!("+teg-op{lights}")
        };
        if look.state != state {
            look.state = state;
        }
    }
}

/// What examining a generator shows (Luna's window: output and both loops).
pub fn examine(teg: &Teg) -> LocMsg {
    let round = |v: f64| v.round() as i64;
    let [(hot_t, hot_p), (cold_t, cold_p)] = teg.loops.unwrap_or([(0.0, 0.0); 2]);
    LocMsg::new("examine-teg")
        .num("watts", round(teg.last))
        .num("hot", round(hot_t))
        .num("hotkpa", round(hot_p))
        .num("cold", round(cold_t))
        .num("coldkpa", round(cold_p))
}
