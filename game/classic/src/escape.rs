//! Getting off the station: the communications console calls the escape pods, which launch after a
//! countdown, and the round is over when they arrive (Luna's `shuttle_controller`, `PodControl`,
//! `computer/communications`). Spec: `docs/reference/round.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{
    Appearance, CommsPanelView, Dense, DisplayName, GridPos, MapLayer, MobStat, NetId, TilePos,
};

use crate::access::Shown;
use crate::actions::{Outbox, in_reach};
use crate::doors::{Airlock, DoorState, SHUTTLE_DOOR};
use crate::health::Vitals;
use crate::jobs::HasJob;
use crate::movement::Mob;
use crate::power::{Channel, PowerUser};
use crate::time::WorldTime;
use crate::wires::PanelViewer;

pub const COMMS_CONSOLE: &str = "comms-console";
/// The access that calls the pods (Luna `req_access`).
pub const COMMS_ACCESS: &str = "heads";
/// How long from the call to the launch, and the launch to arriving, ds (Luna `PODLAUNCHTIME`,
/// `PODTRANSITTIME`).
pub const POD_LAUNCH_DS: f32 = 6000.0;
pub const POD_TRANSIT_DS: f32 = 1500.0;
/// Too close to launching to stop it, ds (Luna `timeleft() < 30`).
pub const NO_RECALL_DS: f32 = 300.0;
/// What the escape pods' areas are called (`map-import.md`).
const POD_AREA: &str = "shuttle-escape-station-pod";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PodState {
    /// Waiting, with their doors bolted.
    Docked,
    /// Called: launching at this time, ds; the last time told, ds.
    Launching {
        at: f32,
        told_at_left: f32,
    },
    /// Gone: arriving at this time, ds.
    Transit {
        at: f32,
    },
    Arrived,
}

/// The escape pods.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct EscapePods {
    pub state: PodState,
    pub launch_ds: f32,
    pub transit_ds: f32,
    bolted: bool,
}

impl Default for EscapePods {
    fn default() -> Self {
        Self {
            state: PodState::Docked,
            launch_ds: POD_LAUNCH_DS,
            transit_ds: POD_TRANSIT_DS,
            bolted: false,
        }
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommsConsole;

/// What a mob asked of a console this tick: to call the pods, or to stop them.
#[derive(Component, Debug, Default)]
pub struct PendingCommsOps(pub Vec<(Entity, bool)>);

/// What a mob's open console window shows.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct CommsPanelOut(pub Option<CommsPanelView>);

pub fn spawn_console(world: &mut World, pos: TilePos, id: NetId) -> Entity {
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(COMMS_CONSOLE, "comm"),
            Dense,
            CommsConsole,
            PowerUser::new(Channel::Equipment, 100.0),
        ))
        .id()
}

/// A click opens it, with power.
pub fn click(world: &mut World, mob: Entity, console: Entity) {
    if world.get::<PowerUser>(console).is_some_and(|p| p.powered) {
        world.entity_mut(mob).insert(PanelViewer(console));
    }
}

fn tell_everyone(world: &mut World, msg: &LocMsg) {
    let mut people = world.query::<&mut Outbox>();
    for mut outbox in people.iter_mut(world) {
        outbox.0.push(msg.clone());
    }
}

fn in_pod(world: &mut World, at: TilePos) -> bool {
    let mut maps = world.query::<&MapLayer>();
    maps.iter(world)
        .find_map(|m| {
            if m.0.z == at.z {
                m.0.area(at).map(str::to_owned)
            } else {
                None
            }
        })
        .is_some_and(|a| a.starts_with(POD_AREA))
}

/// Every pod's doors, as (door, tile).
fn pod_doors(world: &mut World) -> Vec<Entity> {
    let mut doors = world.query_filtered::<(Entity, &GridPos, &Appearance), With<Airlock>>();
    let all: Vec<(Entity, TilePos)> = doors
        .iter(world)
        .filter(|(_, _, a)| a.sprite == SHUTTLE_DOOR)
        .map(|(e, p, _)| (e, p.0))
        .collect();
    all.into_iter()
        .filter(|(_, at)| in_pod(world, *at))
        .map(|(e, _)| e)
        .collect()
}

fn set_bolts(world: &mut World, on: bool) {
    for door in pod_doors(world) {
        if let Some(mut airlock) = world.get_mut::<Airlock>(door) {
            airlock.bolted = on;
        }
    }
}

/// Call the pods, or stop them (Luna `call_shuttle_proc`, `cancel_call_proc`).
fn apply(world: &mut World, mob: Entity, call: bool, now: f32) {
    if !Shown::of_mob(world, mob).opens(&[COMMS_ACCESS.to_owned()]) {
        if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
            outbox.0.push(LocMsg::new("comms-denied"));
        }
        return;
    }
    let pods = *world.resource::<EscapePods>();
    match (call, pods.state) {
        (true, PodState::Docked) => {
            world.resource_mut::<EscapePods>().state = PodState::Launching {
                at: now + pods.launch_ds,
                told_at_left: pods.launch_ds,
            };
            set_bolts(world, false);
            let minutes = (pods.launch_ds / 600.0).round() as i64;
            tell_everyone(world, &LocMsg::new("pods-called").num("value", minutes));
        }
        (true, PodState::Launching { .. }) => {
            if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
                outbox.0.push(LocMsg::new("pods-already-called"));
            }
        }
        (false, PodState::Launching { at, .. }) if at - now >= NO_RECALL_DS => {
            world.resource_mut::<EscapePods>().state = PodState::Docked;
            set_bolts(world, true);
            tell_everyone(world, &LocMsg::new("pods-recalled"));
        }
        _ => {}
    }
}

