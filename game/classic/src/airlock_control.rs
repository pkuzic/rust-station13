//! Airlocks that cycle (Luna's `embedded_controller`): a console works two doors and, for an
//! airlock to space, the pump between them, and buttons beside the doors ask it to cycle.
//! Spec: `docs/reference/airlock-control.md`.

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, GridPos, NetId, ObjectSpec, TilePos};

use crate::actions::Outbox;
use crate::atmos::StationAtmos;
use crate::doors::{Airlock, DoorState};
use crate::machines::{MACHINE_TICKS, Switch, Vent};
use crate::time::WorldTime;

pub const AIRLOCK_CONSOLE: &str = "airlock-console";
pub const ACCESS_CONSOLE: &str = "access-console";
pub const ACCESS_BUTTON: &str = "access-button";
/// Pressure the chamber must reach before a door opens, kPa (Luna's 95% and 5% of an atmosphere).
pub const PRESSURIZED: f64 = 101.325 * 0.95;
pub const DEPRESSURIZED: f64 = 101.325 * 0.05;

/// The door a console knows by this tag (Luna `id_tag`).
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct DoorTag(pub String);

/// The pump a console knows by this tag (Luna `id`).
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct PumpTag(pub String);

/// Where a pumped airlock is in its cycle (Luna `AIRLOCK_STATE_*`), inside to outside.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Cycle {
    InOpen,
    Pressurize,
    Closed,
    Depressurize,
    OutOpen,
}

/// Which side an access lock is open to (Luna `ACCESS_STATE_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Access {
    Internal,
    Locked,
    External,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Program {
    /// Two doors and a pump: never both open, and the air changes between them.
    Pumped {
        pump: String,
        state: Cycle,
        target: Cycle,
    },
    /// Two doors and no pump: only one unlocked at a time.
    Access { state: Access, target: Access },
}

/// A console.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct AirlockConsole {
    pub id: String,
    pub exterior: String,
    pub interior: String,
    pub program: Program,
}

/// What a button tells its console.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct AccessButton {
    pub master: String,
    pub command: String,
}

pub fn is_airlock_control(proto: &str) -> bool {
    matches!(proto, AIRLOCK_CONSOLE | ACCESS_CONSOLE | ACCESS_BUTTON)
}

pub fn spawn_part(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    let var = |name: &str| {
        spec.vars
            .get(name)
            .map(|v| v.trim().trim_matches('"').to_owned())
            .unwrap_or_default()
    };
    if spec.id == ACCESS_BUTTON {
        return world
            .spawn((
                GridPos(pos),
                id,
                Appearance::new(ACCESS_BUTTON, "access_button_standby"),
                AccessButton {
                    master: var("master_tag"),
                    command: var("command"),
                },
            ))
            .id();
    }
    let (program, look) = if spec.id == ACCESS_CONSOLE {
        (
            Program::Access {
                state: Access::Locked,
                target: Access::Locked,
            },
            "access_control_standby",
        )
    } else {
        // Luna starts with the inside open.
        (
            Program::Pumped {
                pump: var("airpump_tag"),
                state: Cycle::Closed,
                target: Cycle::InOpen,
            },
            "airlock_control_standby",
        )
    };
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(spec.id.clone(), look),
            AirlockConsole {
                id: var("id_tag"),
                exterior: var("exterior_door_tag"),
                interior: var("interior_door_tag"),
                program,
            },
        ))
        .id()
}

/// Luna's `command`s: which way a console should go. A plain `cycle` goes to the other side.
fn command(console: &mut AirlockConsole, command: &str) {
    match &mut console.program {
        Program::Pumped { state, target, .. } => {
            *target = match command {
                "cycle_interior" => Cycle::InOpen,
                "cycle_exterior" => Cycle::OutOpen,
                "cycle" if *state < Cycle::Closed => Cycle::OutOpen,
                "cycle" => Cycle::InOpen,
                _ => *target,
            };
        }
        Program::Access { state, target } => {
            *target = match command {
                "cycle_interior" => Access::Internal,
                "cycle_exterior" => Access::External,
                "cycle" if *state < Access::Locked => Access::External,
                "cycle" => Access::Internal,
                _ => *target,
            };
        }
    }
}

