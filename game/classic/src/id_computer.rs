//! The identification computer (Luna's `computer/card`): put a card in, and with `change_ids` access
//! give it another job's access. Spec: `docs/reference/access.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, Dense, DisplayName, GridPos, IdComputerView, MobStat, NetId, Slot, TilePos};

use crate::access::{IdCard, allowed};
use crate::actions::{Outbox, in_reach};
use crate::closets::InCloset;
use crate::equipment::Equipment;
use crate::hands::{Hands, HeldBy};
use crate::health::Vitals;
use crate::jobs::{CardOwner, jobs};
use crate::power::{Channel, PowerUser};
use crate::wires::PanelViewer;

pub const ID_COMPUTER: &str = "id-computer";
/// The access that lets someone change a card (Luna `access_change_ids`).
pub const CHANGE_IDS: &str = "change_ids";

/// The computer, and the card in it.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IdComputer {
    pub card: Option<Entity>,
}

/// A button on its window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdOp {
    /// Give the card this line of the job list.
    Assign(usize),
    Eject,
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
    let Some(card) = world.get::<IdComputer>(machine).and_then(|c| c.card) else {
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
    }
}

/// Buttons pressed.
pub fn run_id_computers(world: &mut World) {
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
