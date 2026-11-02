//! The window of a canister, portable pump or scrubber (Luna's `canister`, `pump` and
//! `scrubber` windows), and the gas tank one can hold. Spec: `docs/reference/portables.md`,
//! `docs/reference/gas-tanks.md`.

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_simulation::atmos::gas::ONE_ATMOSPHERE;
use rs13_world::{GasMachine, GasPanelView, GridPos, MobStat, NetId, TilePos};

use crate::actions::{ClickOutcome, in_reach};
use crate::equipment::TankGas;
use crate::hands::{Hands, HeldBy};
use crate::health::Vitals;
use crate::machines::{Canister, GasStore, PipeMember, PipeNets, Portable, Switch};
use crate::portables::{PortablePump, PortableScrubber};
use crate::wires::PanelViewer;

/// Lowest and highest release pressure of a canister, kPa (Luna: a tenth of an
/// atmosphere to ten).
pub const RELEASE_MIN: f64 = ONE_ATMOSPHERE / 10.0;
pub const RELEASE_MAX: f64 = 10.0 * ONE_ATMOSPHERE;
/// Highest target of a pump, and (Luna's own clamp) highest rate of a scrubber.
pub const SETTING_MAX: f64 = 10.0 * ONE_ATMOSPHERE;

/// On a canister, pump or scrubber: the gas tank put in it (Luna `holding`). It works on
/// the tank's gas instead of the room's.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeldTank(pub Entity);

/// On a gas tank held in a canister, pump or scrubber (it is off the map).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InPortable(pub Entity);

/// Window buttons waiting for the next tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GasOp {
    /// A canister's release valve, or a pump's or scrubber's power.
    Toggle,
    /// A pump's direction.
    Direction,
    /// Change the release pressure, target or rate by this much.
    Adjust(f64),
    /// Take the held tank out, onto the floor.
    RemoveTank,
}

#[derive(Component, Debug, Default)]
pub struct PendingGasOps(pub Vec<(Entity, GasOp)>);

/// What the mob's open gas window shows, for the server to send when it changes.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct GasPanelOut(pub Option<GasPanelView>);

/// Everything clicking a portable needs.
#[derive(SystemParam)]
pub struct GasPanelWork<'w, 's> {
    portables: Query<'w, 's, (&'static GridPos, Has<HeldTank>), With<Portable>>,
    tanks: Query<'w, 's, (), With<TankGas>>,
}

impl GasPanelWork<'_, '_> {
    pub fn contains(&self, target: Entity) -> bool {
        self.portables.contains(target)
    }

    /// Luna `attackby` with a gas tank: it goes in, if none is there. Otherwise
    /// `attack_hand`: the window opens. The caller checks reach (and wrenches).
    pub fn use_on(
        &self,
        mob: Entity,
        target: Entity,
        hands: &mut Hands,
        commands: &mut Commands,
    ) -> Option<ClickOutcome> {
        let (_, holding) = self.portables.get(target).ok()?;
        match hands.active_item() {
            Some(tank) if self.tanks.contains(tank) => {
                if !holding {
                    *hands.slot_mut(hands.active) = None;
                    commands
                        .entity(tank)
                        .remove::<HeldBy>()
                        .insert(InPortable(target));
                    commands.entity(target).insert(HeldTank(tank));
                }
            }
            _ => {
                commands.entity(mob).insert(PanelViewer(target));
            }
        }
        Some(ClickOutcome::default())
    }
}

type Machine<'a> = (
    &'a GridPos,
    &'a mut Switch,
    Option<&'a mut Canister>,
    Option<&'a mut PortablePump>,
    Option<&'a mut PortableScrubber>,
    Option<&'a HeldTank>,
);

