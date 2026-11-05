//! Hitting people, with items or fists, and telling onlookers.
//! Spec: `docs/reference/health.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, DisplayName, GridPos, Intent, MobStat, Opaque, Slot, TilePos};

use crate::actions::{Outbox, in_reach};
use crate::equipment::Equipment;
use crate::flashes::Weakened;
use crate::hands::{Hands, HeldBy, Item};
use crate::health::{Body, Vitals, Zone, ran_zone};
use crate::movement::Mob;
use crate::rng::GameRng;
use crate::speech::{HEARING_RANGE, can_see, within_range};
use crate::time::WorldTime;

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

/// The body part someone is aiming at (Luna's `zone_sel`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct AimZone(pub Zone);

impl Default for AimZone {
    fn default() -> Self {
        Self(Zone::Chest)
    }
}

/// Unconscious until this time, ds, however healthy (Luna `paralysis`).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Paralyzed(pub f32);

/// Can't be hurt (Luna's `nodamage`, from the admin verb).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Godmode;

/// Can't speak (Luna's `muted`, from the admin verb).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Muted;

/// Whoever can't be hurt is made whole again each tick.
pub fn keep_gods_whole(mut gods: Query<&mut Body, With<Godmode>>) {
    for mut body in &mut gods {
        *body = Body::default();
    }
}

/// How long one of Luna's stun counts lasts, ds: its mobs count down once every 2 seconds.
pub const COUNT_DS: f32 = 20.0;
/// Hats that stop a weapon (Luna's `head/helmet`).
pub const HELMETS: [&str; 9] = [
    "helmet",
    "riot-helmet",
    "space-helmet",
    "welding-helmet",
    "hardhat",
    "hardhat-orange",
    "hardhat-red",
    "hardhat-white",
    "hardhat-blue",
];
/// Suits that stop a weapon (Luna's `suit/armor`).
pub const ARMOUR: [&str; 7] = [
    "armor-vest",
    "armor-rig-vest",
    "ablative-vest",
    "captains-armor",
    "riot-suit",
    "blue-tag-armor",
    "red-tag-armor",
];

/// What a target is wearing, as far as stopping a blow goes.
#[derive(Debug, Clone, Copy, Default)]
struct Worn {
    helmet: bool,
    /// A helmet, or a mask over the head.
    head_covered: bool,
    armour: bool,
    /// A suit or a uniform.
    body_covered: bool,
}

fn worn(equipment: Option<&Equipment>, looks: &Query<&Appearance>) -> Worn {
    let on = |slot: Slot| {
        equipment
            .and_then(|e| e.get(slot))
            .and_then(|item| looks.get(item).ok())
            .map(|look| look.sprite.as_str())
    };
    let hat = on(Slot::Head);
    let suit = on(Slot::Suit);
    let helmet = hat.is_some_and(|h| HELMETS.contains(&h));
    Worn {
        helmet,
        head_covered: helmet || on(Slot::Mask) == Some("gas-mask"),
        armour: suit.is_some_and(|s| ARMOUR.contains(&s)),
        body_covered: suit.is_some() || on(Slot::Uniform).is_some(),
    }
}

/// Everything that came of a blow, to be done once the queries are let go.
#[derive(Debug, Default)]
struct Consequence {
    weaken_ds: f32,
    paralyse_ds: f32,
}

fn knock_down(world: &mut World, target: Entity, consequence: Consequence) {
    let now = world.resource::<WorldTime>().deciseconds();
    if consequence.weaken_ds > 0.0 {
        let until = now + consequence.weaken_ds;
        if world.get::<Weakened>(target).is_none_or(|w| w.0 < until) {
            world.entity_mut(target).insert(Weakened(until));
        }
    }
    if consequence.paralyse_ds > 0.0 {
        let until = now + consequence.paralyse_ds;
        if world.get::<Paralyzed>(target).is_none_or(|p| p.0 < until) {
            world.entity_mut(target).insert(Paralyzed(until));
        }
    }
}

