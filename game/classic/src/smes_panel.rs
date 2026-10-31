//! Working a SMES: its window and its lights. Spec: `docs/reference/power.md` (SMES).

use bevy_ecs::prelude::*;
use rs13_world::{Appearance, GridPos, MobStat, NetId, SmesPanelView, TilePos};

use crate::actions::in_reach;
use crate::health::Vitals;
use crate::power::{SMES_MAX_LEVEL, Smes};
use crate::wires::PanelViewer;

/// A SMES window button.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SmesOp {
    ChargeMode,
    Online,
    Input(Level),
    Output(Level),
}

/// How a level button changes it (Luna's M, -, +, M links).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Level {
    Min,
    Max,
    Change(f64),
}

impl Level {
    fn apply(self, level: f64) -> f64 {
        match self {
            Level::Min => 0.0,
            Level::Max => SMES_MAX_LEVEL,
            Level::Change(delta) => (level + delta).clamp(0.0, SMES_MAX_LEVEL),
        }
    }
}

#[derive(Component, Debug, Default)]
pub struct PendingSmesOps(pub Vec<(Entity, SmesOp)>);

/// What the mob's open SMES window shows, for the server to send when it changes.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct SmesPanelOut(pub Option<SmesPanelView>);

/// An empty hand on a SMES in reach opens its window (Luna `attack_hand`). `None` when
/// `target` is no SMES.
pub fn open(
    smes: &Query<&GridPos, With<Smes>>,
    mob: Entity,
    mob_pos: TilePos,
    target: Entity,
    commands: &mut Commands,
) -> Option<()> {
    let pos = smes.get(target).ok()?;
    if in_reach(mob_pos, pos.0) {
        commands.entity(mob).insert(PanelViewer(target));
    }
    Some(())
}

/// Apply SMES window buttons (Luna `smes/Topic`), in reach. Levels stay within 0–200 kW.
pub fn process_smes_ops(
    mut mobs: Query<(&GridPos, &mut PendingSmesOps, Option<&Vitals>)>,
    mut units: Query<(&GridPos, &mut Smes)>,
) {
    for (pos, mut pending, vitals) in &mut mobs {
        let awake = vitals.is_none_or(|v| v.stat == MobStat::Conscious);
        for (target, op) in pending.0.drain(..) {
            let Ok((at, mut smes)) = units.get_mut(target) else {
                continue;
            };
            if !awake || !in_reach(pos.0, at.0) {
                continue;
            }
            match op {
                SmesOp::ChargeMode => {
                    smes.charge_auto = !smes.charge_auto;
                    if !smes.charge_auto {
                        smes.charging = false;
                    }
                }
                SmesOp::Online => smes.online = !smes.online,
                SmesOp::Input(level) => smes.input = level.apply(smes.input),
                SmesOp::Output(level) => smes.output = level.apply(smes.output),
            }
        }
    }
}

/// Keep each viewer's SMES window in step; close it out of reach.
pub fn update_smes_views(
    mut viewers: Query<(
        Entity,
        &GridPos,
        Option<&PanelViewer>,
        &mut SmesPanelOut,
        Option<&Vitals>,
    )>,
    units: Query<(&GridPos, &Smes, &NetId)>,
    mut commands: Commands,
) {
    for (mob, pos, viewer, mut out, vitals) in &mut viewers {
        let target = viewer.and_then(|v| units.get(v.0).ok());
        let view = target.and_then(|(at, smes, id)| {
            let usable = in_reach(pos.0, at.0) && vitals.is_none_or(|v| v.stat == MobStat::Conscious);
            usable.then(|| SmesPanelView {
                target: *id,
                charge_percent: (100.0 * smes.charge / smes.capacity.max(1.0))
                    .round()
                    .clamp(0.0, 100.0) as u8,
                charging: smes.charging,
                charge_auto: smes.charge_auto,
                input: smes.input.round() as u32,
                output: smes.output.round() as u32,
                online: smes.online,
                load: smes.last_output.round() as u32,
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

/// A SMES's lights (Luna `update_icon`): output on or off, charging (or set to charge by
/// itself), and its charge in five steps.
pub fn update_smes_looks(mut units: Query<(&Smes, &mut Appearance), Changed<Smes>>) {
    for (smes, mut look) in &mut units {
        let mut state = format!("+smes-op{}", u8::from(smes.online));
        if smes.charging {
            state.push_str("+smes-oc1");
        } else if smes.charge_auto {
            state.push_str("+smes-oc0");
        }
        let level = (5.5 * smes.charge / smes.capacity.max(1.0)).round() as u8;
        if level > 0 {
            state.push_str(&format!("+smes-og{}", level.min(5)));
        }
        if look.state != state {
            look.state = state;
        }
    }
}
