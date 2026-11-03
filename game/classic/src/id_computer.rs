//! The identification computer (Luna's `computer/card`): put a card in, and with `change_ids` access
//! give it another job's access. It also shows and prints the crew manifest. Spec:
//! `docs/reference/access.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{
    Appearance, Dense, DisplayName, GridPos, IdComputerView, MobStat, NetId, ObjectSpec, Slot, TilePos,
};

use crate::access::{IdCard, allowed};
use crate::actions::{Outbox, in_reach};
use crate::closets::InCloset;
use crate::equipment::Equipment;
use crate::hands::{Hands, HeldBy};
use crate::health::Vitals;
use crate::jobs::{CardOwner, HasJob, jobs};
use crate::objects::spawn_object;
use crate::paperwork::PAPER;
use crate::power::{Channel, PowerUser};
use crate::time::WorldTime;
use crate::wires::PanelViewer;

pub const ID_COMPUTER: &str = "id-computer";
/// The access that lets someone change a card (Luna `access_change_ids`).
pub const CHANGE_IDS: &str = "change_ids";

/// How long a printout takes, ds (Luna's `sleep(50)`).
pub const PRINT_DS: f32 = 50.0;

/// The computer, the card in it, and whether it's on the crew manifest.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct IdComputer {
    pub card: Option<Entity>,
    pub manifest: bool,
    /// When a printout comes out, ds.
    pub printing_at: Option<f32>,
}

/// A button on its window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdOp {
    /// Give the card this line of the job list.
    Assign(usize),
    Eject,
    /// Switch to the crew manifest, or back.
    Manifest,
    Print,
}

/// What a mob asked of an identification computer this tick.
#[derive(Component, Debug, Default)]
pub struct PendingIdOps(pub Vec<(Entity, IdOp)>);

/// What a mob's open identification computer window shows.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct IdPanelOut(pub Option<IdComputerView>);

pub fn spawn_id_computer(world: &mut World, pos: TilePos, id: NetId) -> Entity {
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(ID_COMPUTER, "id"),
            Dense,
            IdComputer::default(),
            PowerUser::new(Channel::Equipment, 100.0),
        ))
        .id()
}

/// The user's own card has to allow it: the one they wear, since the one in hand goes in the slot.
fn permitted(world: &World, mob: Entity) -> bool {
    let worn = world.get::<Equipment>(mob).and_then(|e| e.get(Slot::Id));
    allowed(
        &[CHANGE_IDS.to_owned()],
        worn.and_then(|item| world.get::<IdCard>(item)),
    )
}

/// A click: a card in hand goes in; anything else just opens the window, with power.
pub fn click(world: &mut World, mob: Entity, machine: Entity) {
    if !world.get::<PowerUser>(machine).is_some_and(|p| p.powered) {
        return;
    }
    let held = world.get::<Hands>(mob).and_then(Hands::active_item);
    if let Some(item) = held.filter(|i| world.get::<IdCard>(*i).is_some()) {
        let empty = world.get::<IdComputer>(machine).is_some_and(|c| c.card.is_none());
        if empty {
            if let Some(mut hands) = world.get_mut::<Hands>(mob) {
                let active = hands.active;
                *hands.slot_mut(active) = None;
            }
            world
                .entity_mut(item)
                .remove::<HeldBy>()
                .insert(InCloset(machine));
            if let Some(mut computer) = world.get_mut::<IdComputer>(machine) {
                computer.card = Some(item);
            }
        }
    }
    world.entity_mut(mob).insert(PanelViewer(machine));
}

fn apply(world: &mut World, mob: Entity, machine: Entity, op: IdOp) {
    let now = world.resource::<WorldTime>().deciseconds();
    let Some(mut computer) = world.get_mut::<IdComputer>(machine) else {
        return;
    };
    match op {
        IdOp::Manifest => {
            computer.manifest = !computer.manifest;
            return;
        }
        IdOp::Print => {
            computer.printing_at.get_or_insert(now + PRINT_DS);
            return;
        }
        _ => {}
    }
    let Some(card) = computer.card else {
        return;
    };
    match op {
        IdOp::Eject => {
            let Some(at) = world.get::<GridPos>(machine).copied() else {
                return;
            };
            world.entity_mut(card).remove::<InCloset>().insert(at);
            if let Some(mut computer) = world.get_mut::<IdComputer>(machine) {
                computer.card = None;
            }
        }
        IdOp::Assign(line) => {
            if !permitted(world, mob) {
                if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
                    outbox.0.push(LocMsg::new("records-denied"));
                }
                return;
            }
            let Some(job) = jobs().get(line) else {
                return;
            };
            let owner = world.get::<CardOwner>(card).and_then(|c| c.owner);
            world.entity_mut(card).insert((
                IdCard {
                    access: job.access.clone(),
                },
                CardOwner {
                    owner,
                    job: job.id.clone(),
                },
            ));
        }
        IdOp::Manifest | IdOp::Print => {}
    }
}

