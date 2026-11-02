//! Microwaves (Luna's `kitchen/microwave`): put food in, start it, and take out what comes out
//! when it dings. Only warming Donk-pockets works so far, as recipes need ingredients and
//! chemistry we don't have. Spec: `docs/reference/microwaves.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, Dense, GridPos, NetId, ObjectSpec, TilePos};

use crate::actions::Outbox;
use crate::closets::InCloset;
use crate::combat::VisibleEvents;
use crate::food::Snack;
use crate::hands::{Hands, HeldBy};
use crate::health::{Body, Zone};
use crate::power::{Channel, PowerUser};
use crate::sound::{DING, SoundEvents};
use crate::time::WorldTime;

pub const MICROWAVE: &str = "microwave";
pub const DONK_POCKET: &str = "donk-pocket";
pub const BURNT_MESS: &str = "burnt-mess";
/// How long a run takes, ds (Luna's recipe default and `wzhzhzh(10)`).
pub const COOK_DS: f32 = 100.0;
/// What it draws while running, W (Luna `use_power(500)` each second).
pub const COOK_WATTS: f64 = 500.0;
/// What it draws idle, W.
pub const IDLE_WATTS: f64 = 5.0;
/// The most it holds (Luna's largest recipe).
pub const MAX_ITEMS: usize = 6;
/// How long a warmed Donk-pocket stays warm, ds (Luna `cooltime`).
pub const WARM_DS: f32 = 4200.0;
/// How many bites a warm Donk-pocket is at most (Luna's `bitesize = 6` of its 12 units).
pub const WARM_BITES: u8 = 2;

#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct Microwave {
    pub contents: Vec<Entity>,
    pub done_at: Option<f32>,
}

/// A warm Donk-pocket: until this time, ds. Each bite heals a little.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Warm(pub f32);

pub fn spawn_microwave(world: &mut World, _spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(MICROWAVE, "mw"),
            Dense,
            Microwave::default(),
            PowerUser::new(Channel::Equipment, IDLE_WATTS),
        ))
        .id()
}

/// A bite of a warm Donk-pocket: half of what its tricordrazine (Luna: 5 units at 1 of each
/// damage a unit) heals in all.
pub fn warm_bite(body: &mut Body) {
    const HEAL: f32 = 2.5;
    body.oxy = (body.oxy - HEAL).max(0.0);
    body.tox = (body.tox - HEAL).max(0.0);
    for zone in Zone::ALL {
        body.heal(zone, HEAL / Zone::ALL.len() as f32, HEAL / Zone::ALL.len() as f32);
    }
}

fn tell(world: &mut World, mob: Entity, id: &str) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(LocMsg::new(id));
    }
}

/// A click on a microwave: with food in hand it goes in, and with an empty hand it starts
/// (Luna's window's Cook button).
pub fn click(world: &mut World, mob: Entity, oven: Entity) {
    let Some(state) = world.get::<Microwave>(oven) else {
        return;
    };
    if state.done_at.is_some() {
        return;
    }
    let held = world.get::<Hands>(mob).and_then(Hands::active_item);
    if let Some(item) = held {
        let sprite = world.get::<Appearance>(item).map(|a| a.sprite.clone());
        if sprite.as_deref() != Some(DONK_POCKET) {
            tell(world, mob, "microwave-unsuitable");
            return;
        }
        if state.contents.len() >= MAX_ITEMS {
            tell(world, mob, "microwave-full");
            return;
        }
        if let Some(mut hands) = world.get_mut::<Hands>(mob) {
            let active = hands.active;
            *hands.slot_mut(active) = None;
        }
        world.entity_mut(item).remove::<HeldBy>().insert(InCloset(oven));
        if let Some(mut state) = world.get_mut::<Microwave>(oven) {
            state.contents.push(item);
        }
        tell(world, mob, "microwave-added");
        return;
    }
    if !world.get::<PowerUser>(oven).is_some_and(|p| p.powered) {
        return;
    }
    let now = world.resource::<WorldTime>().deciseconds();
    if let Some(mut state) = world.get_mut::<Microwave>(oven) {
        state.done_at = Some(now + COOK_DS);
    }
    if let Some(mut look) = world.get_mut::<Appearance>(oven) {
        look.state = "mw1".to_owned();
    }
    if let Some(mut power) = world.get_mut::<PowerUser>(oven) {
        power.watts = COOK_WATTS;
    }
    if let Some(at) = world.get::<GridPos>(oven).map(|p| p.0) {
        world
            .resource_mut::<VisibleEvents>()
            .0
            .push((at, LocMsg::new("microwave-on"), None));
    }
}

fn stop(world: &mut World, oven: Entity) {
    if let Some(mut state) = world.get_mut::<Microwave>(oven) {
        state.done_at = None;
    }
    if let Some(mut look) = world.get_mut::<Appearance>(oven) {
        look.state = "mw".to_owned();
    }
    if let Some(mut power) = world.get_mut::<PowerUser>(oven) {
        power.watts = IDLE_WATTS;
    }
}

/// Finish what's done, stop for a power cut, and let Donk-pockets go cold.
pub fn run_microwaves(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    let mut ovens = world.query::<(Entity, &Microwave, &PowerUser, &GridPos)>();
    let running: Vec<(Entity, f32, bool, TilePos)> = ovens
        .iter(world)
        .filter_map(|(e, m, p, at)| m.done_at.map(|t| (e, t, p.powered, at.0)))
        .collect();
    for (oven, done_at, powered, at) in running {
        if !powered {
            // Luna `abort`: everything stays in.
            stop(world, oven);
        } else if now >= done_at {
            finish(world, oven, at, now);
        }
    }
    let mut warm = world.query::<(Entity, &Warm)>();
    let cold: Vec<Entity> = warm
        .iter(world)
        .filter(|(_, w)| now >= w.0)
        .map(|(e, _)| e)
        .collect();
    for pocket in cold {
        world.entity_mut(pocket).remove::<Warm>();
    }
}

/// One Donk-pocket warms up. Anything else burns to one mess (Luna `fail`), and an empty one
/// has just run.
fn finish(world: &mut World, oven: Entity, at: TilePos, now: f32) {
    let contents = world
        .get_mut::<Microwave>(oven)
        .map(|mut m| std::mem::take(&mut m.contents))
        .unwrap_or_default();
    stop(world, oven);
    world.resource_mut::<SoundEvents>().play(DING, at, 50.0);
    match contents.as_slice() {
        [] => {}
        [pocket] => {
            let warm = Warm(now + WARM_DS);
            let mut item = world.entity_mut(*pocket);
            item.remove::<InCloset>().insert((GridPos(at), warm));
            if let Some(mut snack) = item.get_mut::<Snack>() {
                snack.bites_left = snack.bites_left.min(WARM_BITES);
            }
        }
        many => {
            for thing in many {
                world.entity_mut(*thing).despawn();
            }
            let _ = crate::objects::spawn_object(world, &ObjectSpec::new(BURNT_MESS), at);
        }
    }
}
