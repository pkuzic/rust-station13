//! Power monitoring computers: the grid's supply and load, and every APC. Spec:
//! `docs/reference/power.md` (power monitor).

use bevy_ecs::prelude::*;
use rs13_world::{
    Appearance, Dense, GridPos, LightSource, MobStat, NetId, ObjectSpec, PowerMonitorRow, PowerMonitorView,
    TilePos,
};

use crate::actions::in_reach;
use crate::health::Vitals;
use crate::power::{Apc, Channel, ChannelMode, GridTotals, PowerUser};
use crate::wires::PanelViewer;

/// The computer's prototype.
pub const POWER_MONITOR: &str = "power-monitor";
/// Its draw while it has power, W (Luna `use_power(250)`).
pub const MONITOR_POWER: f64 = 250.0;
/// Its screen's glow (Luna `ul_SetLuminosity(0, 0, 2)`).
pub const SCREEN_LIGHT: [u8; 3] = [0, 0, 2];

/// A power monitoring computer.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerMonitor;

/// What the mob's open power monitor shows, for the server to send when it changes.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct PowerMonitorOut(pub Option<PowerMonitorView>);

pub fn spawn_power_monitor(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(spec.id.as_str(), "power"),
            PowerMonitor,
            PowerUser::new(Channel::Equipment, MONITOR_POWER),
            LightSource(SCREEN_LIGHT),
            Dense,
        ))
        .id()
}

/// Luna `attack_hand`: a working monitor in reach opens its window. `None` when `target`
/// is no monitor.
pub fn open(
    monitors: &Query<(&GridPos, &PowerUser), With<PowerMonitor>>,
    mob: Entity,
    mob_pos: TilePos,
    target: Entity,
    commands: &mut Commands,
) -> Option<()> {
    let (pos, power) = monitors.get(target).ok()?;
    if power.powered && in_reach(mob_pos, pos.0) {
        commands.entity(mob).insert(PanelViewer(target));
    }
    Some(())
}

/// A monitor without power goes dark (Luna `power_change`).
pub fn update_monitor_looks(
    mut monitors: Query<(Entity, &PowerUser, &mut Appearance, Has<LightSource>), With<PowerMonitor>>,
    mut commands: Commands,
) {
    for (entity, power, mut look, lit) in &mut monitors {
        let state = if power.powered { "power" } else { "unpowered" };
        if look.state != state {
            look.state = state.to_owned();
        }
        if power.powered && !lit {
            commands.entity(entity).insert(LightSource(SCREEN_LIGHT));
        } else if !power.powered && lit {
            commands.entity(entity).remove::<LightSource>();
        }
    }
}

fn mode_number(mode: ChannelMode) -> u8 {
    match mode {
        ChannelMode::Off => 0,
        ChannelMode::AutoOff => 1,
        ChannelMode::On => 2,
        ChannelMode::AutoOn => 3,
    }
}

/// Keep each viewer's monitor window in step with the grid (Luna `interact`): total
/// supply and load, and every APC, sorted by area. It closes out of reach or without power.
pub fn update_monitor_views(
    totals: Res<GridTotals>,
    mut viewers: Query<(
        Entity,
        &GridPos,
        Option<&PanelViewer>,
        &mut PowerMonitorOut,
        Option<&Vitals>,
    )>,
    monitors: Query<(&GridPos, &PowerUser, &NetId), With<PowerMonitor>>,
    apcs: Query<&Apc>,
    mut commands: Commands,
) {
    let mut rows: Vec<PowerMonitorRow> = apcs
        .iter()
        .map(|apc| PowerMonitorRow {
            area: apc.area.clone(),
            channels: apc.channels.map(mode_number),
            load: apc.last_used.iter().sum::<f64>().round() as u32,
            cell_percent: apc
                .cell
                .map(|_| apc.cell_percent().round().clamp(0.0, 100.0) as u8),
            charging: apc.charging,
        })
        .collect();
    rows.sort_by(|a, b| a.area.cmp(&b.area));
    for (mob, pos, viewer, mut out, vitals) in &mut viewers {
        let target = viewer.and_then(|v| monitors.get(v.0).ok());
        let view = target.and_then(|(monitor_pos, power, id)| {
            let usable = power.powered
                && in_reach(pos.0, monitor_pos.0)
                && vitals.is_none_or(|v| v.stat == MobStat::Conscious);
            usable.then(|| PowerMonitorView {
                target: *id,
                supply: totals.supply.round() as u32,
                load: totals.load.round() as u32,
                apcs: rows.clone(),
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