/// A shove makes someone let go of what's in their active hand.
fn drop_held(world: &mut World, target: Entity) {
    let Some(at) = world.get::<GridPos>(target).copied() else {
        return;
    };
    let item = world.get_mut::<Hands>(target).and_then(|mut hands| {
        let active = hands.active;
        hands.slot_mut(active).take()
    });
    if let Some(item) = item {
        world.entity_mut(item).remove::<HeldBy>().insert(at);
    }
}

/// Being shaken takes off some of what keeps someone down (Luna's 3 counts of each).
fn shake_awake(world: &mut World, target: Entity) {
    let less = 3.0 * COUNT_DS;
    if let Some(mut down) = world.get_mut::<Weakened>(target) {
        down.0 -= less;
    }
    if let Some(mut out) = world.get_mut::<Paralyzed>(target) {
        out.0 -= less;
    }
}

/// What someone finds checking themselves over (Luna's `attack_hand` on yourself).
fn check_self(body: &Body) -> Vec<LocMsg> {
    Zone::ALL
        .into_iter()
        .filter(|zone| *zone != Zone::Groin)
        .map(|zone| {
            let organ = body.organ(zone);
            let hurt = match organ.brute {
                b if b > 30.0 => Some("status-mangled"),
                b if b > 0.0 => Some("status-bruised"),
                _ => None,
            };
            let burnt = match organ.burn {
                b if b > 40.0 => Some("status-peeling"),
                b if b > 20.0 => Some("status-blistered"),
                b if b > 0.0 => Some("status-numb"),
                _ => None,
            };
            let msg = |id: &str| LocMsg::new(id).arg("zone", zone.noun_id());
            match (hurt, burnt) {
                (Some(a), Some(b)) => msg("self-part-two").msg("a", a).msg("b", b),
                (Some(a), None) | (None, Some(a)) => msg("self-part-one").msg("a", a),
                (None, None) => msg("self-part-ok"),
            }
        })
        .collect()
}

