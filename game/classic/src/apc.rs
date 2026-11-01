//! Working an APC: its interface lock, cover, cell and control window. Spec:
//! `docs/reference/power.md` (APC).

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_world::{ApcPanelView, GridPos, MobStat, NetId, ObjectSpec, TilePos};

use crate::access::{IdCard, allowed};
use crate::actions::{ClickOutcome, in_reach};
use crate::cells::PowerCell;
use crate::hands::{Hands, HeldBy};
use crate::health::Vitals;
use crate::power::{APC_ACCESS, Apc, ChannelMode};
use crate::wires::PanelViewer;

/// How far away an open APC window stays open (the alarm window's range).
const VIEW_RANGE: i32 = 1;

/// A button pressed on the APC window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApcCommand {
    Breaker,
    /// A channel (0 equipment, 1 lighting, 2 environment) set to a mode.
    Channel(usize, ChannelMode),
    ChargeMode,
    CoverLock,
}

/// APC window buttons waiting for the next tick: which APC, what.
#[derive(Component, Debug, Default)]
pub struct PendingApcOps(pub Vec<(Entity, ApcCommand)>);

/// What the mob's open APC window shows, for the server to send when it changes.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct ApcPanelOut(pub Option<ApcPanelView>);

/// Everything clicking an APC needs.
#[derive(SystemParam)]
pub struct ApcWork<'w, 's> {
    apcs: Query<'w, 's, &'static mut Apc>,
    cells: Query<'w, 's, &'static PowerCell>,
    cards: Query<'w, 's, &'static IdCard>,
}

impl ApcWork<'_, '_> {
    pub fn contains(&self, target: Entity) -> bool {
        self.apcs.contains(target)
    }

    pub fn get(&self, target: Entity) -> Option<&Apc> {
        self.apcs.get(target).ok()
    }

    /// Luna `attackby` and `attack_hand`:
    /// - an ID card locks or unlocks the interface, with the cover shut;
    /// - a crowbar opens the cover, unless it is locked, or shuts it;
    /// - a cell goes in through the open cover;
    /// - an empty hand takes the cell out of an open APC, or else opens its window.
    ///
    /// The caller checks reach; `at` is where the APC is.
    pub fn use_on(
        &mut self,
        mob: Entity,
        target: Entity,
        held: Option<&str>,
        hands: &mut Hands,
        at: TilePos,
        commands: &mut Commands,
    ) -> Option<ClickOutcome> {
        let mut apc = self.apcs.get_mut(target).ok()?;
        let active = hands.active_item();
        if let Some(card) = active.and_then(|e| self.cards.get(e).ok()) {
            return Some(ClickOutcome::told(if apc.opened {
                "apc-close-cover-first"
            } else if allowed(&[APC_ACCESS.to_owned()], Some(card)) {
                apc.locked = !apc.locked;
                if apc.locked {
                    "apc-interface-locked"
                } else {
                    "apc-interface-unlocked"
                }
            } else {
                "apc-locked"
            }));
        }
        if held == Some("crowbar") {
            return Some(ClickOutcome::told(if apc.opened {
                apc.opened = false;
                "apc-cover-closed"
            } else if apc.cover_locked {
                "apc-cover-locked"
            } else {
                apc.opened = true;
                "apc-cover-opened"
            }));
        }
        if let Some(item) = active
            && let Ok(cell) = self.cells.get(item)
        {
            if !apc.opened {
                return None;
            }
            if apc.cell.is_some() {
                return Some(ClickOutcome::told("apc-has-cell"));
            }
            apc.cell = Some((cell.charge, cell.capacity));
            *hands.slot_mut(hands.active) = None;
            commands.entity(item).despawn();
            return Some(ClickOutcome::told("apc-cell-inserted"));
        }
        if active.is_some() {
            return None;
        }
        if apc.opened {
            let (charge, capacity) = apc.cell.take()?;
            let hand = hands.active;
            commands.queue(move |world: &mut World| {
                let proto = if capacity > 2.5e6 {
                    "high-power-cell"
                } else {
                    "power-cell"
                };
                let Ok(cell) = crate::objects::spawn_object(world, &ObjectSpec::new(proto), at) else {
                    return;
                };
                world
                    .entity_mut(cell)
                    .insert(PowerCell { charge, capacity })
                    .remove::<GridPos>()
                    .insert(HeldBy(mob));
                if let Some(mut hands) = world.get_mut::<Hands>(mob) {
                    *hands.slot_mut(hand) = Some(cell);
                }
            });
            return Some(ClickOutcome::told("apc-cell-removed"));
        }
        commands.entity(mob).insert(PanelViewer(target));
        Some(ClickOutcome::default())
    }
}

