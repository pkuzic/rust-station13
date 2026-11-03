//! What the air does to a body: breathing, pressure, temperature.
//! Spec and tuning: `docs/reference/environment.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_simulation::atmos::gas::{ONE_ATMOSPHERE, T_SPACE, TILE_VOLUME};
use rs13_simulation::atmos::{Gas, GasMix};
use rs13_world::{GridPos, MobStat, Slot};

use crate::actions::Outbox;
use crate::atmos::StationAtmos;
use crate::equipment::{Equipment, TankGas, Wearable, WornBy, sealed};
use crate::hands::HeldBy;
use crate::health::{Body, Vitals, Zone};
use crate::time::WorldTime;

/// A body's life cycle runs every this many ticks (2 s).
pub const LIFE_TICKS: u64 = 40;
/// Breathe every this many life cycles (4 s).
pub const LIFE_CYCLES_PER_BREATH: u64 = 2;
/// Litres in one breath.
pub const BREATH_VOLUME: f64 = 0.5;
/// Below this oxygen partial pressure (kPa) a breath does not satisfy.
pub const SAFE_OXYGEN: f64 = 16.0;
/// Above this CO2 partial pressure (kPa) a breath starts to poison.
pub const SAFE_CO2: f64 = 10.0;
/// Above this plasma partial pressure (kPa) a breath is toxic.
pub const SAFE_PLASMA: f64 = 0.5;

/// Pressure thresholds and damage per life cycle (/tg/station values).
pub const HAZARD_LOW_PRESSURE: f64 = 20.0;
pub const WARNING_LOW_PRESSURE: f64 = 50.0;
pub const WARNING_HIGH_PRESSURE: f64 = 325.0;
pub const HAZARD_HIGH_PRESSURE: f64 = 550.0;
pub const LOW_PRESSURE_DAMAGE: f32 = 4.0;
pub const MAX_HIGH_PRESSURE_DAMAGE: f32 = 4.0;
pub const PRESSURE_DAMAGE_COEFFICIENT: f32 = 4.0;

/// Normal body temperature, K.
pub const BODY_TEMPERATURE: f32 = 310.15;
/// Below this the body takes cold damage, K (/tg/).
pub const COLD_DAMAGE_LIMIT: f32 = 260.15;
/// Above this the body takes heat damage, K (/tg/).
pub const HEAT_DAMAGE_LIMIT: f32 = 360.15;
/// How far thermoregulation corrects body temperature per life cycle, K.
pub const METABOLIC_WARMING: f32 = 1.5;
/// How far a body cools per life cycle in vacuum (radiation and evaporation, exaggerated
/// for gameplay), K.
pub const VACUUM_COOLING: f32 = 5.0;
/// Below this pressure, kPa, the air is too thin to matter for body heat: count as vacuum.
pub const THIN_AIR: f64 = 1.0;
/// Fraction of the body/air temperature difference closed per life cycle at 1 atm.
pub const AIR_HEAT_EXCHANGE: f32 = 0.05;
/// How much heat a sealed space suit and helmet still let through, as a fraction.
pub const SUIT_INSULATION: f32 = 0.05;

/// Breathing state: the bits of a body that only matter to lungs.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Lungs {
    /// Breaths in a row that will be missed (Luna `losebreath`).
    pub losebreath: u32,
    /// Seconds spent breathing too much CO2.
    pub co2_exposure: f32,
    pub body_temperature: f32,
    /// The tank feeding the mask, if internals are on.
    pub internal: Option<Entity>,
}

impl Default for Lungs {
    fn default() -> Self {
        Self {
            losebreath: 0,
            co2_exposure: 0.0,
            body_temperature: BODY_TEMPERATURE,
            internal: None,
        }
    }
}

/// What a mob's surroundings are like, refreshed every life cycle. Used for the HUD.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct Surroundings {
    pub pressure: f32,
    pub temperature: f32,
    pub oxygen: f32,
}

