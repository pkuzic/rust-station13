//! Grabbing someone with an empty hand, on grab intent: first by the clothes, then with both hands,
//! then by the neck, and at last choking them (Luna's `obj/item/weapon/grab`). Spec:
//! `docs/reference/combat.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, DisplayName, GridPos, MobStat, ObjectSpec, Slot, TilePos};

use crate::actions::{Outbox, in_reach};
use crate::combat::{COUNT_DS, HELMETS, VisibleEvents};
use crate::environment::Lungs;
use crate::equipment::Equipment;
use crate::flashes::Weakened;
use crate::hands::{Hands, HeldBy};
use crate::health::Vitals;
use crate::movement::{Mob, MoveIntent};
use crate::objects::spawn_object;
use crate::pulling::Pulling;
use crate::rng::GameRng;
use crate::time::WorldTime;

pub const GRAB: &str = "grab";
/// How often a choke takes its toll, and an attempt to struggle comes, ds.
const CHOKE_EVERY_DS: f32 = 10.0;
const STRUGGLE_DS: f32 = 10.0;
/// The chance of breaking free with each struggle (Luna's 25 and 5).
const BREAK_GRIP: f32 = 25.0;
const BREAK_HEADLOCK: f32 = 5.0;
/// The chance of a grip tightening to both hands (Luna's 75).
const TIGHTEN: f32 = 75.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Hold {
    Passive,
    Aggressive,
    Neck,
}

/// What a grab, in someone's hand, has hold of.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Grab {
    pub assailant: Entity,
    pub victim: Entity,
    pub hold: Hold,
    pub choking: bool,
    /// When it next takes its toll, ds.
    next_choke: f32,
}

fn look(hold: Hold) -> &'static str {
    match hold {
        Hold::Passive => "grabbed",
        Hold::Aggressive => "grabbed1",
        Hold::Neck => "grabbed+1",
    }
}

fn seen(world: &mut World, at: TilePos, id: &str, assailant: Entity, victim: Entity) {
    let name = |e: Entity| {
        world
            .get::<DisplayName>(e)
            .map_or_else(String::new, |n| n.0.clone())
    };
    let msg = LocMsg::new(id)
        .arg("attacker", name(assailant))
        .arg("victim", name(victim));
    world.resource_mut::<VisibleEvents>().0.push((at, msg, None));
}

fn release(world: &mut World, grab: Entity) {
    let Some(held) = world.get::<Grab>(grab).copied() else {
        return;
    };
    if world
        .get::<Pulling>(held.assailant)
        .is_some_and(|p| p.0 == held.victim)
    {
        world.entity_mut(held.assailant).remove::<Pulling>();
    }
    if let Some(mut hands) = world.get_mut::<Hands>(held.assailant) {
        if hands.left == Some(grab) {
            hands.left = None;
        }
        if hands.right == Some(grab) {
            hands.right = None;
        }
    }
    world.entity_mut(grab).despawn();
}

/// Take hold of someone with the active hand (Luna's `attack_hand` on grab intent).
pub fn start(world: &mut World, assailant: Entity, victim: Entity) {
    if assailant == victim || world.get::<crate::chairs::Buckled>(victim).is_some() {
        return;
    }
    let Some(at) = world.get::<GridPos>(assailant).map(|p| p.0) else {
        return;
    };
    if world
        .get::<Hands>(assailant)
        .is_none_or(|h| h.active_item().is_some())
    {
        return;
    }
    let Ok(grab) = spawn_object(world, &ObjectSpec::new(GRAB), at) else {
        return;
    };
    world.entity_mut(grab).remove::<GridPos>().insert((
        HeldBy(assailant),
        Appearance::new(GRAB, look(Hold::Passive)),
        Grab {
            assailant,
            victim,
            hold: Hold::Passive,
            choking: false,
            next_choke: 0.0,
        },
    ));
    if let Some(mut hands) = world.get_mut::<Hands>(assailant) {
        let active = hands.active;
        *hands.slot_mut(active) = Some(grab);
    }
    seen(world, at, "grab-passive", assailant, victim);
}

