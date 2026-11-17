//! The laundromat (Luna's `machinery/washer.dm`): clothes in, cleaning solution poured in from a
//! beaker, and a wash that takes the fingerprints and fibres off everything inside before it all
//! comes back out. Spec: `docs/reference/laundry.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, GridPos, Slot, TilePos};

use crate::actions::Outbox;
use crate::chemistry::Reagents;
use crate::combat::VisibleEvents;
use crate::equipment::Wearable;
use crate::hands::{Hands, HeldBy};
use crate::power::{Channel, PowerUser};
use crate::time::WorldTime;

pub const WASHER: &str = "washer";
/// How long a wash takes, ds (Luna `sleep(51)`).
pub const WASH_DS: f32 = 51.0;
/// Cleaning solution a wash uses, units.
pub const PER_WASH: f32 = 1.0;
const CLEANER: &str = "cleaner";

/// A washer: cleaning solution in it, what's inside, and when the wash running now ends.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct Washer {
    pub cleaner: f32,
    pub contents: Vec<Entity>,
    pub done_at: Option<f32>,
}

/// In a washer (it is off the map).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InWasher(pub Entity);

/// Give a new washer its parts. Luna sets no power draw, but it won't start unpowered.
pub fn equip_item(world: &mut World, thing: Entity, proto: &str) {
    if proto == WASHER {
        world
            .entity_mut(thing)
            .insert((Washer::default(), PowerUser::new(Channel::Equipment, 0.0)));
    }
}

fn tell(world: &mut World, mob: Entity, msg: LocMsg) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(msg);
    }
}

fn sprite(world: &World, e: Entity) -> Option<String> {
    world.get::<Appearance>(e).map(|a| a.sprite.clone())
}

fn at(world: &World, e: Entity) -> Option<TilePos> {
    world.get::<GridPos>(e).map(|p| p.0)
}

fn now(world: &World) -> f32 {
    world.get_resource::<WorldTime>().map_or(0.0, |t| t.deciseconds())
}

/// Clothing and backpacks go in (Luna `istype(W, /obj/item/clothing/)` or a backpack): worn
/// things other than belts, cards and headsets, or a bag worn on the back.
fn washable(world: &World, item: Entity) -> bool {
    let Some(wear) = world.get::<Wearable>(item) else {
        return false;
    };
    match wear.slot {
        Slot::Uniform | Slot::Suit | Slot::Head | Slot::Mask | Slot::Gloves | Slot::Shoes | Slot::Glasses => {
            true
        }
        Slot::Ears => world.get::<crate::radio::Radio>(item).is_none(),
        Slot::Back => world.get::<crate::storage::Storage>(item).is_some(),
        _ => false,
    }
}

fn running(world: &World, washer: Entity) -> bool {
    world.get::<Washer>(washer).is_some_and(|w| w.done_at.is_some())
}

/// Luna `washer/attackby` and `attack_hand`. The caller checks reach.
pub fn use_on(world: &mut World, mob: Entity, held: Option<Entity>, washer: Entity) {
    let Some(held) = held else {
        start(world, mob, washer);
        return;
    };
    if running(world, washer) {
        tell(world, mob, LocMsg::new("washer-busy"));
        return;
    }
    let glass = sprite(world, held).is_some_and(|s| crate::chemistry::is_glass(&s));
    let cleaner = world.get::<Reagents>(held).map_or(0.0, |r| r.get(CLEANER));
    if glass && cleaner >= 1.0 {
        // All the cleaning solution in it goes in.
        if let Some(mut r) = world.get_mut::<Reagents>(held) {
            r.remove(CLEANER, cleaner);
        }
        if let Some(mut w) = world.get_mut::<Washer>(washer) {
            w.cleaner += cleaner;
        }
        tell(
            world,
            mob,
            LocMsg::new("washer-poured").num("value", cleaner.round() as i64),
        );
        return;
    }
    let item = sprite(world, held).unwrap_or_default();
    if !washable(world, held) {
        tell(world, mob, LocMsg::new("washer-beeps").arg("item", item));
        return;
    }
    if let Some(mut hands) = world.get_mut::<Hands>(mob) {
        for hand in [hands.active, hands.active.other()] {
            if hands.get(hand) == Some(held) {
                *hands.slot_mut(hand) = None;
            }
        }
    }
    world
        .entity_mut(held)
        .remove::<(HeldBy, GridPos)>()
        .insert(InWasher(washer));
    if let Some(mut w) = world.get_mut::<Washer>(washer) {
        w.contents.push(held);
    }
    if let Some(pos) = at(world, washer) {
        let name = world
            .get::<rs13_world::DisplayName>(mob)
            .map(|n| n.0.clone())
            .unwrap_or_default();
        if let Some(mut seen) = world.get_resource_mut::<VisibleEvents>() {
            seen.0.push((
                pos,
                LocMsg::new("washer-adds").arg("name", name).arg("item", item),
                None,
            ));
        }
    }
}

