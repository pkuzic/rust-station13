//! The CheMaster (Luna's `chem_master`): takes a beaker, moves what's in it through a buffer, and
//! makes pills and bottles out of the buffer. Spec: `docs/reference/chemistry.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, ChemMasterPanelView, Dense, GridPos, MobStat, NetId, ObjectSpec, TilePos};

use crate::actions::{Outbox, in_reach};
use crate::chemistry::{Reagents, is_glass, is_syringe, react};
use crate::closets::InCloset;
use crate::hands::{Hands, HeldBy};
use crate::health::Vitals;
use crate::objects::spawn_object;
use crate::power::{Channel, PowerUser};
use crate::wires::PanelViewer;

pub const CHEM_MASTER: &str = "chem-master";
/// Its buffer's size (Luna's `new/datum/reagents(100)`).
pub const BUFFER: f32 = 100.0;
/// What one pill and one bottle take from the buffer (Luna `trans_to(P,50)`, `trans_to(P,30)`).
pub const PILL_UNITS: f32 = 50.0;
pub const BOTTLE_UNITS: f32 = 30.0;
/// What it makes.
pub const MADE_PILL: &str = "pill-made";
pub const MADE_BOTTLE: &str = "bottle-made";
/// How much of a reagent one button moves.
pub const MOVE_UNITS: f32 = 10.0;

#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChemMaster {
    pub beaker: Option<Entity>,
}

/// A button on the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MasterOp {
    /// This line of the beaker's list, into the buffer.
    Add(usize),
    /// This line of the buffer's list, back into the beaker.
    Remove(usize),
    Pill,
    Bottle,
    Eject,
}

/// What a mob asked of a CheMaster this tick.
#[derive(Component, Debug, Default)]
pub struct PendingMasterOps(pub Vec<(Entity, MasterOp)>);

/// What a mob's open CheMaster window shows.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct MasterPanelOut(pub Option<ChemMasterPanelView>);

pub fn spawn_chem_master(world: &mut World, pos: TilePos, id: NetId) -> Entity {
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(CHEM_MASTER, "mixer0"),
            Dense,
            ChemMaster::default(),
            Reagents::new(BUFFER),
            PowerUser::new(Channel::Equipment, 20.0),
        ))
        .id()
}

fn set_look(world: &mut World, machine: Entity, state: &str) {
    if let Some(mut look) = world.get_mut::<Appearance>(machine) {
        look.state = state.to_owned();
    }
}

/// A click: a beaker or syringe in hand goes in; an empty hand opens the window (Luna `attackby`,
/// `attack_hand`).
pub fn click(world: &mut World, mob: Entity, machine: Entity) {
    let held = world.get::<Hands>(mob).and_then(Hands::active_item);
    if let Some(item) = held {
        let sprite = world
            .get::<Appearance>(item)
            .map(|a| a.sprite.clone())
            .unwrap_or_default();
        if !is_glass(&sprite) && !is_syringe(&sprite) {
            return;
        }
        if world
            .get::<ChemMaster>(machine)
            .is_some_and(|m| m.beaker.is_some())
        {
            if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
                outbox.0.push(LocMsg::new("dispenser-occupied"));
            }
            return;
        }
        if let Some(mut hands) = world.get_mut::<Hands>(mob) {
            let active = hands.active;
            *hands.slot_mut(active) = None;
        }
        world
            .entity_mut(item)
            .remove::<HeldBy>()
            .insert(InCloset(machine));
        if let Some(mut m) = world.get_mut::<ChemMaster>(machine) {
            m.beaker = Some(item);
        }
        set_look(
            world,
            machine,
            if is_syringe(&sprite) { "mixers" } else { "mixer1" },
        );
    }
    world.entity_mut(mob).insert(PanelViewer(machine));
}

/// Move up to `amount` of the reagent on `line` of `from`'s list into `to`.
fn move_line(world: &mut World, from: Entity, to: Entity, line: usize, amount: f32) {
    let Some((reagent, there)) = world
        .get::<Reagents>(from)
        .and_then(|r| r.amounts.iter().nth(line).map(|(id, n)| (id.clone(), *n)))
    else {
        return;
    };
    let Some(mut into) = world.get_mut::<Reagents>(to) else {
        return;
    };
    let moved = into.add(&reagent, amount.min(there));
    react(&mut into);
    if let Some(mut out) = world.get_mut::<Reagents>(from) {
        out.remove(&reagent, moved);
    }
}

