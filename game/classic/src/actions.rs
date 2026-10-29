//! Player and NPC actions other than movement: clicking things, dropping, swapping
//! hands, examining. Spec: `docs/reference/hands.md`.

use std::collections::HashSet;

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, DisplayName, GridPos, Intent, MobStat, TilePos};

use crate::access::{IdCard, allowed};
use crate::combat::{CombatIntent, PendingAttacks};
use crate::doors::Airlock;
use crate::hands::{Hands, HeldBy, Item};
use crate::health::{Body, Vitals};
use crate::machines::{Canister, Connector, GasStore, PipeMember, PipeNets, Switch, Valve};
use crate::movement::Mob;
use crate::power::{APC_ACCESS, Apc, Channel, Smes};
use crate::time::WorldTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Click on something with the active hand: pick it up, or use it / the held item on it.
    UseOn(Entity),
    /// Put the active-hand item on the floor.
    Drop,
    SwapHands,
    Examine(Entity),
    SetIntent(Intent),
}

/// Actions waiting for the next tick, in the order they were requested.
#[derive(Component, Debug, Default)]
pub struct PendingActions(pub Vec<Action>);

/// Messages for the player controlling this mob, waiting to be sent.
#[derive(Component, Debug, Default)]
pub struct Outbox(pub Vec<LocMsg>);

/// Same tile or one of the eight neighbours, on the same deck.
pub fn in_reach(a: TilePos, b: TilePos) -> bool {
    a.z == b.z && (a.x - b.x).abs() <= 1 && (a.y - b.y).abs() <= 1
}

type ActingMob<'a> = (
    Entity,
    &'a GridPos,
    &'a mut Hands,
    &'a mut PendingActions,
    &'a mut Outbox,
    Option<&'a mut CombatIntent>,
    Option<&'a Vitals>,
);

