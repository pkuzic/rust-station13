//! Freezers: chill the gas in their pipe network, for the engine's cold loop. Spec:
//! `docs/reference/engine.md` (freezer).

use bevy_ecs::prelude::*;
use rs13_simulation::atmos::gas::{T0C, T20C};
use rs13_world::{Appearance, FreezerPanelView, GridPos, MobStat, NetId, TilePos};

use crate::actions::in_reach;
use crate::health::Vitals;
use crate::machines::{DEVICE_VOLUME, MACHINE_TICKS, PipeNets};
use crate::time::WorldTime;
use crate::wires::PanelViewer;

pub const FREEZER: &str = "freezer";
/// Heat capacity of the cold it gives the gas, J/K (Luna `current_heat_capacity = 1000`).
pub const FREEZER_HEAT_CAPACITY: f64 = 1000.0;
/// Coldest and warmest it can be set, K (Luna `T0C - 200` and `T20C`).
pub const COLDEST: f64 = T0C - 200.0;
pub const WARMEST: f64 = T20C;

/// A freezer (Luna `cold_sink/freezer`). It starts off and set to 20 °C, as in Luna.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Freezer {
    pub on: bool,
    /// The temperature it cools to, K (Luna `current_temperature`).
    pub setting: f64,
}

impl Default for Freezer {
    fn default() -> Self {
        Self {
            on: false,
            setting: WARMEST,
        }
    }
}

/// Freezer buttons waiting for the next tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FreezerOp {
    Toggle,
    Setting(f64),
}

#[derive(Component, Debug, Default)]
pub struct PendingFreezerOps(pub Vec<(Entity, FreezerOp)>);

/// What the mob's open freezer shows, for the server to send when it changes.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct FreezerPanelOut(pub Option<FreezerPanelView>);

/// Every machine cycle, a running freezer evens out its own 200 L of the network's gas
/// with 1000 J/K of cold at its setting (Luna `cold_sink/process`). The cold never runs out.
pub fn run_freezers(time: Res<WorldTime>, mut nets: ResMut<PipeNets>, freezers: Query<(Entity, &Freezer)>) {
    if !time.tick.0.is_multiple_of(MACHINE_TICKS) {
        return;
    }
    for (entity, freezer) in &freezers {
        if !freezer.on {
            continue;
        }
        let Some(&id) = nets.net_of.get(&entity) else {
            continue;
        };
        let net = &mut nets.nets[id];
        if net.volume <= 0.0 {
            continue;
        }
        let share = net.mix.scaled(DEVICE_VOLUME.min(net.volume) / net.volume);
        let capacity = share.heat_capacity();
        if capacity <= 0.0 {
            continue;
        }
        let combined = (share.temperature * capacity + freezer.setting * FREEZER_HEAT_CAPACITY)
            / (capacity + FREEZER_HEAT_CAPACITY);
        net.mix.add_energy((combined - share.temperature) * capacity);
    }
}

/// Clicking a freezer in reach opens its window. `None` when `target` is no freezer.
pub fn open(
    freezers: &Query<&GridPos, With<Freezer>>,
    mob: Entity,
    mob_pos: TilePos,
    target: Entity,
    commands: &mut Commands,
) -> Option<()> {
    let pos = freezers.get(target).ok()?;
    if in_reach(mob_pos, pos.0) {
        commands.entity(mob).insert(PanelViewer(target));
    }
    Some(())
}

/// Apply freezer buttons (Luna `Topic`): in reach, the setting kept within −200 °C and 20 °C.
pub fn process_freezer_ops(
    mut mobs: Query<(&GridPos, &mut PendingFreezerOps, Option<&Vitals>)>,
    mut freezers: Query<(&GridPos, &mut Freezer)>,
) {
    for (pos, mut pending, vitals) in &mut mobs {
        let awake = vitals.is_none_or(|v| v.stat == MobStat::Conscious);
        for (target, op) in pending.0.drain(..) {
            let Ok((at, mut freezer)) = freezers.get_mut(target) else {
                continue;
            };
            if !awake || !in_reach(pos.0, at.0) {
                continue;
            }
            match op {
                FreezerOp::Toggle => freezer.on = !freezer.on,
                FreezerOp::Setting(delta) => {
                    freezer.setting = (freezer.setting + delta).clamp(COLDEST, WARMEST)
                }
            }
        }
    }
}

/// Keep each viewer's freezer window in step; close it out of reach.
pub fn update_freezer_views(
    nets: Res<PipeNets>,
    mut viewers: Query<(
        Entity,
        &GridPos,
        Option<&PanelViewer>,
        &mut FreezerPanelOut,
        Option<&Vitals>,
    )>,
    freezers: Query<(Entity, &GridPos, &Freezer, &NetId)>,
    mut commands: Commands,
) {
    for (mob, pos, viewer, mut out, vitals) in &mut viewers {
        let target = viewer.and_then(|v| freezers.get(v.0).ok());
        let view = target.and_then(|(entity, at, freezer, id)| {
            let usable = in_reach(pos.0, at.0) && vitals.is_none_or(|v| v.stat == MobStat::Conscious);
            if !usable {
                return None;
            }
            let net = nets.net_of.get(&entity).map(|i| &nets.nets[*i]);
            Some(FreezerPanelView {
                target: *id,
                on: freezer.on,
                gas_dk: net.map_or(0, |n| (n.mix.temperature * 10.0).round().max(0.0) as u32),
                pressure_dkpa: net.map_or(0, |n| (n.pressure() * 10.0).round().max(0.0) as u32),
                setting: freezer.setting.round() as u32,
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

/// Luna `update_icon`: `freezer_1` running, `freezer` off.
pub fn update_freezer_looks(mut freezers: Query<(&Freezer, &mut Appearance), Changed<Freezer>>) {
    for (freezer, mut look) in &mut freezers {
        let state = if freezer.on { "freezer_1" } else { "freezer" };
        if look.state != state {
            look.state = state.to_owned();
        }
    }
}