/// Apply APC window buttons: the APC must be in reach and its interface unlocked.
pub fn process_apc_ops(
    mut mobs: Query<(&GridPos, &mut PendingApcOps, Option<&Vitals>)>,
    mut apcs: Query<(&GridPos, &mut Apc)>,
) {
    for (pos, mut pending, vitals) in &mut mobs {
        let awake = vitals.is_none_or(|v| v.stat == MobStat::Conscious);
        for (target, command) in pending.0.drain(..) {
            let Ok((apc_pos, mut apc)) = apcs.get_mut(target) else {
                continue;
            };
            if !awake || apc.locked || !in_reach(pos.0, apc_pos.0) {
                continue;
            }
            match command {
                ApcCommand::Breaker => apc.operating = !apc.operating,
                ApcCommand::Channel(channel, mode) => {
                    if let Some(slot) = apc.channels.get_mut(channel) {
                        *slot = mode;
                    }
                }
                ApcCommand::ChargeMode => apc.charge_auto = !apc.charge_auto,
                ApcCommand::CoverLock => apc.cover_locked = !apc.cover_locked,
            }
        }
    }
}

/// Luna's channel numbers: 0 off, 1 auto-off, 2 on, 3 auto-on.
fn mode_number(mode: ChannelMode) -> u8 {
    match mode {
        ChannelMode::Off => 0,
        ChannelMode::AutoOff => 1,
        ChannelMode::On => 2,
        ChannelMode::AutoOn => 3,
    }
}

/// Keep each viewer's APC window in step with the APC; close it once out of range.
pub fn update_apc_views(
    map: crate::decks::Decks,
    mut viewers: Query<(
        Entity,
        &GridPos,
        Option<&PanelViewer>,
        &mut ApcPanelOut,
        Option<&Vitals>,
    )>,
    apcs: Query<(&GridPos, &Apc, &NetId)>,
    mut commands: Commands,
) {
    for (mob, pos, viewer, mut out, vitals) in &mut viewers {
        let target = viewer.and_then(|v| apcs.get(v.0).ok());
        let view = target.and_then(|(apc_pos, apc, id)| {
            let near = apc_pos.0.z == pos.0.z
                && (apc_pos.0.x - pos.0.x).abs() <= VIEW_RANGE
                && (apc_pos.0.y - pos.0.y).abs() <= VIEW_RANGE;
            if !near || apc.opened || vitals.is_some_and(|v| v.stat != MobStat::Conscious) {
                return None;
            }
            Some(ApcPanelView {
                target: *id,
                area: map.area(apc_pos.0).unwrap_or_default().to_owned(),
                locked: apc.locked,
                breaker: apc.operating,
                external: apc.main_status,
                cell_percent: apc
                    .cell
                    .map(|_| apc.cell_percent().round().clamp(0.0, 100.0) as u8),
                charging: apc.charging,
                charge_auto: apc.charge_auto,
                channels: std::array::from_fn(|i| {
                    (mode_number(apc.channels[i]), apc.last_used[i].round() as u32)
                }),
                cover_locked: apc.cover_locked,
                controls: in_reach(pos.0, apc_pos.0),
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