/// Make `proto` under the machine out of up to `units` of the buffer (Luna `createpill`,
/// `createbottle`). An empty buffer makes nothing, where Luna makes an empty pill.
fn make(world: &mut World, machine: Entity, proto: &str, units: f32) {
    let Some(at) = world.get::<GridPos>(machine).map(|p| p.0) else {
        return;
    };
    let empty = world.get::<Reagents>(machine).is_none_or(|r| r.total() <= 0.0);
    if empty {
        return;
    }
    let Ok(made) = spawn_object(world, &ObjectSpec::new(proto), at) else {
        return;
    };
    let taken = world
        .get_mut::<Reagents>(machine)
        .map(|mut buffer| buffer.take(units))
        .unwrap_or_default();
    if let Some(mut into) = world.get_mut::<Reagents>(made) {
        for (reagent, amount) in taken {
            into.add(&reagent, amount);
        }
        react(&mut into);
    }
}

fn apply(world: &mut World, machine: Entity, op: MasterOp) {
    let Some(state) = world.get::<ChemMaster>(machine).copied() else {
        return;
    };
    if !world.get::<PowerUser>(machine).is_some_and(|p| p.powered) {
        return;
    }
    let Some(beaker) = state.beaker else { return };
    match op {
        MasterOp::Add(line) => move_line(world, beaker, machine, line, MOVE_UNITS),
        MasterOp::Remove(line) => move_line(world, machine, beaker, line, MOVE_UNITS),
        MasterOp::Pill => make(world, machine, MADE_PILL, PILL_UNITS),
        MasterOp::Bottle => make(world, machine, MADE_BOTTLE, BOTTLE_UNITS),
        MasterOp::Eject => {
            let Some(at) = world.get::<GridPos>(machine).copied() else {
                return;
            };
            world.entity_mut(beaker).remove::<InCloset>().insert(at);
            if let Some(mut m) = world.get_mut::<ChemMaster>(machine) {
                m.beaker = None;
            }
            // The buffer goes with it.
            if let Some(mut buffer) = world.get_mut::<Reagents>(machine) {
                *buffer = Reagents::new(BUFFER);
            }
            set_look(world, machine, "mixer0");
        }
    }
}

/// Buttons pressed.
pub fn run_chem_masters(world: &mut World) {
    let mut askers = world.query::<(&GridPos, &mut PendingMasterOps)>();
    let asked: Vec<(TilePos, Vec<(Entity, MasterOp)>)> = askers
        .iter_mut(world)
        .filter(|(_, ops)| !ops.0.is_empty())
        .map(|(at, mut ops)| (at.0, std::mem::take(&mut ops.0)))
        .collect();
    for (at, ops) in asked {
        for (machine, op) in ops {
            if world.get::<GridPos>(machine).is_some_and(|p| in_reach(at, p.0)) {
                apply(world, machine, op);
            }
        }
    }
}

fn shown(reagents: &Reagents) -> Vec<(String, u16)> {
    reagents
        .amounts
        .iter()
        .map(|(id, n)| (id.clone(), n.round() as u16))
        .collect()
}

/// Show each player the CheMaster they have open.
#[allow(clippy::type_complexity)]
pub fn update_master_views(
    mut viewers: Query<(
        Entity,
        &GridPos,
        Option<&PanelViewer>,
        &mut MasterPanelOut,
        Option<&Vitals>,
    )>,
    machines: Query<(&GridPos, &ChemMaster, &NetId, &Reagents)>,
    beakers: Query<&Reagents, Without<ChemMaster>>,
    mut commands: Commands,
) {
    for (mob, pos, viewer, mut out, vitals) in &mut viewers {
        let target = viewer.and_then(|v| machines.get(v.0).ok());
        let view = target.and_then(|(at, machine, id, buffer)| {
            let usable = in_reach(pos.0, at.0) && vitals.is_none_or(|v| v.stat == MobStat::Conscious);
            usable.then(|| ChemMasterPanelView {
                target: *id,
                beaker: machine.beaker.and_then(|b| beakers.get(b).ok()).map(shown),
                buffer: shown(buffer),
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