/// Measure each mob's surroundings, every tick (cheap; keeps the HUD current).
pub fn sense_surroundings(atmos: Option<Res<StationAtmos>>, mut mobs: Query<(&GridPos, &mut Surroundings)>) {
    let Some(atmos) = atmos else { return };
    for (pos, mut around) in &mut mobs {
        let air = atmos.0.air_at(pos.0);
        around.set_if_neq(Surroundings {
            pressure: air.pressure(TILE_VOLUME) as f32,
            temperature: if air.is_vacuum() {
                0.0
            } else {
                air.temperature as f32
            },
            oxygen: air.partial_pressure(Gas::Oxygen, TILE_VOLUME) as f32,
        });
    }
}

/// Luna's `BURN_DMG`, and a fire's share of a tile (its hotspot's starting volume over a tile's).
const BURN_DAMAGE: f32 = 6.0;
const FIRE_SHARE: f32 = 0.05;

/// How well what someone has on stands up to a fire this hot (Luna `add_fire_protection`). Anything
/// made for hotter than this helps, the suit most.
fn fire_protection(equipment: &Equipment, wearables: &Query<&Wearable>, temperature: f32) -> f32 {
    [
        (Slot::Head, 30.0),
        (Slot::Mask, 40.0),
        (Slot::Glasses, 60.0),
        (Slot::Suit, 10.0),
        (Slot::Uniform, 20.0),
        (Slot::Gloves, 40.0),
        (Slot::Shoes, 40.0),
    ]
    .into_iter()
    .filter_map(|(slot, share)| {
        let worn = wearables.get(equipment.get(slot)?).ok()?;
        (worn.protect_k > temperature).then_some(worn.protect_k / share)
    })
    .sum()
}

/// Standing in a fire burns, each part according to what's over it (Luna `FireBurn`).
pub fn burn_in_fires(
    time: Res<WorldTime>,
    atmos: Option<Res<StationAtmos>>,
    mut mobs: Query<(&GridPos, &mut Body, &Vitals, Option<&Equipment>)>,
    wearables: Query<&Wearable>,
) {
    let Some(atmos) = atmos else { return };
    if !time.tick.0.is_multiple_of(LIFE_TICKS) {
        return;
    }
    for (pos, mut body, vitals, equipment) in &mut mobs {
        if vitals.stat == MobStat::Dead || !atmos.0.burning_at(pos.0) {
            continue;
        }
        let temperature = atmos.0.air_at(pos.0).temperature as f32;
        let protection = equipment.map_or(0.0, |e| fire_protection(e, &wearables, temperature));
        let intensity = FIRE_SHARE / protection.max(0.5) * BURN_DAMAGE * 10.0;
        for zone in Zone::ALL {
            let share = match zone {
                Zone::Head | Zone::Chest => 2.5,
                Zone::Groin => 2.0,
                Zone::LLeg | Zone::RLeg => 0.6,
                Zone::LArm | Zone::RArm => 0.4,
                _ => 0.25,
            };
            let exposure: f32 = equipment.map_or(1.0, |e| {
                e.0.values()
                    .filter_map(|item| wearables.get(*item).ok())
                    .filter(|worn| worn.covers & zone.cover_bit() != 0)
                    .map(|worn| worn.heat)
                    .product()
            });
            body.take_damage(zone, 0.0, share * intensity * exposure);
        }
    }
}

