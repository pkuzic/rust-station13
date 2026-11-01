//! Air alarms and area emergency seals. Spec: `docs/reference/air-alarm.md`.

use std::collections::HashSet;

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_simulation::atmos::gas::{ONE_ATMOSPHERE, T0C, T20C, TILE_VOLUME};
use rs13_simulation::atmos::{Gas, GasMix};
use rs13_world::{AlarmPanelView, AlarmReading, Appearance, Dir, GridPos, MobStat, NetId, TileMap, TilePos};

use crate::actions::in_reach;
use crate::atmos::StationAtmos;
use crate::doors::{Airlock, DoorState};
use crate::health::Vitals;
use crate::machines::{MACHINE_TICKS, Vent};
use crate::power::PowerUser;
use crate::time::WorldTime;
use crate::wires::PanelViewer;

/// Pressure outside this range (kPa) raises an alert (Luna `ALERT_ATMOSPHERE_L/U`).
pub const ALERT_PRESSURE: (f64, f64) = (0.9 * ONE_ATMOSPHERE, 1.1 * ONE_ATMOSPHERE);
/// Pressure outside this range is dangerous and seals the area (`UNSAFE_ATMOSPHERE_L/U`).
pub const UNSAFE_PRESSURE: (f64, f64) = (0.7 * ONE_ATMOSPHERE, 1.4 * ONE_ATMOSPHERE);
/// Oxygen in the sensed tile outside this share of standard air raises an alert.
pub const ALERT_OXYGEN: (f64, f64) = (0.9, 1.1);
/// Temperature this far from 20 °C, K, raises an alert.
pub const ALERT_TEMPERATURE: f64 = 10.0;
/// Moles of CO2 in the sensed tile that raise an alert.
pub const ALERT_CO2: f64 = 0.05;
/// Moles of plasma in the sensed tile that raise an alert.
pub const ALERT_PLASMA: f64 = 1.0;
/// How long the air must stay dangerous before the seal closes, ticks (Luna `spawn(50)`).
pub const SEAL_DELAY_TICKS: u64 = 100;
/// Continuous draw from the environment channel, W (Luna `use_power(5, ENVIRON)`).
pub const ALARM_POWER: f64 = 5.0;
/// The window stays open within this many tiles (Luna `range(3)`); the buttons need reach.
pub const VIEW_RANGE: i32 = 3;
/// Panic fill stops once the room is back to this share of one atmosphere.
pub const PANIC_FILL_UNTIL: f64 = 0.95;

/// How the air looks to an alarm (Luna `safe`: 2, 1 with `alert_info`, 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Safety {
    #[default]
    Normal,
    Alert,
    Danger,
}

/// A wall-mounted air alarm. It watches the air of the floor tile in front of it.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct AirAlarm {
    /// Disconnected with wirecutters (Luna `BROKEN`).
    pub broken: bool,
    pub safety: Safety,
    /// The tick the air turned dangerous.
    pub danger_since: Option<u64>,
}

/// Areas whose emergency seal is on (Luna `air_doors_activated`).
#[derive(Resource, Debug, Default)]
pub struct AreaSeals(pub HashSet<String>);

/// A request from the alarm window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlarmCommand {
    Seal(bool),
    PanicFill(bool),
}

#[derive(Component, Debug, Default)]
pub struct PendingAlarmOps(pub Vec<(Entity, AlarmCommand)>);

/// What the mob's open alarm window shows, for the server to send when it changes.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct AlarmPanelOut(pub Option<AlarmPanelView>);