/// What covers the neck: it has to come off first (Luna checks for anything over the head).
fn in_the_way(world: &World, victim: Entity) -> Option<String> {
    let on = |slot: Slot| {
        world
            .get::<Equipment>(victim)
            .and_then(|e| e.get(slot))
            .and_then(|item| world.get::<Appearance>(item))
            .map(|look| look.sprite.clone())
    };
    let hat = on(Slot::Head).filter(|h| HELMETS.contains(&h.as_str()));
    hat.or_else(|| on(Slot::Mask).filter(|m| m == "gas-mask"))
}

/// Using a grab, on who it holds or by itself, tightens it. On a table it puts them on the table
/// (Luna's `s_dbclick`, `s_click`, `table/attackby`).
pub fn use_grab(world: &mut World, mob: Entity, grab: Entity, target: Entity) {
    let Some(held) = world.get::<Grab>(grab).copied() else {
        return;
    };
    let Some(at) = world.get::<GridPos>(mob).map(|p| p.0) else {
        return;
    };
    let on_table = world
        .get::<Appearance>(target)
        .is_some_and(|look| matches!(look.sprite.as_str(), "table" | "table-reinforced" | "table-wood"));
    if on_table {
        let Some(table) = world
            .get::<GridPos>(target)
            .copied()
            .filter(|p| in_reach(at, p.0))
        else {
            return;
        };
        let now = world.resource::<WorldTime>().deciseconds();
        world
            .entity_mut(held.victim)
            .insert((table, Weakened(now + 5.0 * COUNT_DS)));
        seen(world, table.0, "grab-on-table", mob, held.victim);
        release(world, grab);
        return;
    }
    if target != held.victim && target != grab {
        return;
    }
    match held.hold {
        Hold::Passive => {
            // Not with the other hand holding someone else (Luna `allow_upgrade`).
            let busy = world.get::<Hands>(mob).is_some_and(|hands| {
                [hands.left, hands.right]
                    .into_iter()
                    .flatten()
                    .filter(|e| *e != grab)
                    .any(|e| world.get::<Grab>(e).is_some_and(|g| g.victim != held.victim))
            });
            if busy {
                return;
            }
            if world.resource_mut::<GameRng>().prob(TIGHTEN) {
                set_hold(world, grab, Hold::Aggressive);
                seen(world, at, "grab-aggressive", mob, held.victim);
            } else {
                seen(world, at, "grab-failed", mob, held.victim);
                release(world, grab);
            }
        }
        Hold::Aggressive => {
            if let Some(item) = in_the_way(world, held.victim) {
                if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
                    outbox.0.push(LocMsg::new("grab-take-off").arg("item", item));
                }
                return;
            }
            set_hold(world, grab, Hold::Neck);
            seen(world, at, "grab-neck", mob, held.victim);
        }
        Hold::Neck => {
            let choking = !held.choking;
            if let Some(mut grabbing) = world.get_mut::<Grab>(grab) {
                grabbing.choking = choking;
            }
            let id = if choking { "grab-choke" } else { "grab-loosen" };
            seen(world, at, id, mob, held.victim);
        }
    }
}

fn set_hold(world: &mut World, grab: Entity, hold: Hold) {
    if let Some(mut grabbing) = world.get_mut::<Grab>(grab) {
        grabbing.hold = hold;
    }
    world.entity_mut(grab).insert(Appearance::new(GRAB, look(hold)));
}