/// Every life cycle: breathe (every other cycle), suffer pressure, gain or lose heat.
pub fn life(
    time: Res<WorldTime>,
    atmos: Option<ResMut<StationAtmos>>,
    mut mobs: Query<(
        Entity,
        &GridPos,
        &mut Body,
        &mut Lungs,
        &Vitals,
        Option<&mut Outbox>,
        Option<&Equipment>,
    )>,
    mut tanks: Query<(&mut TankGas, Option<&HeldBy>, Option<&WornBy>)>,
    wearables: Query<&Wearable>,
) {
    let Some(mut atmos) = atmos else { return };
    let tick = time.tick.0;
    if !tick.is_multiple_of(LIFE_TICKS) {
        return;
    }
    let breathe_now = (tick / LIFE_TICKS).is_multiple_of(LIFE_CYCLES_PER_BREATH);

    for (mob, pos, mut body, mut lungs, vitals, outbox, equipment) in &mut mobs {
        if vitals.stat == MobStat::Dead {
            continue;
        }
        let air = atmos.0.air_at(pos.0);
        let pressure = air.pressure(TILE_VOLUME);
        let sealed = equipment.is_some_and(|e| sealed(e, &wearables));

        if breathe_now {
            // Internals: the tank must still be on this person, behind an internals mask.
            let mask_ok = equipment
                .and_then(|e| e.get(Slot::Mask))
                .and_then(|m| wearables.get(m).ok())
                .is_some_and(|w| w.internals);
            let tank = lungs
                .internal
                .filter(|_| mask_ok)
                .and_then(|t| tanks.get_mut(t).ok())
                .filter(|(_, held, worn)| {
                    held.is_some_and(|h| h.0 == mob) || worn.is_some_and(|w| w.0 == mob)
                });
            match tank {
                Some((mut tank, ..)) => {
                    let mut breath = tank.take_breath();
                    breathe_from(&mut body, &mut lungs, vitals, &mut breath);
                    // Exhaled into the room, as in Luna.
                    atmos.0.add_at(pos.0, &breath);
                }
                None => {
                    lungs.internal = None;
                    breathe(&mut atmos, pos, &mut body, &mut lungs, vitals, &air);
                }
            }
        }
        // A sealed space suit and helmet hold pressure and heat in (/tg/ style).
        if !sealed {
            pressure_damage(&mut body, pressure);
        }
        let insulation = if sealed { SUIT_INSULATION } else { 1.0 };
        temperature(&mut body, &mut lungs, &air, pressure, insulation);

        if let Some(mut outbox) = outbox
            && vitals.stat == MobStat::Conscious
            && pressure < HAZARD_LOW_PRESSURE
            && breathe_now
        {
            outbox.0.push(LocMsg::new("gasp"));
        }
    }
}

fn breathe(
    atmos: &mut StationAtmos,
    pos: &GridPos,
    body: &mut Body,
    lungs: &mut Lungs,
    vitals: &Vitals,
    air: &GasMix,
) {
    let in_crit = vitals.health < 0.0;
    if in_crit {
        lungs.losebreath += 1;
    }
    if in_crit && lungs.losebreath > 10 {
        // Not breathing at all.
        lungs.losebreath -= 1;
        body.oxy += 4.0;
        return;
    }
    let moles = air.total_moles() * BREATH_VOLUME / TILE_VOLUME;
    let mut breath = atmos.0.remove_at(pos.0, moles);
    handle_breath(body, lungs, &mut breath);
    atmos.0.add_at(pos.0, &breath);
}

/// A breath from a tank: the same crit rules, then `handle_breath`.
fn breathe_from(body: &mut Body, lungs: &mut Lungs, vitals: &Vitals, breath: &mut GasMix) {
    let in_crit = vitals.health < 0.0;
    if in_crit {
        lungs.losebreath += 1;
    }
    if in_crit && lungs.losebreath > 10 {
        lungs.losebreath -= 1;
        body.oxy += 4.0;
        return;
    }
    handle_breath(body, lungs, breath);
}