/// Everyone with a job, in the order they arrived (Luna's `data_core.general`).
fn crew(people: impl Iterator<Item = (Entity, String, String)>) -> Vec<(String, String)> {
    let mut all: Vec<_> = people.collect();
    all.sort_by_key(|(e, ..)| *e);
    all.into_iter().map(|(_, name, job)| (name, job)).collect()
}

/// A sheet with the crew manifest on it. The job names are in the reader's language, so the sheet
/// says which ones it means, and the client fills them in (`paperwork.md`).
fn print_manifest(world: &mut World, at: TilePos) {
    let mut people = world.query::<(Entity, &DisplayName, &HasJob)>();
    let list = crew(people.iter(world).map(|(e, n, j)| (e, n.0.clone(), j.0.clone())));
    let mut text = String::from("[msg:id-manifest-title]\n");
    for (name, job) in list {
        text.push_str(&format!("[b]{name}[/b] - [job:{job}]\n"));
    }
    let mut sheet = ObjectSpec::new(PAPER);
    sheet
        .vars
        .insert("name".to_owned(), "[msg:id-manifest-paper]".to_owned());
    sheet.vars.insert("info".to_owned(), text);
    // Paper is always a known prototype.
    let _ = spawn_object(world, &sheet, at);
}

/// Printouts that are ready.
fn finish_printing(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    let mut machines = world.query::<(Entity, &GridPos, &IdComputer)>();
    let done: Vec<(Entity, TilePos)> = machines
        .iter(world)
        .filter(|(.., c)| c.printing_at.is_some_and(|at| now >= at))
        .map(|(e, p, _)| (e, p.0))
        .collect();
    for (machine, at) in done {
        if let Some(mut computer) = world.get_mut::<IdComputer>(machine) {
            computer.printing_at = None;
        }
        print_manifest(world, at);
    }
}

/// Buttons pressed.
pub fn run_id_computers(world: &mut World) {
    finish_printing(world);
    let mut askers = world.query::<(Entity, &GridPos, &mut PendingIdOps)>();
    let asked: Vec<(Entity, TilePos, Vec<(Entity, IdOp)>)> = askers
        .iter_mut(world)
        .filter(|(.., ops)| !ops.0.is_empty())
        .map(|(e, at, mut ops)| (e, at.0, std::mem::take(&mut ops.0)))
        .collect();
    for (mob, at, ops) in asked {
        for (machine, op) in ops {
            if world.get::<GridPos>(machine).is_some_and(|p| in_reach(at, p.0)) {
                apply(world, mob, machine, op);
            }
        }
    }
}

/// Who a card says it belongs to, and its job, when someone looks at it (Luna `card/id/examine`).
pub fn examine(world: &World, target: Entity) -> Option<LocMsg> {
    let card = world.get::<CardOwner>(target)?;
    let name = card
        .owner
        .and_then(|o| world.get::<DisplayName>(o))
        .map_or_else(String::new, |n| n.0.clone());
    Some(
        LocMsg::new("examine-card")
            .arg("name", name)
            .arg("job", card.job.clone()),
    )
}

/// Show each player the identification computer they have open.
#[allow(clippy::type_complexity)]
pub fn update_id_views(
    mut viewers: Query<(
        Entity,
        &GridPos,
        Option<&PanelViewer>,
        &mut IdPanelOut,
        Option<&Vitals>,
    )>,
    machines: Query<(&GridPos, &IdComputer, &NetId, &PowerUser)>,
    cards: Query<Option<&CardOwner>, With<IdCard>>,
    names: Query<&DisplayName>,
    people: Query<(Entity, &DisplayName, &HasJob)>,
    mut commands: Commands,
) {
    for (mob, pos, viewer, mut out, vitals) in &mut viewers {
        let target = viewer.and_then(|v| machines.get(v.0).ok());
        let view = target.and_then(|(at, computer, id, power)| {
            let usable =
                in_reach(pos.0, at.0) && power.powered && vitals.is_none_or(|v| v.stat == MobStat::Conscious);
            usable.then(|| {
                let inside = computer.card.and_then(|c| cards.get(c).ok());
                let owner = inside.flatten();
                IdComputerView {
                    target: *id,
                    has_card: inside.is_some(),
                    owner: owner
                        .and_then(|c| c.owner)
                        .and_then(|o| names.get(o).ok())
                        .map(|n| n.0.clone()),
                    job: owner.map(|c| c.job.clone()),
                    jobs: jobs().iter().map(|j| j.id.clone()).collect(),
                    manifest: computer
                        .manifest
                        .then(|| crew(people.iter().map(|(e, n, j)| (e, n.0.clone(), j.0.clone())))),
                    printing: computer.printing_at.is_some(),
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
