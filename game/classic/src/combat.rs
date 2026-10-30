//! Hitting people, with items or fists, and telling onlookers.
//! Spec: `docs/reference/health.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, DisplayName, GridPos, Intent, MapLayer, MobStat, Opaque, TilePos};

use crate::actions::{Outbox, in_reach};
use crate::hands::{Hands, Item};
use crate::health::{Body, Vitals, Zone, ran_zone};
use crate::movement::Mob;
use crate::rng::GameRng;
use crate::speech::{HEARING_RANGE, can_see};

/// The intent a mob acts with (Luna `a_intent`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CombatIntent(pub Intent);

/// Clicks on people, waiting to be resolved this tick.
#[derive(Resource, Debug, Default)]
pub struct PendingAttacks(pub Vec<(Entity, Entity)>);

/// Things that happened at a place, to be shown to everyone who can see it.
#[derive(Resource, Debug, Default)]
/// Where it happened, the message, and who should not get it (they get their own).
pub struct VisibleEvents(pub Vec<(TilePos, LocMsg, Option<Entity>)>);

/// Without a zone selector, every blow is aimed at the chest (Luna's default).
const AIMED_ZONE: Zone = Zone::Chest;

pub fn resolve_attacks(
    mut pending: ResMut<PendingAttacks>,
    mut rng: ResMut<GameRng>,
    mut visible: ResMut<VisibleEvents>,
    attackers: Query<(&GridPos, &Hands, &CombatIntent, &DisplayName)>,
    mut targets: Query<(&GridPos, &mut Body, &DisplayName)>,
    items: Query<(&Item, &Appearance, Option<&crate::welding::Welder>)>,
) {
    for (attacker, target) in pending.0.drain(..) {
        let Ok((from, hands, intent, attacker_name)) = attackers.get(attacker) else {
            continue;
        };
        let attacker_name = attacker_name.0.clone();
        let weapon = hands.active_item().and_then(|e| items.get(e).ok());
        let intent = intent.0;
        let from = from.0;
        let Ok((to, mut body, victim_name)) = targets.get_mut(target) else {
            continue;
        };
        if !in_reach(from, to.0) {
            continue;
        }
        let at = to.0;
        let victim = victim_name.0.clone();

        if let Some((item, appearance, welder)) = weapon {
            // In this era a held item hits on any intent. A lit welding tool burns.
            let zone = ran_zone(AIMED_ZONE, &mut rng);
            if welder.is_some_and(|w| w.lit) {
                body.take_damage(zone, 0.0, crate::welding::LIT_FORCE);
            } else {
                body.take_damage(zone, item.force, 0.0);
            }
            let msg = LocMsg::new("attacked-with")
                .arg("attacker", attacker_name)
                .arg("victim", victim)
                .arg("item", appearance.sprite.clone())
                .arg("zone", zone.noun_id());
            visible.0.push((at, msg, None));
        } else if intent == Intent::Harm {
            let damage = rng.range(1, 9) as f32;
            let zone = ran_zone(AIMED_ZONE, &mut rng);
            let id = if !rng.prob(90.0) {
                "punch-missed"
            } else {
                body.take_damage(zone, damage, 0.0);
                if zone == Zone::Head {
                    "punched-face"
                } else {
                    "punched"
                }
            };
            let msg = LocMsg::new(id)
                .arg("attacker", attacker_name)
                .arg("victim", victim);
            visible.0.push((at, msg, None));
        }
        // Help intent, empty hand: shaking and hugging are not implemented yet.
    }
}

/// Show each event to every conscious mob that can see where it happened.
pub fn deliver_visible(
    mut visible: ResMut<VisibleEvents>,
    map: Single<&MapLayer>,
    mut viewers: Query<(Entity, &GridPos, &mut Outbox, Option<&Vitals>), With<Mob>>,
    opaque: Query<&GridPos, (With<Opaque>, Without<Mob>)>,
) {
    if visible.0.is_empty() {
        return;
    }
    let opaque: Vec<TilePos> = opaque.iter().map(|p| p.0).collect();
    for (at, msg, except) in visible.0.drain(..) {
        for (viewer, pos, mut outbox, vitals) in &mut viewers {
            if except == Some(viewer) {
                continue;
            }
            let awake = vitals.is_none_or(|v| v.stat == MobStat::Conscious);
            if awake && can_see(&map.0, &opaque, at, pos.0, HEARING_RANGE) {
                outbox.0.push(msg.clone());
            }
        }
    }
}
