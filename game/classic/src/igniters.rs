//! Igniters: a floor-mounted spark plug that, while on, keeps touching the air with a hot
//! spark, and the wall switch that turns every igniter with its id on or off. Burn chambers
//! and incinerators light their gas with them. Spec: `docs/reference/igniters.md`.

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_world::{Appearance, GridPos, NetId, ObjectSpec, TilePos};

use crate::actions::ClickOutcome;
use crate::atmos::StationAtmos;
use crate::machines::MACHINE_TICKS;
use crate::power::{Channel, PowerUser};
use crate::time::WorldTime;

/// The floor-mounted igniter prototype.
pub const IGNITER: &str = "igniter";
/// The wall switch prototype.
pub const IGNITION_SWITCH: &str = "ignition-switch";
/// How hot an igniter's spark is, K (Luna `hotspot_expose(6000, …)`).
pub const SPARK_TEMPERATURE: f64 = 6000.0;
/// A pressed switch stays down this long before it can be pressed again, ds (Luna `sleep(50)`).
pub const SWITCH_COOLDOWN_DS: f32 = 50.0;

/// A floor-mounted igniter, and the id its switches share.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct Igniter {
    pub id: String,
    pub on: bool,
}

/// A wall-mounted ignition switch for the igniters with its id.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct IgnitionSwitch {
    pub id: String,
    /// While pressed: when it comes back up, ds.
    pub pressed_until: Option<f32>,
}

/// Luna's map sets `on`, `0` or `1`; it starts on if unset (Luna `on = 1`).
fn var_on(spec: &ObjectSpec) -> bool {
    spec.vars.get("on").is_none_or(|v| v.trim() != "0")
}

fn var_id(spec: &ObjectSpec) -> String {
    spec.vars.get("id").cloned().unwrap_or_default()
}

pub fn spawn_igniter(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    let igniter = Igniter {
        id: var_id(spec),
        on: var_on(spec),
    };
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(IGNITER, igniter_look(&igniter, true)),
            igniter,
            PowerUser::new(Channel::Equipment, 0.0),
        ))
        .id()
}

pub fn spawn_ignition_switch(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(IGNITION_SWITCH, "launcherbtt"),
            IgnitionSwitch {
                id: var_id(spec),
                pressed_until: None,
            },
            PowerUser::new(Channel::Equipment, 0.0),
        ))
        .id()
}

/// `igniter1` while on and powered, else `igniter0` (Luna `power_change`).
fn igniter_look(igniter: &Igniter, powered: bool) -> &'static str {
    if igniter.on && powered {
        "igniter1"
    } else {
        "igniter0"
    }
}

/// Everything clicking igniters and their switches needs.
#[derive(SystemParam)]
pub struct IgniterWork<'w, 's> {
    igniters: Query<'w, 's, (&'static mut Igniter, &'static PowerUser)>,
    switches: Query<'w, 's, (&'static mut IgnitionSwitch, &'static PowerUser), Without<Igniter>>,
}

impl IgniterWork<'_, '_> {
    pub fn contains(&self, target: Entity) -> bool {
        self.igniters.contains(target) || self.switches.contains(target)
    }

    /// Luna `igniter/attack_hand` flips an igniter, powered or not. Luna
    /// `ignition_switch/attack_hand`: a powered switch that isn't already down goes down
    /// for five seconds and flips every igniter with its id. Anything in the hand works as
    /// a hand. Nobody is told anything. The caller checks reach.
    pub fn use_on(&mut self, target: Entity, now_ds: f32) -> Option<ClickOutcome> {
        if let Ok((mut igniter, _)) = self.igniters.get_mut(target) {
            igniter.on = !igniter.on;
            return Some(ClickOutcome::default());
        }
        let (mut switch, power) = self.switches.get_mut(target).ok()?;
        if !power.powered || switch.pressed_until.is_some() {
            return Some(ClickOutcome::default());
        }
        switch.pressed_until = Some(now_ds + SWITCH_COOLDOWN_DS);
        let id = switch.id.clone();
        for (mut igniter, _) in &mut self.igniters {
            if igniter.id == id {
                igniter.on = !igniter.on;
            }
        }
        Some(ClickOutcome::default())
    }
}

/// Every machine cycle a powered igniter that is on touches the air on its tile with its
/// spark (Luna `igniter/process`). Pressed switches come back up when their time is over.
pub fn run_igniters(
    time: Res<WorldTime>,
    atmos: Option<ResMut<StationAtmos>>,
    igniters: Query<(&Igniter, &GridPos, &PowerUser)>,
    mut switches: Query<&mut IgnitionSwitch>,
) {
    let now = time.deciseconds();
    for mut switch in &mut switches {
        if switch.pressed_until.is_some_and(|t| now >= t) {
            switch.pressed_until = None;
        }
    }
    let Some(mut atmos) = atmos else { return };
    if !time.tick.0.is_multiple_of(MACHINE_TICKS) {
        return;
    }
    for (igniter, pos, power) in &igniters {
        if igniter.on && power.powered {
            atmos.0.expose(pos.0, SPARK_TEMPERATURE);
        }
    }
}

pub fn update_igniter_looks(
    mut igniters: Query<(&Igniter, &PowerUser, &mut Appearance), Without<IgnitionSwitch>>,
    mut switches: Query<(&IgnitionSwitch, &mut Appearance), Without<Igniter>>,
) {
    for (igniter, power, mut look) in &mut igniters {
        let state = igniter_look(igniter, power.powered);
        if look.state != state {
            look.state = state.to_owned();
        }
    }
    for (switch, mut look) in &mut switches {
        let state = if switch.pressed_until.is_some() {
            "launcheract"
        } else {
            "launcherbtt"
        };
        if look.state != state {
            look.state = state.to_owned();
        }
    }
}