/// Luna's alarm `process` checks, in order. Only pressure can make the air dangerous.
pub fn assess(air: &GasMix) -> Safety {
    let pressure = air.pressure(TILE_VOLUME);
    let outside = |(low, high): (f64, f64), v: f64| v < low || v > high;
    if outside(UNSAFE_PRESSURE, pressure) {
        return Safety::Danger;
    }
    let standard_oxygen = GasMix::standard_air(TILE_VOLUME).get(Gas::Oxygen);
    let oxygen = air.get(Gas::Oxygen) / standard_oxygen;
    let alert = outside(ALERT_PRESSURE, pressure)
        || outside(ALERT_OXYGEN, oxygen)
        || (air.temperature - T20C).abs() > ALERT_TEMPERATURE
        || air.get(Gas::CarbonDioxide) > ALERT_CO2
        || air.get(Gas::Plasma) > ALERT_PLASMA;
    if alert { Safety::Alert } else { Safety::Normal }
}

/// The floor tile an alarm on `pos` senses: the first open neighbour in its area (N, S, E,
/// W), or its own tile.
pub fn sensor_tile(map: &TileMap, pos: TilePos) -> TilePos {
    let area = map.area(pos);
    [Dir::North, Dir::South, Dir::East, Dir::West]
        .into_iter()
        .map(|d| pos.step(d))
        .find(|p| !map.is_solid(*p) && map.area(*p) == area)
        .unwrap_or(pos)
}

/// Every machine cycle: read the air, update the light, and seal the area once the air has
/// been dangerous for 5 s. The seal lifts when the air stops being dangerous.
pub fn process_alarms(
    time: Res<WorldTime>,
    atmos: Option<Res<StationAtmos>>,
    map: crate::decks::Decks,
    mut seals: ResMut<AreaSeals>,
    mut alarms: Query<(&GridPos, &mut AirAlarm, &PowerUser, &mut Appearance)>,
) {
    let Some(atmos) = atmos else { return };
    let now = time.tick.0;
    if !now.is_multiple_of(MACHINE_TICKS) {
        return;
    }
    for (pos, mut alarm, power, mut appearance) in &mut alarms {
        let state = if alarm.broken || !power.powered {
            alarm.danger_since = None;
            "alarmp"
        } else {
            let area = map.area(pos.0).unwrap_or_default().to_owned();
            let safety = assess(&atmos.0.air_at(sensor_tile(map.at(pos.0.z), pos.0)));
            if safety == Safety::Danger {
                let since = *alarm.danger_since.get_or_insert(now);
                if now - since >= SEAL_DELAY_TICKS {
                    seals.0.insert(area);
                }
            } else {
                alarm.danger_since = None;
                if alarm.safety == Safety::Danger {
                    seals.0.remove(&area);
                }
            }
            alarm.safety = safety;
            if safety == Safety::Danger {
                "alarm1"
            } else {
                "alarm0"
            }
        };
        if appearance.state != state {
            appearance.state = state.to_owned();
        }
    }
}

/// Whether a seal covers a door at `pos`: its own tile, or a non-wall tile next to it, is in
/// a sealed area.
pub fn covered_by_seal(seals: &AreaSeals, map: &TileMap, pos: TilePos) -> bool {
    covered_by(&seals.0, map, pos)
}

/// Whether a door at `pos` is in or next to one of `areas`: its own tile, or a non-wall tile
/// next to it, is in one.
pub fn covered_by(areas: &HashSet<String>, map: &TileMap, pos: TilePos) -> bool {
    !areas.is_empty()
        && [
            None,
            Some(Dir::North),
            Some(Dir::South),
            Some(Dir::East),
            Some(Dir::West),
        ]
        .into_iter()
        .map(|d| d.map_or(pos, |d| pos.step(d)))
        .filter(|p| *p == pos || !map.is_solid(*p))
        .filter_map(|p| map.area(p))
        .any(|area| areas.contains(area))
}