/// An empty hand starts a wash, powered and with cleaning solution in it (Luna `attack_hand`,
/// `wash`).
fn start(world: &mut World, mob: Entity, washer: Entity) {
    if !world.get::<PowerUser>(washer).is_some_and(|p| p.powered) || running(world, washer) {
        return;
    }
    if world.get::<Washer>(washer).is_none_or(|w| w.cleaner < PER_WASH) {
        tell(world, mob, LocMsg::new("washer-no-cleaner"));
        return;
    }
    let done = now(world) + WASH_DS;
    if let Some(mut w) = world.get_mut::<Washer>(washer) {
        w.done_at = Some(done);
    }
    if let Some(mut look) = world.get_mut::<Appearance>(washer) {
        look.state = "landanim".to_owned();
    }
    if let Some(pos) = at(world, washer)
        && let Some(mut sounds) = world.get_resource_mut::<crate::sound::SoundEvents>()
    {
        sounds.play(crate::sound::WASHING_MACHINE, pos, 100.0);
    }
}

/// Luna's Empty verb: everything inside comes out, unless it's washing.
pub fn empty(world: &mut World, mob: Entity, washer: Entity) {
    if running(world, washer) {
        tell(world, mob, LocMsg::new("washer-locked"));
        return;
    }
    let_out(world, washer);
}

/// Everything inside out onto the floor, as when it is destroyed.
pub fn empty_out(world: &mut World, washer: Entity) {
    let_out(world, washer);
}

fn let_out(world: &mut World, washer: Entity) {
    let Some(pos) = at(world, washer) else { return };
    let contents = world
        .get_mut::<Washer>(washer)
        .map(|mut w| std::mem::take(&mut w.contents))
        .unwrap_or_default();
    for item in contents {
        if world.get_entity(item).is_ok() {
            world.entity_mut(item).remove::<InWasher>().insert(GridPos(pos));
        }
    }
}

/// A wash ends: fingerprints and fibres come off everything inside, it all comes out, and a unit
/// of cleaning solution is used.
pub fn run_washers(world: &mut World) {
    let t = now(world);
    let mut q = world.query::<(Entity, &Washer)>();
    let done: Vec<Entity> = q
        .iter(world)
        .filter(|(_, w)| w.done_at.is_some_and(|d| t >= d))
        .map(|(e, _)| e)
        .collect();
    for washer in done {
        let contents = world
            .get::<Washer>(washer)
            .map(|w| w.contents.clone())
            .unwrap_or_default();
        for item in contents {
            if let Some(mut traces) = world.get_mut::<crate::forensics::Traces>(item) {
                *traces = crate::forensics::Traces::default();
            }
        }
        let_out(world, washer);
        if let Some(mut w) = world.get_mut::<Washer>(washer) {
            w.done_at = None;
            w.cleaner = (w.cleaner - PER_WASH).max(0.0);
        }
        if let Some(mut look) = world.get_mut::<Appearance>(washer) {
            look.state = String::new();
        }
    }
}

/// Examining it tells how much cleaning solution is left (Luna `washer/examine`).
pub fn examine(world: &World, washer: Entity) -> Option<LocMsg> {
    let w = world.get::<Washer>(washer)?;
    Some(LocMsg::new("examine-washer").num("value", w.cleaner.round() as i64))
}