/// A click on a button or console (Luna `access_button/attack_hand`; a console's window is only
/// its cycle command here).
pub fn click(world: &mut World, mob: Entity, target: Entity) {
    let (master, order) = if let Some(button) = world.get::<AccessButton>(target) {
        (button.master.clone(), button.command.clone())
    } else if let Some(console) = world.get::<AirlockConsole>(target) {
        (console.id.clone(), "cycle".to_owned())
    } else {
        return;
    };
    let mut consoles = world.query::<&mut AirlockConsole>();
    let mut found = false;
    for mut console in consoles.iter_mut(world) {
        if console.id == master {
            command(&mut console, &order);
            found = true;
        }
    }
    if found && let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(LocMsg::new("airlock-cycling"));
    }
}

/// Luna `secure_close`: shut it and bolt it.
fn secure_close(door: &mut Airlock, now: f32) {
    door.bolted = false;
    if matches!(door.state, DoorState::Open { .. }) {
        door.toggle(now);
    }
    door.bolted = true;
}

/// Luna `secure_open`: open it and bolt it open.
fn secure_open(door: &mut Airlock, now: f32) {
    door.bolted = false;
    if door.state == DoorState::Closed {
        door.toggle(now);
    }
    door.bolted = true;
}

/// What a door reports (Luna `door_status`, `lock_status`).
fn shut(door: &Airlock) -> bool {
    door.state == DoorState::Closed
}

/// One cycle of each console's program (Luna `embedded_program/process`).
pub fn run_consoles(world: &mut World) {
    if !world.resource::<WorldTime>().tick.0.is_multiple_of(MACHINE_TICKS) {
        return;
    }
    let now = world.resource::<WorldTime>().deciseconds();
    let mut tagged = world.query::<(Entity, &DoorTag)>();
    let doors: HashMap<String, Entity> = tagged.iter(world).map(|(e, t)| (t.0.clone(), e)).collect();
    let mut tagged = world.query::<(Entity, &PumpTag, &GridPos)>();
    let pumps: HashMap<String, (Entity, TilePos)> = tagged
        .iter(world)
        .map(|(e, t, p)| (t.0.clone(), (e, p.0)))
        .collect();
    let mut consoles = world.query::<(Entity, &AirlockConsole)>();
    let all: Vec<(Entity, AirlockConsole)> = consoles.iter(world).map(|(e, c)| (e, c.clone())).collect();
    for (entity, mut console) in all {
        let (Some(&outer), Some(&inner)) = (doors.get(&console.exterior), doors.get(&console.interior))
        else {
            continue;
        };
        let busy = match &mut console.program {
            Program::Pumped { pump, state, target } => {
                let Some(&(pump, at)) = pumps.get(pump.as_str()) else {
                    continue;
                };
                let pressure = world.resource::<StationAtmos>().0.pressure_at(at);
                step_pumped(world, now, state, *target, (inner, outer), pump, pressure);
                *state != *target
            }
            Program::Access { state, target } => {
                step_access(world, now, state, *target, (inner, outer));
                *state != *target
            }
        };
        let kind = match console.program {
            Program::Pumped { .. } => "airlock_control",
            Program::Access { .. } => "access_control",
        };
        let look = format!("{kind}_{}", if busy { "process" } else { "standby" });
        if let Some(mut appearance) = world.get_mut::<Appearance>(entity)
            && appearance.state != look
        {
            appearance.state = look;
        }
        if let Some(mut live) = world.get_mut::<AirlockConsole>(entity) {
            live.program = console.program;
        }
    }
}