/// Close and bolt every powered door in or next to a sealed area; raise the bolts the seal
/// dropped once no seal covers the door (Luna `air_doors_close` / `air_doors_open`).
/// Unpowered doors are left alone either way.
pub fn apply_seals(
    time: Res<WorldTime>,
    seals: Res<AreaSeals>,
    map: crate::decks::Decks,
    mut doors: Query<(&GridPos, &mut Airlock)>,
) {
    let now = time.deciseconds();
    for (pos, mut door) in &mut doors {
        let covered = covered_by_seal(&seals, map.at(pos.0.z), pos.0);
        if covered && !door.in_seal {
            door.in_seal = true;
            door.seal_pending = true;
        } else if !covered && door.in_seal {
            door.in_seal = false;
            door.seal_pending = false;
            if door.air_locked && door.has_power() {
                door.bolted = false;
                door.air_locked = false;
            }
        }
        if !door.seal_pending {
            continue;
        }
        if !door.has_power() {
            door.seal_pending = false;
            continue;
        }
        match door.state {
            DoorState::Closed => {
                // Don't take over bolts dropped for another reason.
                if !door.bolted {
                    door.bolted = true;
                    door.air_locked = true;
                }
                door.seal_pending = false;
            }
            DoorState::Open { .. } => {
                if !door.toggle(now) {
                    door.seal_pending = false;
                }
            }
            DoorState::Opening { .. } | DoorState::Closing { .. } => {}
        }
    }
}

/// Apply alarm window requests: the alarm must work and be in reach.
pub fn process_alarm_ops(
    map: crate::decks::Decks,
    mut seals: ResMut<AreaSeals>,
    mut mobs: Query<(&GridPos, &mut PendingAlarmOps, Option<&Vitals>)>,
    alarms: Query<(&GridPos, &AirAlarm, &PowerUser)>,
    mut vents: Query<(&GridPos, &mut Vent)>,
) {
    for (pos, mut pending, vitals) in &mut mobs {
        let awake = vitals.is_none_or(|v| v.stat == MobStat::Conscious);
        for (target, command) in pending.0.drain(..) {
            let Ok((alarm_pos, alarm, power)) = alarms.get(target) else {
                continue;
            };
            if !awake || alarm.broken || !power.powered || !in_reach(pos.0, alarm_pos.0) {
                continue;
            }
            let area = map.area(alarm_pos.0).unwrap_or_default().to_owned();
            match command {
                AlarmCommand::Seal(true) => {
                    seals.0.insert(area);
                }
                AlarmCommand::Seal(false) => {
                    seals.0.remove(&area);
                }
                AlarmCommand::PanicFill(on) => {
                    for (vent_pos, mut vent) in &mut vents {
                        if map.area(vent_pos.0) == Some(area.as_str()) {
                            vent.panic = on;
                        }
                    }
                }
            }
        }
    }
}

/// Toggle an alarm with wirecutters (Luna `attackby`). Returns the message everyone who
/// can see it gets.
pub fn cut_alarm(alarm: &mut AirAlarm, name: &str) -> LocMsg {
    alarm.broken = !alarm.broken;
    let id = if alarm.broken {
        "alarm-deactivated"
    } else {
        "alarm-reactivated"
    };
    LocMsg::new(id).arg("name", name).arg("item", "air-alarm")
}