#[allow(clippy::type_complexity)]
pub fn resolve_attacks(
    mut pending: ResMut<PendingAttacks>,
    mut rng: ResMut<GameRng>,
    mut visible: ResMut<VisibleEvents>,
    attackers: Query<(&GridPos, &Hands, &CombatIntent, &DisplayName, Option<&AimZone>)>,
    mut targets: Query<(
        &GridPos,
        &mut Body,
        &DisplayName,
        Option<&Equipment>,
        Option<&mut crate::actions::Outbox>,
        Has<Weakened>,
    )>,
    items: Query<(&Item, &Appearance, Option<&crate::welding::Welder>)>,
    looks: Query<&Appearance>,
    mut commands: Commands,
) {
    for (attacker, target) in pending.0.drain(..) {
        let Ok((from, hands, intent, attacker_name, aim)) = attackers.get(attacker) else {
            continue;
        };
        let attacker_name = attacker_name.0.clone();
        let weapon = hands.active_item().and_then(|e| items.get(e).ok());
        let intent = intent.0;
        let from = from.0;
        let aimed = aim.map_or(Zone::Chest, |a| a.0);
        let Ok((to, mut body, victim_name, equipment, mut outbox, lying)) = targets.get_mut(target) else {
            continue;
        };
        if !in_reach(from, to.0) {
            continue;
        }
        let at = to.0;
        let victim = victim_name.0.clone();
        let gear = worn(equipment, &looks);
        let mut told = |id: &str| {
            if let Some(outbox) = outbox.as_mut() {
                outbox.0.push(LocMsg::new(id));
            }
        };
        let mut consequence = Consequence::default();

        if let Some((item, appearance, welder)) = weapon {
            // In this era a held item hits on any intent. A lit welding tool burns.
            let zone = ran_zone(aimed, &mut rng);
            let burning = welder.is_some_and(|w| w.lit);
            let msg = LocMsg::new("attacked-with")
                .arg("attacker", attacker_name)
                .arg("victim", victim.clone())
                .arg("item", appearance.sprite.clone())
                .arg("zone", zone.noun_id());
            visible.0.push((at, msg, None));
            if burning {
                body.take_damage(zone, 0.0, crate::welding::LIT_FORCE);
                continue;
            }
            let force = item.force;
            let organ = body.organ(zone);
            let knock_chance = force + organ.brute + organ.burn;
            match zone {
                Zone::Head => {
                    if force > 0.0 && gear.helmet && rng.prob(80.0 - force) {
                        if rng.prob(20.0) {
                            body.take_damage(zone, force, 0.0);
                        } else {
                            told("protected-head");
                        }
                        continue;
                    }
                    if force > 0.0 && rng.prob(knock_chance) {
                        let time = rng.range(10, 120) as f32 * COUNT_DS / 10.0;
                        if rng.prob(90.0) {
                            consequence.paralyse_ds = time;
                        } else {
                            consequence.weaken_ds = time;
                        }
                        commands.queue(move |world: &mut World| {
                            crate::revolution::knocked_on_head(world, target);
                        });
                        let msg = LocMsg::new("knocked-out").arg("victim", victim.clone());
                        visible.0.push((at, msg, None));
                    }
                }
                Zone::Chest => {
                    if force > 0.0 && gear.armour && rng.prob(90.0 - force) {
                        told("protected-chest");
                        continue;
                    }
                    if force > 0.0 && rng.prob(knock_chance) {
                        let id = if rng.prob(50.0) {
                            consequence.weaken_ds = 5.0 * COUNT_DS;
                            "knocked-down"
                        } else {
                            consequence.weaken_ds = 2.0 * COUNT_DS;
                            "stunned"
                        };
                        visible
                            .0
                            .push((at, LocMsg::new(id).arg("victim", victim.clone()), None));
                    }
                }
                Zone::Groin if force > 0.0 && gear.armour && rng.prob(90.0 - force) => {
                    told("protected-groin");
                    continue;
                }
                _ => {}
            }
            body.take_damage(zone, force, 0.0);
        } else if intent == Intent::Harm {
            let damage = rng.range(1, 9) as f32;
            let zone = ran_zone(aimed, &mut rng);
            let line = |id: &str| {
                LocMsg::new(id)
                    .arg("attacker", attacker_name.clone())
                    .arg("victim", victim.clone())
            };
            if !rng.prob(90.0) {
                visible.0.push((at, line("punch-missed"), None));
                continue;
            }
            let id = if zone == Zone::Head {
                "punched-face"
            } else {
                "punched"
            };
            visible.0.push((at, line(id), None));
            match zone {
                Zone::Head => {
                    if gear.head_covered && rng.prob(99.0) {
                        if rng.prob(20.0) {
                            body.take_damage(zone, damage, 0.0);
                        } else {
                            told("protected-head");
                        }
                        continue;
                    }
                    if damage > 7.0 {
                        consequence.weaken_ds = 8.0 * COUNT_DS;
                        visible.0.push((at, line("punch-weakened"), None));
                    }
                }
                Zone::Chest | Zone::Groin => {
                    let chest = zone == Zone::Chest;
                    if gear.body_covered && rng.prob(if chest { 85.0 } else { 75.0 }) {
                        told(if chest {
                            "protected-chest"
                        } else {
                            "protected-groin"
                        });
                        continue;
                    }
                    if damage > 7.0 {
                        let counts = if chest { 5.0 } else { 3.0 };
                        let id = if rng.prob(50.0) {
                            "punch-knocked-down"
                        } else {
                            "punch-stunned"
                        };
                        consequence.weaken_ds = counts * COUNT_DS;
                        visible.0.push((at, line(id), None));
                    }
                }
                _ => {}
            }
            body.take_damage(zone, damage, 0.0);
        } else if intent == Intent::Grab {
            commands.queue(move |world: &mut World| crate::grabs::start(world, attacker, target));
            continue;
        } else if intent == Intent::Disarm {
            if lying {
                continue;
            }
            let line = |id: &str| {
                LocMsg::new(id)
                    .arg("attacker", attacker_name.clone())
                    .arg("victim", victim.clone())
            };
            match rng.range(1, 100) {
                1..=25 => {
                    consequence.weaken_ds = 2.0 * COUNT_DS;
                    visible.0.push((at, line("pushed-down"), None));
                }
                26..=60 => {
                    commands.queue(move |world: &mut World| drop_held(world, target));
                    visible.0.push((at, line("disarmed"), None));
                }
                _ => visible.0.push((at, line("disarm-missed"), None)),
            }
        } else if intent == Intent::Help {
            if target == attacker {
                let found = check_self(&body);
                if let Some(outbox) = outbox.as_mut() {
                    outbox.0.extend(found);
                }
                let msg = LocMsg::new("checks-self").arg("name", victim);
                visible.0.push((at, msg, Some(target)));
            } else if body.health() > 0.0 {
                commands.queue(move |world: &mut World| shake_awake(world, target));
                let msg = LocMsg::new("shakes-awake")
                    .arg("attacker", attacker_name)
                    .arg("victim", victim);
                visible.0.push((at, msg, None));
            }
            continue;
        }
        if consequence.weaken_ds > 0.0 || consequence.paralyse_ds > 0.0 {
            commands.queue(move |world: &mut World| knock_down(world, target, consequence));
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmorKind {
    Bullet,
    Laser,
    Energy,
}

/// How much armour someone has on this part against this, % (Luna's `checkarmor`, which only counts
/// what's on the head, the face, the body and under it).
pub fn armour_on(world: &World, mob: Entity, zone: Zone, kind: ArmorKind) -> f32 {
    let Some(equipment) = world.get::<Equipment>(mob) else {
        return 0.0;
    };
    [Slot::Head, Slot::Mask, Slot::Suit, Slot::Uniform]
        .into_iter()
        .filter_map(|slot| equipment.get(slot))
        .filter_map(|item| world.get::<crate::equipment::Wearable>(item))
        .filter(|worn| worn.covers & zone.cover_bit() != 0)
        .map(|worn| {
            f32::from(match kind {
                ArmorKind::Bullet => worn.armor.bullet,
                ArmorKind::Laser => worn.armor.laser,
                ArmorKind::Energy => worn.armor.energy,
            })
        })
        .sum()
}

/// Two rolls against someone's armour (Luna `run_armor_check`): 0 is no help, 1 halves it, 2 stops it.
pub fn armour_check(world: &mut World, mob: Entity, zone: Zone, kind: ArmorKind) -> u8 {
    let armour = armour_on(world, mob, zone, kind);
    let absorbed = {
        let mut rng = world.resource_mut::<GameRng>();
        u8::from(rng.prob(armour)) + u8::from(rng.prob(armour))
    };
    let told = match absorbed {
        2 => Some("armor-absorbs"),
        1 => Some("armor-softens"),
        _ => None,
    };
    if let (Some(told), Some(mut outbox)) = (told, world.get_mut::<crate::actions::Outbox>(mob)) {
        outbox.0.push(LocMsg::new(told));
    }
    absorbed
}

/// Those knocked out come round.
pub fn come_round(time: Res<WorldTime>, out: Query<(Entity, &Paralyzed)>, mut commands: Commands) {
    let now = time.deciseconds();
    for (mob, paralyzed) in &out {
        if now >= paralyzed.0 {
            commands.entity(mob).remove::<Paralyzed>();
        }
    }
}

/// Show each event to every conscious mob that can see where it happened.
pub fn deliver_visible(
    mut visible: ResMut<VisibleEvents>,
    map: crate::decks::Decks,
    mut viewers: Query<
        (
            Entity,
            &GridPos,
            &mut Outbox,
            Option<&Vitals>,
            Has<rs13_world::Ghost>,
        ),
        Or<(With<Mob>, With<rs13_world::Ghost>)>,
    >,
    opaque: Query<&GridPos, (With<Opaque>, Without<Mob>)>,
) {
    if visible.0.is_empty() {
        return;
    }
    let opaque: Vec<TilePos> = opaque.iter().map(|p| p.0).collect();
    for (at, msg, except) in visible.0.drain(..) {
        for (viewer, pos, mut outbox, vitals, ghost) in &mut viewers {
            if except == Some(viewer) {
                continue;
            }
            let awake = vitals.is_none_or(|v| v.stat == MobStat::Conscious);
            let seen = if ghost {
                within_range(at, pos.0, HEARING_RANGE)
            } else {
                can_see(map.at(at.z), &opaque, at, pos.0, HEARING_RANGE)
            };
            if awake && seen {
                outbox.0.push(msg.clone());
            }
        }
    }
}