/// Someone held trying to walk struggles instead, every so often, and may break free. Walking off
/// from a passive hold just ends it (Luna's `client/Move`). Before movement, so a failed struggle
/// keeps them there.
pub fn struggle(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    let mut grabs = world.query::<(Entity, &Grab)>();
    let all: Vec<(Entity, Grab)> = grabs.iter(world).map(|(e, g)| (e, *g)).collect();
    for (grab, held) in all {
        let wants_to_go = world
            .get::<MoveIntent>(held.victim)
            .is_some_and(|i| i.dir.is_some());
        let awake = world
            .get::<Vitals>(held.victim)
            .is_none_or(|v| v.stat == MobStat::Conscious);
        let down = world.get::<Weakened>(held.victim).is_some();
        if !wants_to_go || !awake || down {
            continue;
        }
        let chance = match held.hold {
            Hold::Passive => {
                release(world, grab);
                continue;
            }
            Hold::Aggressive => BREAK_GRIP,
            Hold::Neck => BREAK_HEADLOCK,
        };
        let Some(mut mob) = world.get_mut::<Mob>(held.victim) else {
            continue;
        };
        if now < mob.next_move_ds {
            continue;
        }
        mob.next_move_ds = now + STRUGGLE_DS;
        if world.resource_mut::<GameRng>().prob(chance) {
            let at = world.get::<GridPos>(held.victim).map(|p| p.0);
            if let Some(at) = at {
                let id = if held.hold == Hold::Neck {
                    "grab-broke-headlock"
                } else {
                    "grab-broke-free"
                };
                seen(world, at, id, held.assailant, held.victim);
            }
            release(world, grab);
        }
    }
}

/// What holds do each tick, and letting go of the ones that are over.
pub fn run_grabs(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    let mut grabs = world.query::<(Entity, &Grab, Option<&HeldBy>)>();
    let all: Vec<(Entity, Grab, bool)> = grabs
        .iter(world)
        .map(|(e, g, h)| (e, *g, h.is_some_and(|h| h.0 == g.assailant)))
        .collect();
    for (grab, held, in_hand) in all {
        let place = |e: Entity| world.get::<GridPos>(e).map(|p| p.0);
        let near =
            matches!((place(held.assailant), place(held.victim)), (Some(a), Some(b)) if in_reach(a, b));
        let awake = world
            .get::<Vitals>(held.assailant)
            .is_none_or(|v| v.stat == MobStat::Conscious)
            && world.get::<Weakened>(held.assailant).is_none();
        if !in_hand || !near || !awake {
            release(world, grab);
            continue;
        }
        match held.hold {
            // It drags them along as a pull does.
            Hold::Passive => {
                world.entity_mut(held.assailant).insert(Pulling(held.victim));
            }
            // Both hands on them: they can't hold on to anything.
            Hold::Aggressive => {
                world.entity_mut(held.assailant).insert(Pulling(held.victim));
                empty_hands(world, held.victim);
            }
            Hold::Neck => {
                if world
                    .get::<Pulling>(held.assailant)
                    .is_some_and(|p| p.0 == held.victim)
                {
                    world.entity_mut(held.assailant).remove::<Pulling>();
                }
                empty_hands(world, held.victim);
                if let Some(at) = world.get::<GridPos>(held.assailant).copied()
                    && world.get::<crate::chairs::Buckled>(held.victim).is_none()
                {
                    world.entity_mut(held.victim).insert(at);
                }
                if held.choking && now >= held.next_choke {
                    choke(world, grab, held.victim, now);
                }
            }
        }
    }
}

fn empty_hands(world: &mut World, victim: Entity) {
    let Some(at) = world.get::<GridPos>(victim).copied() else {
        return;
    };
    let items: Vec<Entity> = world.get_mut::<Hands>(victim).map_or_else(Vec::new, |mut hands| {
        [hands.left.take(), hands.right.take()]
            .into_iter()
            .flatten()
            .collect()
    });
    for item in items {
        if world.get::<Grab>(item).is_some() {
            release_from_victim(world, item);
        } else {
            world.entity_mut(item).remove::<HeldBy>().insert(at);
        }
    }
}

/// A grab the victim was holding themselves goes when their hands are forced open.
fn release_from_victim(world: &mut World, grab: Entity) {
    release(world, grab);
}

fn choke(world: &mut World, grab: Entity, victim: Entity, now: f32) {
    if let Some(mut grabbing) = world.get_mut::<Grab>(grab) {
        grabbing.next_choke = now + CHOKE_EVERY_DS;
    }
    if world.resource_mut::<GameRng>().prob(45.0) {
        let until = now + 3.0 * COUNT_DS;
        if world.get::<Weakened>(victim).is_none_or(|w| w.0 < until) {
            world.entity_mut(victim).insert(Weakened(until));
        }
    }
    if let Some(mut lungs) = world.get_mut::<Lungs>(victim) {
        lungs.losebreath = (lungs.losebreath + 2).min(3);
    }
}
