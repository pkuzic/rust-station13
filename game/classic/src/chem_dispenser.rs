//! The chem dispenser (Luna's `chem_dispenser`): pure chemicals into a beaker, on the energy it
//! stores. Spec: `docs/reference/chemistry.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, ChemPanelView, Dense, GridPos, MobStat, NetId, TilePos};

use crate::actions::{Outbox, in_reach};
use crate::chemistry::{Reagents, is_glass, react};
use crate::closets::InCloset;
use crate::hands::{Hands, HeldBy};
use crate::health::Vitals;
use crate::machines::MACHINE_TICKS;
use crate::power::{Channel, PowerUser};
use crate::time::WorldTime;
use crate::wires::PanelViewer;

pub const CHEM_DISPENSER: &str = "chem-dispenser";
/// What it can give (Luna `dispensable_reagents`).
pub const DISPENSABLE: [&str; 21] = [
    "aluminium",
    "carbon",
    "chlorine",
    "copper",
    "ethanol",
    "fluorine",
    "hydrogen",
    "iron",
    "lithium",
    "mercury",
    "nitrogen",
    "oxygen",
    "phosphorus",
    "potassium",
    "radium",
    "sacid",
    "silicon",
    "sodium",
    "sugar",
    "sulfur",
    "water",
];
/// Its store of energy (Luna `max_energy`), and how many cycles until it gains 1 (`recharged`).
pub const MAX_ENERGY: f32 = 100.0;
pub const RECHARGE_CYCLES: u32 = 16;
/// How much a unit of energy dispenses (Luna `energy * 20`, charged at a tenth a unit).
pub const UNITS_PER_ENERGY: f32 = 10.0;

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ChemDispenser {
    pub energy: f32,
    pub amount: u8,
    pub beaker: Option<Entity>,
    pub recharging: u32,
}

/// A button on a dispenser's window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChemOp {
    Dispense(usize),
    Amount(u8),
    Eject,
}

/// What a mob asked of a dispenser this tick.
#[derive(Component, Debug, Default)]
pub struct PendingChemOps(pub Vec<(Entity, ChemOp)>);

/// What a mob's open dispenser window shows.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct ChemPanelOut(pub Option<ChemPanelView>);

pub fn spawn_chem_dispenser(world: &mut World, pos: TilePos, id: NetId) -> Entity {
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(CHEM_DISPENSER, "dispenser"),
            Dense,
            ChemDispenser {
                energy: MAX_ENERGY,
                amount: 30,
                beaker: None,
                recharging: 0,
            },
            PowerUser::new(Channel::Equipment, 20.0),
        ))
        .id()
}

/// A click: glass in hand goes in; an empty hand opens the window (Luna `attackby`, `attack_hand`).
pub fn click(world: &mut World, mob: Entity, machine: Entity) {
    let held = world.get::<Hands>(mob).and_then(Hands::active_item);
    if let Some(item) = held {
        let is_beaker = world.get::<Appearance>(item).is_some_and(|a| is_glass(&a.sprite));
        if !is_beaker {
            return;
        }
        let full = world
            .get::<ChemDispenser>(machine)
            .is_some_and(|d| d.beaker.is_some());
        if full {
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
        if let Some(mut d) = world.get_mut::<ChemDispenser>(machine) {
            d.beaker = Some(item);
        }
    }
    world.entity_mut(mob).insert(PanelViewer(machine));
}

fn apply(world: &mut World, machine: Entity, op: ChemOp) {
    let Some(state) = world.get::<ChemDispenser>(machine).copied() else {
        return;
    };
    match op {
        ChemOp::Amount(amount) => {
            if let Some(mut d) = world.get_mut::<ChemDispenser>(machine) {
                d.amount = amount.min(100);
            }
        }
        ChemOp::Eject => {
            let Some(beaker) = state.beaker else { return };
            let Some(at) = world.get::<GridPos>(machine).copied() else {
                return;
            };
            world.entity_mut(beaker).remove::<InCloset>().insert(at);
            if let Some(mut d) = world.get_mut::<ChemDispenser>(machine) {
                d.beaker = None;
            }
        }
        ChemOp::Dispense(line) => {
            let Some(reagent) = DISPENSABLE.get(line) else {
                return;
            };
            let Some(beaker) = state.beaker else { return };
            if !world.get::<PowerUser>(machine).is_some_and(|p| p.powered) {
                return;
            }
            let wanted = f32::from(state.amount).min(state.energy * UNITS_PER_ENERGY);
            let Some(mut into) = world.get_mut::<Reagents>(beaker) else {
                return;
            };
            let added = into.add(reagent, wanted);
            react(&mut into);
            if let Some(mut d) = world.get_mut::<ChemDispenser>(machine) {
                d.energy = (d.energy - added / UNITS_PER_ENERGY).max(0.0);
            }
        }
    }
}

/// Buttons pressed, and energy slowly coming back (Luna `process`, `recharge`).
pub fn run_dispensers(world: &mut World) {
    let mut askers = world.query::<(&GridPos, &mut PendingChemOps)>();
    let asked: Vec<(TilePos, Vec<(Entity, ChemOp)>)> = askers
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
    if !world.resource::<WorldTime>().tick.0.is_multiple_of(MACHINE_TICKS) {
        return;
    }
    let mut machines = world.query::<(&mut ChemDispenser, &PowerUser)>();
    for (mut d, power) in machines.iter_mut(world) {
        if !power.powered || d.energy >= MAX_ENERGY {
            continue;
        }
        d.recharging += 1;
        if d.recharging >= RECHARGE_CYCLES {
            d.recharging = 0;
            d.energy = (d.energy + 1.0).min(MAX_ENERGY);
        }
    }
}

/// Show each player the dispenser they have open.
#[allow(clippy::type_complexity)]
pub fn update_chem_views(
    mut viewers: Query<(
        Entity,
        &GridPos,
        Option<&PanelViewer>,
        &mut ChemPanelOut,
        Option<&Vitals>,
    )>,
    machines: Query<(&GridPos, &ChemDispenser, &NetId)>,
    beakers: Query<&Reagents>,
    mut commands: Commands,
) {
    for (mob, pos, viewer, mut out, vitals) in &mut viewers {
        let target = viewer.and_then(|v| machines.get(v.0).ok());
        let view = target.and_then(|(at, machine, id)| {
            let usable = in_reach(pos.0, at.0) && vitals.is_none_or(|v| v.stat == MobStat::Conscious);
            usable.then(|| {
                let beaker = machine.beaker.and_then(|b| beakers.get(b).ok());
                ChemPanelView {
                    target: *id,
                    energy: machine.energy.round() as u16,
                    amount: machine.amount,
                    beaker: beaker.map(|b| (b.total().round() as u16, b.max.round() as u16)),
                    reagents: DISPENSABLE.iter().map(|r| (*r).to_owned()).collect(),
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
