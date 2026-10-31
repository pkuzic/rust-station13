//! Fire alarms: pull one, or let it feel the heat, and every firedoor around its area
//! closes. Spec: `docs/reference/fire-alarm.md`.

use std::collections::HashSet;

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_i18n::LocMsg;
use rs13_simulation::atmos::gas::T0C;
use rs13_world::{Appearance, GridPos, MapLayer, ObjectSpec, TilePos};

use crate::actions::ClickOutcome;
use crate::atmos::StationAtmos;
use crate::machines::MACHINE_TICKS;
use crate::power::{Channel, PowerUser};
use crate::time::WorldTime;

/// The fire alarm prototype.
pub const FIRE_ALARM: &str = "fire-alarm";
/// Air hotter than this, K, sets off a detecting alarm (Luna `T0C+200`).
pub const DETECT_TEMPERATURE: f64 = T0C + 200.0;
/// Continuous draw from the environment channel, W (Luna `use_power(10, ENVIRON)`).
pub const FIRE_ALARM_POWER: f64 = 10.0;

/// A wall-mounted fire alarm.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FireAlarm {
    /// Its heat detector is connected (Luna `detecting`); wirecutters toggle it.
    pub detecting: bool,
}

/// Areas whose fire alarm has gone off (Luna area `fire`).
#[derive(Resource, Debug, Default)]
pub struct AreaFires(pub HashSet<String>);

pub fn spawn_fire_alarm(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: rs13_world::NetId) -> Entity {
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(spec.id.as_str(), "fire0"),
            FireAlarm { detecting: true },
            PowerUser::new(Channel::Environment, FIRE_ALARM_POWER),
        ))
        .id()
}

/// Everything clicking a fire alarm needs.
#[derive(SystemParam)]
pub struct FireAlarmWork<'w, 's> {
    alarms: Query<'w, 's, (&'static GridPos, &'static mut FireAlarm, &'static PowerUser)>,
    fires: ResMut<'w, AreaFires>,
    map: Single<'w, 's, &'static MapLayer>,
}

/// What clicking a fire alarm did: for the clicker, and for everyone who sees it.
pub struct FireAlarmOutcome {
    pub to_mob: ClickOutcome,
    pub seen: Option<LocMsg>,
}

impl FireAlarmWork<'_, '_> {
    pub fn contains(&self, target: Entity) -> bool {
        self.alarms.contains(target)
    }

    /// Luna `attack_hand` and `attackby`. An empty hand works the panel's lockdown button:
    /// it raises the alarm, or resets it if it is up. Wirecutters disconnect or reconnect
    /// the heat detector. Anything else raises it. The caller checks reach.
    pub fn use_on(&mut self, target: Entity, held: Option<&str>, name: &str) -> Option<FireAlarmOutcome> {
        let (pos, mut alarm, power) = self.alarms.get_mut(target).ok()?;
        let area = self.map.0.area(pos.0).map(str::to_owned);
        if held == Some("wirecutters") {
            alarm.detecting = !alarm.detecting;
            let id = if alarm.detecting {
                "fire-alarm-reconnected"
            } else {
                "fire-alarm-disconnected"
            };
            return Some(FireAlarmOutcome {
                to_mob: ClickOutcome::default(),
                seen: Some(LocMsg::new(id).arg("name", name).arg("item", FIRE_ALARM)),
            });
        }
        let Some(area) = area.filter(|_| power.powered) else {
            return Some(FireAlarmOutcome {
                to_mob: ClickOutcome::default(),
                seen: None,
            });
        };
        let to_mob = if held.is_none() && self.fires.0.contains(&area) {
            self.fires.0.remove(&area);
            ClickOutcome::told("fire-alarm-reset")
        } else {
            self.fires.0.insert(area);
            ClickOutcome::told("fire-alarm-raised")
        };
        Some(FireAlarmOutcome { to_mob, seen: None })
    }
}

/// Every machine cycle, a powered, detecting alarm in air hotter than 200 °C goes off (Luna
/// `temperature_expose`). Every tick, it shows whether it has power and whether its area
/// is on alert.
pub fn process_fire_alarms(
    time: Res<WorldTime>,
    atmos: Option<Res<StationAtmos>>,
    map: Single<&MapLayer>,
    mut fires: ResMut<AreaFires>,
    mut alarms: Query<(&GridPos, &FireAlarm, &PowerUser, &mut Appearance)>,
) {
    let cycle = time.tick.0.is_multiple_of(MACHINE_TICKS);
    for (pos, alarm, power, mut look) in &mut alarms {
        let area = map.0.area(pos.0);
        if cycle
            && power.powered
            && alarm.detecting
            && let (Some(area), Some(atmos)) = (area, atmos.as_ref())
        {
            let air = atmos.0.air_at(crate::alarms::sensor_tile(&map.0, pos.0));
            if air.temperature > DETECT_TEMPERATURE {
                fires.0.insert(area.to_owned());
            }
        }
        let state = if !power.powered {
            "firep"
        } else if area.is_some_and(|a| fires.0.contains(a)) {
            "fire1"
        } else {
            "fire0"
        };
        if look.state != state {
            look.state = state.to_owned();
        }
    }
}
