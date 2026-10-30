//! Pipe pressure meters. Spec: `docs/reference/meters.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_simulation::atmos::gas::ONE_ATMOSPHERE;
use rs13_world::{Appearance, GridPos, TilePos};

use crate::machines::{GasStore, PipeMember, PipeNets};
use crate::power::PowerUser;

/// A meter reads from this far away, in tiles (Luna `get_dist(usr, src) <= 3`).
pub const READ_RANGE: i32 = 3;
/// Luna's `use_power(5)` every machine cycle.
pub const METER_POWER: f64 = 5.0;

/// A pressure meter on a pipe: the pipe it reads, once it has found one on its tile, and
/// the pressure it last read there, kPa.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq)]
pub struct Meter {
    pub target: Option<Entity>,
    pub kpa: Option<f64>,
}

/// The pipe pieces a meter can sit on (Luna `/obj/machinery/atmospherics/pipe`).
fn is_pipe(sprite: &str) -> bool {
    matches!(sprite, "pipe" | "manifold") || crate::machines::is_pipe_tank(sprite)
}

/// Luna's dial: one of six steps in each pressure band.
pub fn dial(kpa: f64) -> String {
    let atm = kpa / ONE_ATMOSPHERE;
    // BYOND `round(x)`: to the nearest whole number, halves up.
    let round = |x: f64| (x + 0.5).floor() as i64;
    if atm <= 0.15 {
        "meter0".to_owned()
    } else if atm <= 1.8 {
        format!("meter1_{}", round(atm / 0.3 + 0.5).min(6))
    } else if atm <= 30.0 {
        format!("meter2_{}", (round(atm / 5.0 - 0.35) + 1).min(6))
    } else if atm <= 59.0 {
        format!("meter3_{}", (round(atm / 5.0 - 6.0) + 1).min(6))
    } else {
        "meter4".to_owned()
    }
}

/// Find each meter's pipe, read it and turn the dial (Luna `meter/process`).
pub fn update_meters(
    nets: Res<PipeNets>,
    mut meters: Query<(&GridPos, &mut Meter, &PowerUser, &mut Appearance)>,
    pipes: Query<(Entity, &GridPos, &PipeMember, Option<&GasStore>, &Appearance), Without<Meter>>,
) {
    for (pos, mut meter, power, mut look) in &mut meters {
        if meter.target.is_none_or(|t| !pipes.contains(t)) {
            meter.target = pipes
                .iter()
                .find(|(_, p, _, _, a)| p.0 == pos.0 && is_pipe(&a.sprite))
                .map(|(e, ..)| e);
        }
        let kpa = meter
            .target
            .and_then(|t| pipes.get(t).ok())
            .map(|(entity, _, member, store, _)| nets.gauge(entity, store, member.volume));
        if meter.kpa != kpa {
            meter.kpa = kpa;
        }
        let state = match (kpa, power.powered) {
            (None, _) => "meterX".to_owned(),
            (Some(_), false) => "meter0".to_owned(),
            (Some(kpa), true) => dial(kpa),
        };
        if look.state != state {
            look.state = state;
        }
    }
}

/// Clicking a meter, with anything, from up to three tiles away: the pressure (Luna
/// `meter/Click`). `None` when `target` is not a meter; `Some(None)` when it says nothing
/// (unpowered).
pub fn click(
    mob_pos: TilePos,
    target: Entity,
    meters: &Query<(&GridPos, &Meter, &PowerUser)>,
) -> Option<Option<LocMsg>> {
    let (pos, meter, power) = meters.get(target).ok()?;
    if !power.powered {
        return Some(None);
    }
    let near = pos.0.z == mob_pos.z
        && (pos.0.x - mob_pos.x).abs() <= READ_RANGE
        && (pos.0.y - mob_pos.y).abs() <= READ_RANGE;
    Some(Some(if !near {
        LocMsg::new("meter-too-far")
    } else {
        match meter.kpa {
            Some(kpa) => LocMsg::new("meter-reading").num("value", kpa.round() as i64),
            None => LocMsg::new("meter-no-pipe"),
        }
    }))
}

/// What examining a meter adds (Luna `meter/examine`).
pub fn examine(target: Entity, meters: &Query<(&GridPos, &Meter, &PowerUser)>) -> Option<LocMsg> {
    let (_, meter, _) = meters.get(target).ok()?;
    Some(match meter.kpa {
        Some(kpa) => LocMsg::new("examine-meter").num("value", kpa.round() as i64),
        None => LocMsg::new("examine-meter-no-pipe"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dial_follows_luna() {
        assert_eq!(dial(0.0), "meter0");
        assert_eq!(dial(ONE_ATMOSPHERE), "meter1_4");
        assert_eq!(dial(1.8 * ONE_ATMOSPHERE), "meter1_6");
        assert_eq!(dial(2.0 * ONE_ATMOSPHERE), "meter2_1");
        assert_eq!(dial(4.5 * ONE_ATMOSPHERE), "meter2_2");
        assert_eq!(dial(30.0 * ONE_ATMOSPHERE), "meter2_6");
        assert_eq!(dial(40.0 * ONE_ATMOSPHERE), "meter3_3");
        assert_eq!(dial(60.0 * ONE_ATMOSPHERE), "meter4");
    }
}