pub fn process_actions(
    time: Res<WorldTime>,
    mut mobs: Query<ActingMob, With<Mob>>,
    mut targets: Query<
        (
            Option<&GridPos>,
            Has<Item>,
            Option<&mut Airlock>,
            Option<&mut Switch>,
            Option<&mut PipeMember>,
            Has<Canister>,
            Has<Valve>,
        ),
        Without<Mob>,
    >,
    ports: Query<&GridPos, (With<Connector>, Without<Mob>)>,
    stores: Query<&GasStore>,
    mut nets: ResMut<PipeNets>,
    mut apcs: Query<&mut Apc>,
    batteries: Query<&Smes>,
    cards: Query<&IdCard>,
    people: Query<(), With<Body>>,
    mut attacks: ResMut<PendingAttacks>,
    appearances: Query<(&Appearance, Option<&DisplayName>)>,
    mut commands: Commands,
) {
    let now = time.deciseconds();
    // Items picked up earlier this tick (component changes apply after the system).
    let mut taken: HashSet<Entity> = HashSet::new();

    for (mob, mob_pos, mut hands, mut pending, mut outbox, mut intent, vitals) in &mut mobs {
        if vitals.is_some_and(|v| v.stat != MobStat::Conscious) {
            pending.0.clear();
            continue;
        }
        for action in pending.0.drain(..) {
            match action {
                Action::SwapHands => hands.active = hands.active.other(),
                Action::SetIntent(new) => {
                    if let Some(intent) = intent.as_mut() {
                        intent.0 = new;
                    }
                }
                Action::Drop => {
                    let active = hands.active;
                    if let Some(item) = hands.slot_mut(active).take() {
                        commands
                            .entity(item)
                            .remove::<HeldBy>()
                            .insert(GridPos(mob_pos.0));
                    }
                }
                Action::Examine(target) => match appearances.get(target) {
                    Ok((_, Some(name))) => {
                        outbox
                            .0
                            .push(LocMsg::new("examine-person").arg("name", name.0.clone()));
                    }
                    Ok((appearance, None)) => {
                        outbox
                            .0
                            .push(LocMsg::new("examine").arg("item", appearance.sprite.clone()));
                        if let Ok(apc) = apcs.get(target) {
                            let mode = |c: Channel| {
                                if apc.operating && apc.channels[c as usize].is_on() {
                                    "on"
                                } else {
                                    "off"
                                }
                            };
                            outbox.0.push(
                                LocMsg::new("examine-apc")
                                    .num("value", apc.cell_percent().round() as i64)
                                    .arg("equipment", mode(Channel::Equipment))
                                    .arg("lighting", mode(Channel::Lighting))
                                    .arg("environment", mode(Channel::Environment)),
                            );
                        }
                        if let Ok(smes) = batteries.get(target) {
                            let percent = (100.0 * smes.charge / smes.capacity).round() as i64;
                            outbox.0.push(LocMsg::new("examine-smes").num("value", percent));
                        }
                        // Anything holding gas has a pressure gauge.
                        if let Ok((_, _, _, _, Some(member), ..)) = targets.get(target) {
                            let kpa = nets.gauge(target, stores.get(target).ok(), member.volume);
                            outbox
                                .0
                                .push(LocMsg::new("examine-gauge").num("value", kpa.round() as i64));
                        }
                    }
                    Err(_) => {}
                },
                Action::UseOn(target) if people.contains(target) => {
                    // Reach and the outcome are worked out by `resolve_attacks`.
                    attacks.0.push((mob, target));
                }
                Action::UseOn(target) => {
                    let Ok((Some(target_pos), is_item, airlock, switch, member, is_canister, is_valve)) =
                        targets.get_mut(target)
                    else {
                        // Held by someone, or gone.
                        continue;
                    };
                    if !in_reach(mob_pos.0, target_pos.0) {
                        continue;
                    }
                    if is_item && hands.active_item().is_none() && taken.insert(target) {
                        let active = hands.active;
                        *hands.slot_mut(active) = Some(target);
                        commands.entity(target).remove::<GridPos>().insert(HeldBy(mob));
                    } else if is_canister && holding(&hands, &appearances, "wrench") {
                        // Wrench a canister on or off the port under it.
                        let Some(mut member) = member else { continue };
                        if member.open || ports.iter().any(|p| p.0 == target_pos.0) {
                            member.open = !member.open;
                            nets.dirty = true;
                            let id = if member.open {
                                "canister-connected"
                            } else {
                                "canister-disconnected"
                            };
                            outbox.0.push(LocMsg::new(id).arg("item", "canister"));
                        }
                    } else if let Ok(mut apc) = apcs.get_mut(target) {
                        let card = hands.active_item().and_then(|e| cards.get(e).ok());
                        if allowed(&[APC_ACCESS.to_owned()], card) {
                            apc.operating = !apc.operating;
                            let id = if apc.operating {
                                "apc-breaker-on"
                            } else {
                                "apc-breaker-off"
                            };
                            outbox.0.push(LocMsg::new(id));
                        } else {
                            outbox.0.push(LocMsg::new("apc-locked"));
                        }
                    } else if let Some(mut switch) = switch {
                        switch.0 = !switch.0;
                        let sprite = appearances
                            .get(target)
                            .map(|(a, _)| a.sprite.clone())
                            .unwrap_or_default();
                        let id = match (is_valve, switch.0) {
                            (true, true) => "valve-opened",
                            (true, false) => "valve-closed",
                            (false, true) => "switched-on",
                            (false, false) => "switched-off",
                        };
                        if is_valve && let Some(mut member) = member {
                            member.open = switch.0;
                            nets.dirty = true;
                        }
                        outbox.0.push(LocMsg::new(id).arg("item", sprite));
                    } else if let Some(mut airlock) = airlock {
                        if holding(&hands, &appearances, "crowbar") && airlock.force(now) {
                            outbox.0.push(LocMsg::new("door-forced").arg("item", "airlock"));
                            continue;
                        }
                        // By hand or with any item; only an ID card in the active hand
                        // counts for access.
                        let card = hands.active_item().and_then(|e| cards.get(e).ok());
                        airlock.use_by(now, card);
                    }
                }
            }
        }
    }
}

/// Whether the active hand holds an item with this sprite id.
fn holding(hands: &Hands, appearances: &Query<(&Appearance, Option<&DisplayName>)>, sprite: &str) -> bool {
    hands
        .active_item()
        .and_then(|e| appearances.get(e).ok())
        .is_some_and(|(a, _)| a.sprite == sprite)
}
