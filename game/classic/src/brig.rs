//! Brig door timers (Luna's `door_timer`): set a sentence on the wall, and the cell's window
//! doors shut until it runs out. The same panel fires the cell's flasher. Spec:
//! `docs/reference/brig.md`.

use std::collections::BTreeSet;

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, BrigPanelView, GridPos, MobStat, NetId, ObjectSpec, Slot, TilePos};

use crate::access::{IdCard, allowed};
use crate::actions::{Outbox, in_reach};
use crate::equipment::Equipment;
use crate::flashes::{FLASHER_COOLDOWN_DS, Flasher, fire_flasher};
use crate::hands::Hands;
use crate::health::Vitals;
use crate::machines::MACHINE_TICKS;
use crate::power::{Channel, PowerUser};
use crate::time::WorldTime;
use crate::windoors::{SWING_DS, Swing, Windoor};
use crate::wires::PanelViewer;

pub const DOOR_TIMER: &str = "door-timer";
/// The access a timer wants (Luna `access_brig`).
pub const TIMER_ACCESS: &str = "brig";
/// The longest sentence, seconds (Luna's cap).
pub const MAX_SECONDS: i32 = 3000;

/// A cell's door, by the id it shares with its timer (Luna `brigdoor/id`).
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct CellDoor(pub String);

/// A timer on the wall.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct DoorTimer {
    pub id: String,
    /// While it runs: when the prisoner goes free, ds.
    pub release_at: Option<f32>,
    /// While it doesn't: the sentence set, seconds.
    pub length: i32,
}

impl DoorTimer {
    /// Luna `timeleft`, seconds.
    pub fn seconds_left(&self, now: f32) -> i32 {
        match self.release_at {
            Some(at) => (((at - now) / 10.0).ceil() as i32).max(0),
            None => self.length,
        }
    }
}

/// A button on a timer's window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrigOp {
    /// Start it and shut the door, or stop it and open the door.
    Toggle,
    /// Add these seconds, or take them off.
    Adjust(i32),
    Flash,
}

/// What a mob asked of a timer this tick.
#[derive(Component, Debug, Default)]
pub struct PendingBrigOps(pub Vec<(Entity, BrigOp)>);

/// What a mob's open timer window shows.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct BrigPanelOut(pub Option<BrigPanelView>);

pub fn spawn_door_timer(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    let timer = DoorTimer {
        id: spec
            .vars
            .get("id")
            .map(|v| v.trim().trim_matches('"').to_owned())
            .unwrap_or_default(),
        release_at: None,
        length: 0,
    };
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(DOOR_TIMER, "frame"),
            timer,
            PowerUser::new(Channel::Equipment, 10.0),
        ))
        .id()
}

/// The card this mob shows: in the active hand, or worn.
fn card_of(world: &World, mob: Entity) -> Option<IdCard> {
    let held = world.get::<Hands>(mob).and_then(Hands::active_item);
    let worn = world.get::<Equipment>(mob).and_then(|e| e.get(Slot::Id));
    [held, worn]
        .into_iter()
        .flatten()
        .find_map(|item| world.get::<IdCard>(item).cloned())
}

fn is_allowed(world: &World, mob: Entity) -> bool {
    allowed(&[TIMER_ACCESS.to_owned()], card_of(world, mob).as_ref())
}

/// A click on a timer (Luna `attack_hand`): its window opens. Using it wants access.
pub fn click(world: &mut World, mob: Entity, timer: Entity) {
    world.entity_mut(mob).insert(PanelViewer(timer));
}

fn set_doors(world: &mut World, id: &str, shut: bool, now: f32) {
    let mut doors = world.query::<(&CellDoor, &GridPos, &mut Windoor)>();
    let mut moved: Vec<TilePos> = Vec::new();
    for (tag, at, mut door) in doors.iter_mut(world) {
        if tag.0 != id {
            continue;
        }
        match door.swing {
            Swing::Open { .. } if shut => {
                door.swing = Swing::Closing { at: now + SWING_DS };
                moved.push(at.0);
            }
            Swing::Closed if !shut => {
                door.swing = Swing::Opening {
                    at: now + SWING_DS,
                    auto: false,
                };
                moved.push(at.0);
            }
            _ => {}
        }
    }
    let mut sounds = world.resource_mut::<crate::sound::SoundEvents>();
    for at in moved {
        sounds.play(crate::sound::WINDOOR, at, 100.0);
    }
}

