//! The emag (Luna's `card/emag`, the cryptographic sequencer): a traitor's card that breaks locks.
//! Spec: `docs/reference/emag.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{DisplayName, GridPos, TilePos};

use crate::actions::Outbox;
use crate::closets::{Closet, ClosetLock};
use crate::combat::VisibleEvents;
use crate::doors::Airlock;
use crate::power::Apc;
use crate::rng::GameRng;
use crate::sound::SoundEvents;
use crate::time::WorldTime;
use crate::vending::Vending;
use crate::windoors::Windoor;

pub const EMAG: &str = "emag";
/// The chance an APC's interface gives way, percent (Luna `prob(50)`).
pub const APC_CHANCE: f32 = 50.0;

fn tell(world: &mut World, mob: Entity, msg: LocMsg) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(msg);
    }
}

fn at(world: &World, e: Entity) -> Option<TilePos> {
    world.get::<GridPos>(e).map(|p| p.0)
}

fn sparks(world: &mut World, target: Entity) {
    if let Some(pos) = at(world, target) {
        world
            .resource_mut::<SoundEvents>()
            .play(crate::sound::SPARKS, pos, 60.0);
    }
}

/// The emag swiped on something within reach (Luna's `attackby` with `card/emag`). Anything it
/// doesn't work on is left alone.
pub fn use_on(world: &mut World, mob: Entity, target: Entity) {
    let now = world.resource::<WorldTime>().deciseconds();
    if let Some(mut door) = world.get_mut::<Airlock>(target) {
        if door.emag(now) {
            sparks(world, target);
        }
        return;
    }
    if let Some(mut door) = world.get_mut::<Windoor>(target) {
        if door.emag(now) {
            sparks(world, target);
        }
        return;
    }
    if world.get::<ClosetLock>(target).is_some() {
        return closet(world, mob, target);
    }
    if world.get::<Apc>(target).is_some() {
        return apc(world, mob, target);
    }
    if world.get::<Vending>(target).is_some() {
        crate::hacking::emag_vendor(world, target);
        let sprite = world
            .get::<rs13_world::Appearance>(target)
            .map(|a| a.sprite.clone())
            .unwrap_or_default();
        tell(world, mob, LocMsg::new("emag-vending").arg("machine", sprite));
    }
}

/// Luna `secure_closet` and `crate/secure` `attackby`: a shut, unbroken lock is broken open.
fn closet(world: &mut World, mob: Entity, target: Entity) {
    let Some(closet) = world.get::<Closet>(target) else {
        return;
    };
    if closet.opened {
        return;
    }
    let is_crate = closet.is_crate;
    let Some(mut lock) = world.get_mut::<ClosetLock>(target) else {
        return;
    };
    if lock.broken {
        return tell(world, mob, LocMsg::new("locker-broken"));
    }
    lock.broken = true;
    lock.locked = false;
    sparks(world, target);
    let sprite = world
        .get::<rs13_world::Appearance>(target)
        .map(|a| a.sprite.clone())
        .unwrap_or_default();
    if is_crate {
        tell(world, mob, LocMsg::new("emag-crate").arg("item", sprite));
    } else if let Some(pos) = at(world, target) {
        let name = world
            .get::<DisplayName>(mob)
            .map(|n| n.0.clone())
            .unwrap_or_default();
        let msg = LocMsg::new("emag-locker").arg("name", name);
        world.resource_mut::<VisibleEvents>().0.push((pos, msg, None));
    }
}

/// Luna `apc/attackby` with an emag: with the cover and panel shut, half the time the interface
/// breaks unlocked for good.
fn apc(world: &mut World, mob: Entity, target: Entity) {
    let exposed = crate::hacking::wires(world, target).is_some_and(|w| w.panel_open);
    let Some(apc) = world.get::<Apc>(target) else {
        return;
    };
    if apc.emagged || apc.broken {
        return;
    }
    if apc.opened {
        return tell(world, mob, LocMsg::new("apc-close-cover-first"));
    }
    if exposed {
        return tell(world, mob, LocMsg::new("apc-close-panel-first"));
    }
    let locked = apc.locked;
    sparks(world, target);
    if world.resource_mut::<GameRng>().prob(APC_CHANCE) {
        if let Some(mut apc) = world.get_mut::<Apc>(target) {
            apc.emagged = true;
            apc.locked = false;
        }
        tell(world, mob, LocMsg::new("emag-apc"));
    } else {
        let id = if locked {
            "emag-apc-unlock-failed"
        } else {
            "emag-apc-lock-failed"
        };
        tell(world, mob, LocMsg::new(id));
    }
}