/// Apply window buttons (Luna `Topic`), in reach and awake.
pub fn process_gas_ops(
    mut mobs: Query<(&GridPos, &mut PendingGasOps, Option<&Vitals>)>,
    mut machines: Query<Machine, With<Portable>>,
    mut commands: Commands,
) {
    for (pos, mut pending, vitals) in &mut mobs {
        let awake = vitals.is_none_or(|v| v.stat == MobStat::Conscious);
        for (target, op) in pending.0.drain(..) {
            let Ok((at, mut switch, canister, pump, scrubber, held)) = machines.get_mut(target) else {
                continue;
            };
            if !awake || !in_reach(pos.0, at.0) {
                continue;
            }
            match op {
                GasOp::Toggle => switch.0 = !switch.0,
                GasOp::Direction => {
                    if let Some(mut pump) = pump {
                        pump.out = !pump.out;
                    }
                }
                GasOp::Adjust(by) => {
                    if let Some(mut c) = canister {
                        c.release_target = (c.release_target + by).clamp(RELEASE_MIN, RELEASE_MAX);
                    } else if let Some(mut p) = pump {
                        p.target = (p.target + by).clamp(0.0, SETTING_MAX);
                    } else if let Some(mut s) = scrubber {
                        s.rate = (s.rate + by).clamp(0.0, SETTING_MAX);
                    }
                }
                GasOp::RemoveTank => {
                    if let Some(held) = held {
                        release_tank(&mut commands, target, held.0, at.0);
                    }
                }
            }
        }
    }
}

/// Put a held tank back on the floor at `at`.
fn release_tank(commands: &mut Commands, machine: Entity, tank: Entity, at: TilePos) {
    commands.entity(machine).remove::<HeldTank>();
    commands.entity(tank).remove::<InPortable>().insert(GridPos(at));
}

type Shown<'a> = (
    Entity,
    &'a GridPos,
    &'a NetId,
    &'a Switch,
    &'a PipeMember,
    &'a GasStore,
    Option<&'a Canister>,
    Option<&'a PortablePump>,
    Option<&'a PortableScrubber>,
    Option<&'a HeldTank>,
);

/// Keep each viewer's gas window in step; close it out of reach.
pub fn update_gas_views(
    nets: Res<PipeNets>,
    mut viewers: Query<(
        Entity,
        &GridPos,
        Option<&PanelViewer>,
        &mut GasPanelOut,
        Option<&Vitals>,
    )>,
    machines: Query<Shown, With<Portable>>,
    tanks: Query<&TankGas>,
    mut commands: Commands,
) {
    let dk = |kpa: f64| (kpa * 10.0).round().max(0.0) as u32;
    for (mob, pos, viewer, mut out, vitals) in &mut viewers {
        let target = viewer.and_then(|v| machines.get(v.0).ok());
        let view = target.and_then(
            |(entity, at, id, switch, member, store, canister, pump, scrubber, held)| {
                let usable = in_reach(pos.0, at.0) && vitals.is_none_or(|v| v.stat == MobStat::Conscious);
                if !usable {
                    return None;
                }
                let (kind, setting, out) = match (canister, pump, scrubber) {
                    (Some(c), ..) => (GasMachine::Canister, c.release_target, false),
                    (_, Some(p), _) => (GasMachine::Pump, p.target, p.out),
                    (_, _, Some(s)) => (GasMachine::Scrubber, s.rate, false),
                    _ => return None,
                };
                Some(GasPanelView {
                    target: *id,
                    kind,
                    pressure_dkpa: dk(nets.gauge(entity, Some(store), member.volume)),
                    connected: member.open,
                    on: switch.0,
                    out,
                    setting: setting.round().max(0.0) as u32,
                    tank_dkpa: held
                        .and_then(|h| tanks.get(h.0).ok())
                        .map(|t| dk(t.mix.pressure(t.volume))),
                })
            },
        );
        if view.is_none() && target.is_some() {
            commands.entity(mob).remove::<PanelViewer>();
        }
        if out.0 != view {
            out.0 = view;
        }
    }
}