/// How often the countdown is told, by how long is left (Luna `PodControl/process`).
fn next_told(left: f32) -> f32 {
    if left > 600.0 {
        left - 600.0
    } else if left > 300.0 {
        left - 100.0
    } else {
        left - 50.0
    }
}

/// The console's buttons, the countdown, the launch and the arrival.
pub fn run_escape(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    if !world.resource::<EscapePods>().bolted {
        // Luna's map locks the pods' doors until they're called.
        set_bolts(world, true);
        world.resource_mut::<EscapePods>().bolted = true;
    }
    let mut askers = world.query::<(Entity, &GridPos, &mut PendingCommsOps)>();
    let asked: Vec<(Entity, TilePos, Vec<(Entity, bool)>)> = askers
        .iter_mut(world)
        .filter(|(.., ops)| !ops.0.is_empty())
        .map(|(e, at, mut ops)| (e, at.0, std::mem::take(&mut ops.0)))
        .collect();
    for (mob, at, ops) in asked {
        for (console, call) in ops {
            let near = world.get::<GridPos>(console).is_some_and(|p| in_reach(at, p.0));
            let powered = world.get::<PowerUser>(console).is_some_and(|p| p.powered);
            if near && powered {
                apply(world, mob, call, now);
            }
        }
    }
    let pods = *world.resource::<EscapePods>();
    match pods.state {
        PodState::Launching { at, told_at_left } => {
            let left = at - now;
            if left <= 0.0 {
                world.resource_mut::<EscapePods>().state = PodState::Transit {
                    at: now + pods.transit_ds,
                };
                tell_everyone(world, &LocMsg::new("pods-launched"));
                let doors = pod_doors(world);
                for door in doors {
                    if let Some(mut airlock) = world.get_mut::<Airlock>(door)
                        && matches!(airlock.state, DoorState::Open { .. })
                    {
                        airlock.toggle(now);
                    }
                }
            } else if left < told_at_left {
                let msg = if left >= 600.0 {
                    LocMsg::new("pods-minutes").num("value", (left / 600.0).round() as i64)
                } else {
                    LocMsg::new("pods-seconds").num("value", (left / 10.0).round() as i64)
                };
                tell_everyone(world, &msg);
                world.resource_mut::<EscapePods>().state = PodState::Launching {
                    at,
                    told_at_left: next_told(left),
                };
            }
        }
        PodState::Transit { at } => {
            // Once the doors have swung shut, nobody gets in or out.
            let doors = pod_doors(world);
            for door in doors {
                if let Some(mut airlock) = world.get_mut::<Airlock>(door)
                    && airlock.state == DoorState::Closed
                {
                    airlock.bolted = true;
                }
            }
            if now >= at {
                world.resource_mut::<EscapePods>().state = PodState::Arrived;
                declare_completion(world);
            }
        }
        _ => {}
    }
}

/// Who got away (Luna `declare_completion`, with what this round has to say).
fn declare_completion(world: &mut World) {
    tell_everyone(world, &LocMsg::new("round-over"));
    let mut crew = world.query_filtered::<(&DisplayName, &HasJob, &GridPos, Option<&Vitals>), With<Mob>>();
    let list: Vec<(String, String, TilePos, bool)> = crew
        .iter(world)
        .map(|(n, j, p, v)| {
            let alive = v.is_none_or(|v| v.stat != MobStat::Dead);
            (n.0.clone(), j.0.clone(), p.0, alive)
        })
        .collect();
    for (name, job, at, alive) in list {
        let id = if !alive {
            "round-died"
        } else if in_pod(world, at) {
            "round-escaped"
        } else {
            "round-stranded"
        };
        tell_everyone(world, &LocMsg::new(id).arg("name", name).arg("job", job));
    }
}

/// Show each player the console they have open.
#[allow(clippy::type_complexity)]
pub fn update_comms_views(
    pods: Res<EscapePods>,
    time: Res<WorldTime>,
    mut viewers: Query<(
        Entity,
        &GridPos,
        Option<&PanelViewer>,
        &mut CommsPanelOut,
        Option<&Vitals>,
    )>,
    consoles: Query<(&GridPos, &NetId, &PowerUser), With<CommsConsole>>,
    mut commands: Commands,
) {
    let now = time.deciseconds();
    let launch_in = match pods.state {
        PodState::Launching { at, .. } => Some(((at - now) / 10.0).ceil().max(0.0) as u32),
        _ => None,
    };
    let gone = matches!(pods.state, PodState::Transit { .. } | PodState::Arrived);
    for (mob, pos, viewer, mut out, vitals) in &mut viewers {
        let target = viewer.and_then(|v| consoles.get(v.0).ok());
        let view = target.and_then(|(at, id, power)| {
            let usable =
                in_reach(pos.0, at.0) && power.powered && vitals.is_none_or(|v| v.stat == MobStat::Conscious);
            usable.then_some(CommsPanelView {
                target: *id,
                launch_in,
                gone,
            })
        });
        if view.is_none() && target.is_some() {
            commands.entity(mob).remove::<PanelViewer>();
        }
        if out.0 != view {
            out.0 = view;
        }
    }
}