/// Keep each viewer's alarm window in step with the alarm and the room.
pub fn update_alarm_views(
    atmos: Option<Res<StationAtmos>>,
    map: crate::decks::Decks,
    seals: Res<AreaSeals>,
    mut viewers: Query<(
        Entity,
        &GridPos,
        Option<&PanelViewer>,
        &mut AlarmPanelOut,
        Option<&Vitals>,
    )>,
    alarms: Query<(&GridPos, &AirAlarm, &PowerUser, &NetId)>,
    vents: Query<(&GridPos, &Vent)>,
    mut commands: Commands,
) {
    let Some(atmos) = atmos else { return };
    for (mob, pos, viewer, mut out, vitals) in &mut viewers {
        let target = viewer.and_then(|v| alarms.get(v.0).ok());
        let view = target.and_then(|(alarm_pos, alarm, power, id)| {
            let near = alarm_pos.0.z == pos.0.z
                && (alarm_pos.0.x - pos.0.x).abs() <= VIEW_RANGE
                && (alarm_pos.0.y - pos.0.y).abs() <= VIEW_RANGE;
            let usable =
                near && !alarm.broken && power.powered && vitals.is_none_or(|v| v.stat == MobStat::Conscious);
            if !usable {
                return None;
            }
            let area = map.area(alarm_pos.0).unwrap_or_default().to_owned();
            let air = atmos.0.air_at(sensor_tile(map.at(alarm_pos.0.z), alarm_pos.0));
            let pressure = air.pressure(TILE_VOLUME);
            let area_vents: Vec<bool> = vents
                .iter()
                .filter(|(p, _)| map.area(p.0) == Some(area.as_str()))
                .map(|(_, v)| v.panic)
                .collect();
            Some(AlarmPanelView {
                target: *id,
                pressure_dkpa: (pressure * 10.0).round().max(0.0) as u32,
                pressure_alert: (pressure - ONE_ATMOSPHERE).abs() >= 10.0,
                readings: readings(&air),
                sealed: seals.0.contains(&area),
                panic_fill: (!area_vents.is_empty()).then(|| area_vents.contains(&true)),
                controls: in_reach(pos.0, alarm_pos.0),
                area,
            })
        });
        // Close the window when it can no longer be used; leave other panels alone.
        if view.is_none() && target.is_some() {
            commands.entity(mob).remove::<PanelViewer>();
        }
        if out.0 != view {
            out.0 = view;
        }
    }
}

/// The alarm's gas and temperature readouts (Luna `attack_hand`). Shares are of all moles.
pub fn readings(air: &GasMix) -> Vec<AlarmReading> {
    let total = air.total_moles();
    if total <= 0.0 {
        return Vec::new();
    }
    let share = |gas| air.get(gas) / total;
    let (oxygen, nitrogen) = (share(Gas::Oxygen), share(Gas::Nitrogen));
    let (co2, plasma) = (share(Gas::CarbonDioxide), share(Gas::Plasma));
    let unknown = 1.0 - (oxygen + nitrogen + co2 + plasma);
    let standard = GasMix::standard_air(1.0);
    let standard_share = |gas| standard.get(gas) / standard.total_moles();
    let percent = |v: f64| (v * 100.0).round() as i32;
    let reading = |id: &str, value: i32, alert: bool| AlarmReading {
        id: id.to_owned(),
        value,
        alert,
    };
    let mut out = vec![
        reading(
            "alarm-nitrogen",
            percent(nitrogen),
            (nitrogen - standard_share(Gas::Nitrogen)).abs() >= 0.20,
        ),
        reading(
            "alarm-oxygen",
            percent(oxygen),
            (oxygen - standard_share(Gas::Oxygen)).abs() >= 0.02,
        ),
        reading("alarm-co2", percent(co2), co2 > 0.01),
    ];
    if plasma > 0.01 {
        out.push(reading("alarm-plasma", percent(plasma), true));
    }
    if unknown > 0.01 {
        out.push(reading("alarm-unknown", percent(unknown), true));
    }
    out.push(reading(
        "alarm-temperature",
        (air.temperature - T0C).round() as i32,
        (air.temperature - T20C).abs() >= 7.0,
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_air_is_normal() {
        let air = GasMix::standard_air(TILE_VOLUME);
        assert_eq!(assess(&air), Safety::Normal);
        assert!(readings(&air).iter().all(|r| !r.alert), "{:?}", readings(&air));
    }

    #[test]
    fn thin_air_alerts_then_is_dangerous() {
        let air = GasMix::standard_air(TILE_VOLUME);
        assert_eq!(assess(&air.scaled(0.85)), Safety::Alert);
        assert_eq!(assess(&air.scaled(0.6)), Safety::Danger);
        assert_eq!(assess(&air.scaled(1.5)), Safety::Danger);
        assert_eq!(assess(&GasMix::default()), Safety::Danger);
    }

    #[test]
    fn carbon_dioxide_alerts_but_is_not_dangerous() {
        let mut air = GasMix::standard_air(TILE_VOLUME);
        air.set(Gas::CarbonDioxide, 1.0);
        assert_eq!(assess(&air), Safety::Alert);
    }
}