fn flashers_of(world: &mut World, id: &str) -> Vec<Entity> {
    let mut q = world.query::<(Entity, &Flasher)>();
    q.iter(world)
        .filter(|(_, f)| f.id == id)
        .map(|(e, _)| e)
        .collect()
}

fn apply(world: &mut World, mob: Entity, machine: Entity, op: BrigOp, now: f32) {
    let Some(timer) = world.get::<DoorTimer>(machine) else {
        return;
    };
    let id = timer.id.clone();
    if !is_allowed(world, mob) {
        if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
            outbox.0.push(LocMsg::new("brig-denied"));
        }
        return;
    }
    let Some(mut timer) = world.get_mut::<DoorTimer>(machine) else {
        return;
    };
    let left = timer.seconds_left(now);
    match op {
        BrigOp::Toggle => {
            if timer.release_at.take().is_none() {
                timer.release_at = Some(now + left as f32 * 10.0);
            } else {
                timer.length = left;
            }
        }
        BrigOp::Adjust(by) => {
            let seconds = (left + by).clamp(0, MAX_SECONDS);
            if timer.release_at.is_some() {
                timer.release_at = Some(now + seconds as f32 * 10.0);
            } else {
                timer.length = seconds;
            }
        }
        BrigOp::Flash => {
            for flasher in flashers_of(world, &id) {
                fire_flasher(world, flasher);
            }
        }
    }
}

/// Buttons pressed, sentences served, and doors kept as their timer says (Luna
/// `door_timer/process`: a timer that isn't running keeps its door open).
pub fn run_brig(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    let mut askers = world.query::<(Entity, &GridPos, &mut PendingBrigOps)>();
    let asked: Vec<(Entity, TilePos, Vec<(Entity, BrigOp)>)> = askers
        .iter_mut(world)
        .filter(|(.., ops)| !ops.0.is_empty())
        .map(|(mob, at, mut ops)| (mob, at.0, std::mem::take(&mut ops.0)))
        .collect();
    for (mob, at, ops) in asked {
        for (machine, op) in ops {
            if world.get::<GridPos>(machine).is_some_and(|p| in_reach(at, p.0)) {
                apply(world, mob, machine, op, now);
            }
        }
    }
    if !world.resource::<WorldTime>().tick.0.is_multiple_of(MACHINE_TICKS) {
        return;
    }
    let mut timers = world.query::<(&mut DoorTimer, &PowerUser)>();
    let mut held: BTreeSet<String> = BTreeSet::new();
    let mut free: BTreeSet<String> = BTreeSet::new();
    for (mut timer, power) in timers.iter_mut(world) {
        if !power.powered {
            continue;
        }
        if timer.release_at.is_some_and(|at| now > at) {
            // Luna `timer_end`, and the sentence is cleared.
            timer.release_at = None;
            timer.length = 0;
        }
        if timer.release_at.is_some() {
            held.insert(timer.id.clone());
        } else {
            free.insert(timer.id.clone());
        }
    }
    for id in held {
        set_doors(world, &id, true, now);
    }
    for id in free {
        set_doors(world, &id, false, now);
    }
}

/// Show each player the timer they have open.
#[allow(clippy::type_complexity)]
pub fn update_brig_views(
    time: Res<WorldTime>,
    mut viewers: Query<(
        Entity,
        &GridPos,
        Option<&PanelViewer>,
        &mut BrigPanelOut,
        Option<&Vitals>,
    )>,
    timers: Query<(&GridPos, &DoorTimer, &NetId, &PowerUser)>,
    flashers: Query<&Flasher>,
    mut commands: Commands,
) {
    let now = time.deciseconds();
    for (mob, pos, viewer, mut out, vitals) in &mut viewers {
        let target = viewer.and_then(|v| timers.get(v.0).ok());
        let view = target.and_then(|(at, timer, id, power)| {
            let usable =
                power.powered && in_reach(pos.0, at.0) && vitals.is_none_or(|v| v.stat == MobStat::Conscious);
            usable.then(|| {
                let mine: Vec<&Flasher> = flashers.iter().filter(|f| f.id == timer.id).collect();
                BrigPanelView {
                    target: *id,
                    timing: timer.release_at.is_some(),
                    seconds: timer.seconds_left(now),
                    has_flash: !mine.is_empty(),
                    flash_charging: mine
                        .iter()
                        .any(|f| f.last_flash_ds.is_some_and(|t| now < t + FLASHER_COOLDOWN_DS)),
                }
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
