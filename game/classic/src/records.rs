//! Crew records (Luna's `secure_data` and `med_data` computers): who's aboard and what they do, with
//! their criminal status or their condition. Spec: `docs/reference/records.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{
    Appearance, Dense, DisplayName, GridPos, MobStat, NetId, RecordRow, RecordsPanelView, Slot, TilePos,
};

use crate::access::{IdCard, allowed};
use crate::actions::{Outbox, in_reach};
use crate::equipment::Equipment;
use crate::hands::Hands;
use crate::health::Vitals;
use crate::jobs::HasJob;
use crate::power::{Channel, PowerUser};
use crate::wires::PanelViewer;

pub const SECURITY_RECORDS: &str = "security-records";
pub const DETECTIVE_RECORDS: &str = "detective-records";
pub const MEDICAL_RECORDS: &str = "medical-records";
/// Criminal statuses, in the order a button goes through them (Luna `fields["criminal"]`).
pub const STATUSES: [&str; 5] = ["none", "arrest", "incarcerated", "parolled", "released"];

/// A records computer, and whether it's the medical kind.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordsConsole {
    pub medical: bool,
}

/// Someone's line in the security records: which of `STATUSES`.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Criminal(pub usize);

/// Lines a mob pressed this tick: the console and the line of its list.
#[derive(Component, Debug, Default)]
pub struct PendingRecordOps(pub Vec<(Entity, usize)>);

/// What a mob's open records window shows.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct RecordsPanelOut(pub Option<RecordsPanelView>);

pub fn is_records(proto: &str) -> bool {
    matches!(proto, SECURITY_RECORDS | DETECTIVE_RECORDS | MEDICAL_RECORDS)
}

pub fn spawn_records(world: &mut World, proto: &str, pos: TilePos, id: NetId) -> Entity {
    let look = match proto {
        SECURITY_RECORDS => "security",
        DETECTIVE_RECORDS => "messyfiles",
        _ => "medical",
    };
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(proto, look),
            Dense,
            RecordsConsole {
                medical: proto == MEDICAL_RECORDS,
            },
            PowerUser::new(Channel::Equipment, 100.0),
        ))
        .id()
}

/// The access a console wants (Luna `req_access`).
fn wanted(console: RecordsConsole) -> String {
    if console.medical { "medical" } else { "security" }.to_owned()
}

fn permitted(world: &World, mob: Entity, console: RecordsConsole) -> bool {
    let held = world.get::<Hands>(mob).and_then(Hands::active_item);
    let worn = world.get::<Equipment>(mob).and_then(|e| e.get(Slot::Id));
    let card = [held, worn]
        .into_iter()
        .flatten()
        .find_map(|item| world.get::<IdCard>(item));
    allowed(&[wanted(console)], card)
}

/// A click opens it, with power and the right card (Luna `attack_hand`, its login).
pub fn click(world: &mut World, mob: Entity, machine: Entity) {
    let Some(console) = world.get::<RecordsConsole>(machine).copied() else {
        return;
    };
    if !world.get::<PowerUser>(machine).is_some_and(|p| p.powered) {
        return;
    }
    if !permitted(world, mob, console) {
        if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
            outbox.0.push(LocMsg::new("records-denied"));
        }
        return;
    }
    world.entity_mut(mob).insert(PanelViewer(machine));
}

/// Everyone with a job, in a steady order: their entity, for the list and its buttons.
fn crew(world: &mut World) -> Vec<Entity> {
    let mut people = world.query_filtered::<Entity, With<HasJob>>();
    let mut all: Vec<Entity> = people.iter(world).collect();
    all.sort();
    all
}

/// A press on a security console's line moves that person on to the next status.
pub fn run_records(world: &mut World) {
    let mut askers = world.query::<(Entity, &GridPos, &mut PendingRecordOps)>();
    let asked: Vec<(Entity, TilePos, Vec<(Entity, usize)>)> = askers
        .iter_mut(world)
        .filter(|(.., ops)| !ops.0.is_empty())
        .map(|(e, at, mut ops)| (e, at.0, std::mem::take(&mut ops.0)))
        .collect();
    for (mob, at, ops) in asked {
        for (machine, line) in ops {
            let near = world.get::<GridPos>(machine).is_some_and(|p| in_reach(at, p.0));
            let Some(console) = world.get::<RecordsConsole>(machine).copied() else {
                continue;
            };
            if !near || console.medical || !permitted(world, mob, console) {
                continue;
            }
            let Some(person) = crew(world).get(line).copied() else {
                continue;
            };
            let next = world
                .get::<Criminal>(person)
                .map_or(1, |c| (c.0 + 1) % STATUSES.len());
            world.entity_mut(person).insert(Criminal(next));
        }
    }
}

/// Show each player the records they have open.
#[allow(clippy::type_complexity)]
pub fn update_records_views(
    mut viewers: Query<(
        Entity,
        &GridPos,
        Option<&PanelViewer>,
        &mut RecordsPanelOut,
        Option<&Vitals>,
    )>,
    machines: Query<(&GridPos, &RecordsConsole, &NetId, &PowerUser)>,
    people: Query<(Entity, &HasJob, &DisplayName, Option<&Vitals>, Option<&Criminal>)>,
    mut commands: Commands,
) {
    for (mob, pos, viewer, mut out, vitals) in &mut viewers {
        let target = viewer.and_then(|v| machines.get(v.0).ok());
        let view = target.and_then(|(at, console, id, power)| {
            let usable =
                in_reach(pos.0, at.0) && power.powered && vitals.is_none_or(|v| v.stat == MobStat::Conscious);
            usable.then(|| {
                let mut all: Vec<_> = people.iter().collect();
                all.sort_by_key(|(e, ..)| *e);
                RecordsPanelView {
                    target: *id,
                    medical: console.medical,
                    rows: all
                        .into_iter()
                        .map(|(_, job, name, vitals, criminal)| RecordRow {
                            name: name.0.clone(),
                            job: job.0.clone(),
                            status: if console.medical {
                                match vitals.map_or(MobStat::Conscious, |v| v.stat) {
                                    MobStat::Conscious => "active",
                                    MobStat::Unconscious => "unconscious",
                                    _ => "deceased",
                                }
                            } else {
                                STATUSES[criminal.map_or(0, |c| c.0)]
                            }
                            .to_owned(),
                        })
                        .collect(),
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