/// Luna's `handle_breath`, on a breath of `BREATH_VOLUME` litres. Turns some oxygen into CO2.
pub fn handle_breath(body: &mut Body, lungs: &mut Lungs, breath: &mut GasMix) {
    let o2 = breath.partial_pressure(Gas::Oxygen, BREATH_VOLUME);
    let co2 = breath.partial_pressure(Gas::CarbonDioxide, BREATH_VOLUME);
    let plasma = breath.partial_pressure(Gas::Plasma, BREATH_VOLUME);
    let oxygen = breath.get(Gas::Oxygen);

    let used = if o2 < SAFE_OXYGEN {
        if o2 > 0.0 {
            body.oxy += (5.0 * SAFE_OXYGEN / o2).min(7.0) as f32;
            oxygen * (o2 / SAFE_OXYGEN) / 6.0
        } else {
            body.oxy += 14.0;
            0.0
        }
    } else {
        body.oxy = (body.oxy - 5.0).max(0.0);
        // Good air also lets the body catch its breath again.
        lungs.losebreath = lungs.losebreath.saturating_sub(1);
        oxygen / 6.0
    };
    breath.set(Gas::Oxygen, oxygen - used);
    breath.set(Gas::CarbonDioxide, breath.get(Gas::CarbonDioxide) + used);

    let breath_seconds = (LIFE_TICKS * LIFE_CYCLES_PER_BREATH) as f32 / rs13_core::TICK_RATE_HZ as f32;
    if co2 > SAFE_CO2 {
        lungs.co2_exposure += breath_seconds;
        if lungs.co2_exposure > 12.0 {
            body.oxy += 6.0;
        }
        if lungs.co2_exposure > 30.0 {
            body.oxy += 16.0;
        }
    } else {
        lungs.co2_exposure = 0.0;
    }
    if plasma > SAFE_PLASMA {
        body.tox += ((breath.get(Gas::Plasma) / 0.5) * 3.0).min(30.0) as f32;
    }
    if body.oxy > 10.0 {
        lungs.losebreath += 1;
    }
}

/// /tg/station-style pressure damage, once per life cycle.
pub fn pressure_damage(body: &mut Body, pressure: f64) {
    if pressure < HAZARD_LOW_PRESSURE {
        body.take_damage_spread(LOW_PRESSURE_DAMAGE, 0.0);
    } else if pressure > HAZARD_HIGH_PRESSURE {
        let damage = ((pressure / HAZARD_HIGH_PRESSURE - 1.0) as f32 * PRESSURE_DAMAGE_COEFFICIENT)
            .min(MAX_HIGH_PRESSURE_DAMAGE);
        body.take_damage_spread(damage, 0.0);
    }
}

/// Body heat: air pulls it toward its own temperature (more strongly the denser it is);
/// vacuum drains it; thermoregulation pulls it back. Then cold or heat damage.
pub fn temperature(body: &mut Body, lungs: &mut Lungs, air: &GasMix, pressure: f64, insulation: f32) {
    let t = &mut lungs.body_temperature;
    if pressure < THIN_AIR {
        *t -= VACUUM_COOLING * insulation;
    } else {
        let density = (pressure / ONE_ATMOSPHERE).min(2.0) as f32;
        *t += (air.temperature as f32 - *t) * AIR_HEAT_EXCHANGE * density * insulation;
    }
    // Thermoregulation (shivering, sweating) pulls back toward normal, up to a limit.
    let correction = (BODY_TEMPERATURE - *t).clamp(-METABOLIC_WARMING, METABOLIC_WARMING);
    *t = (*t + correction).max(T_SPACE as f32);

    let burn = if *t < COLD_DAMAGE_LIMIT {
        if *t > 200.0 {
            0.5
        } else if *t > 120.0 {
            1.5
        } else {
            3.0
        }
    } else if *t > HEAT_DAMAGE_LIMIT {
        if *t < 400.0 {
            2.0
        } else if *t < 460.0 {
            3.0
        } else {
            8.0
        }
    } else {
        0.0
    };
    if burn > 0.0 {
        body.take_damage_spread(0.0, burn);
    }
}

/// The HUD warnings for these surroundings and this body.
pub fn alerts(around: &Surroundings, lungs: &Lungs) -> Vec<rs13_world::EnvAlert> {
    use rs13_world::EnvAlert;
    let mut out = Vec::new();
    let p = f64::from(around.pressure);
    if p < HAZARD_LOW_PRESSURE {
        out.push(EnvAlert::Vacuum);
    } else if p < WARNING_LOW_PRESSURE {
        out.push(EnvAlert::LowPressure);
    } else if p > WARNING_HIGH_PRESSURE {
        out.push(EnvAlert::HighPressure);
    }
    if f64::from(around.oxygen) < SAFE_OXYGEN {
        out.push(EnvAlert::LowOxygen);
    }
    if lungs.body_temperature < COLD_DAMAGE_LIMIT {
        out.push(EnvAlert::Cold);
    } else if lungs.body_temperature > HEAT_DAMAGE_LIMIT {
        out.push(EnvAlert::Hot);
    }
    out
}