fn set_pump(world: &mut World, pump: Entity, on: bool, siphon: bool) {
    if let Some(mut switch) = world.get_mut::<Switch>(pump)
        && switch.0 != on
    {
        switch.0 = on;
    }
    if let Some(mut vent) = world.get_mut::<Vent>(pump) {
        vent.siphon = siphon;
    }
}

fn with_door(world: &mut World, door: Entity, act: impl FnOnce(&mut Airlock)) {
    if let Some(mut door) = world.get_mut::<Airlock>(door) {
        act(&mut door);
    }
}

fn is_shut(world: &World, door: Entity) -> bool {
    world.get::<Airlock>(door).is_some_and(shut)
}

fn step_pumped(
    world: &mut World,
    now: f32,
    state: &mut Cycle,
    target: Cycle,
    (inner, outer): (Entity, Entity),
    pump: Entity,
    pressure: f64,
) {
    match *state {
        Cycle::InOpen => {
            if target > *state {
                if is_shut(world, inner) {
                    *state = Cycle::Closed;
                } else {
                    with_door(world, inner, |d| secure_close(d, now));
                }
            } else {
                set_pump(world, pump, false, false);
            }
        }
        Cycle::Pressurize => {
            if target < *state {
                if pressure >= PRESSURIZED {
                    with_door(world, inner, |d| secure_open(d, now));
                    *state = Cycle::InOpen;
                } else {
                    set_pump(world, pump, true, false);
                }
            } else if target > *state {
                *state = Cycle::Closed;
            }
        }
        Cycle::Closed => {
            if target > *state {
                let was_shut = is_shut(world, inner);
                with_door(world, inner, |d| secure_close(d, now));
                if was_shut {
                    *state = Cycle::Depressurize;
                }
            } else if target < *state {
                let was_shut = is_shut(world, outer);
                with_door(world, outer, |d| secure_close(d, now));
                if was_shut {
                    *state = Cycle::Pressurize;
                }
            } else {
                set_pump(world, pump, false, false);
            }
        }
        Cycle::Depressurize => {
            if pressure <= DEPRESSURIZED {
                if target > *state {
                    with_door(world, outer, |d| secure_open(d, now));
                    *state = Cycle::OutOpen;
                } else if target < *state {
                    *state = Cycle::Closed;
                }
            } else if target < *state {
                *state = Cycle::Closed;
            } else {
                set_pump(world, pump, true, true);
            }
        }
        Cycle::OutOpen => {
            if target < *state {
                if is_shut(world, outer) {
                    *state = Cycle::Closed;
                } else {
                    with_door(world, outer, |d| secure_close(d, now));
                }
            } else {
                set_pump(world, pump, false, false);
            }
        }
    }
}

/// Closed and bolted; else close or bolt it on the way there.
fn lock(world: &mut World, door: Entity, now: f32) -> bool {
    let locked = world.get::<Airlock>(door).is_some_and(|d| shut(d) && d.bolted);
    if !locked {
        with_door(world, door, |d| secure_close(d, now));
    }
    locked
}

fn unlock(world: &mut World, door: Entity) -> bool {
    let free = world.get::<Airlock>(door).is_some_and(|d| !d.bolted);
    with_door(world, door, |d| d.bolted = false);
    free
}

fn step_access(
    world: &mut World,
    now: f32,
    state: &mut Access,
    target: Access,
    (inner, outer): (Entity, Entity),
) {
    match *state {
        Access::Internal => {
            if target > *state && lock(world, inner, now) {
                *state = Access::Locked;
            }
        }
        Access::External => {
            if target < *state && lock(world, outer, now) {
                *state = Access::Locked;
            }
        }
        Access::Locked => {
            if target < *state {
                if lock(world, outer, now) && unlock(world, inner) {
                    *state = Access::Internal;
                }
            } else if target > *state {
                if lock(world, inner, now) && unlock(world, outer) {
                    *state = Access::External;
                }
            } else if lock(world, inner, now) {
                lock(world, outer, now);
            }
        }
    }
}
