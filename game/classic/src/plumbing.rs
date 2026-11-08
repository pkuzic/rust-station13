//! Sinks, showers, toilets and mirrors (Luna's `structures.dm` sink and toilet, `watercloset.dm`
//! shower, `structures/mirror.dm`). Spec: `docs/reference/plumbing.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, DisplayName, Facing, GridPos, NetId, TilePos};

use crate::actions::{Outbox, in_reach};
use crate::combat::VisibleEvents;
use crate::environment::{LIFE_TICKS, Lungs};
use crate::hands::{Hands, Item};
use crate::health::Body;
use crate::movement::Mob;
use crate::rng::GameRng;
use crate::sound::{GLASS_HIT, SHATTER, SHATTERED_HIT, SoundEvents};
use crate::time::WorldTime;

pub const SINK: &str = "sink";
pub const KITCHEN_SINK: &str = "kitchen-sink";
pub const TOILET: &str = "toilet";
pub const SHOWER: &str = "shower";
pub const MIRROR: &str = "mirror";
/// The falling water of a shower that's on, and the mist it makes.
pub const SHOWER_WATER: &str = "shower-water";
pub const MIST: &str = "mist";

/// Washing takes 4 s (Luna `sleep(40)`); turning a shower's valve 5 s (`do_after(user, 50)`).
pub const WASH_DS: f32 = 40.0;
pub const VALVE_DS: f32 = 50.0;
/// Water from a sink: a bucket gets 70, other glassware 10 (Luna `add_reagent("water", …)`).
pub const BUCKET_WATER: f32 = 70.0;
pub const GLASS_WATER: f32 = 10.0;
/// A shower mists up 5 s after it's turned on, and the mist lingers 25 s after it's off.
pub const MIST_AFTER_DS: f32 = 50.0;
pub const MIST_LINGERS_DS: f32 = 250.0;
/// Slammed into a toilet: 8 brute (Luna `adjustBruteLoss(8)`).
pub const SLAM_BRUTE: f32 = 8.0;

pub fn is_plumbing(proto: &str) -> bool {
    matches!(proto, SINK | KITCHEN_SINK | TOILET | SHOWER | MIRROR)
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Sink {
    /// Someone's washing here.
    pub busy: bool,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Toilet;

/// How hot a shower runs (Luna `watertemp`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WaterTemp {
    #[default]
    Normal,
    Freezing,
    Boiling,
}

impl WaterTemp {
    fn next(self) -> Self {
        match self {
            WaterTemp::Normal => WaterTemp::Freezing,
            WaterTemp::Freezing => WaterTemp::Boiling,
            WaterTemp::Boiling => WaterTemp::Normal,
        }
    }

    fn id(self) -> &'static str {
        match self {
            WaterTemp::Normal => "shower-temp-normal",
            WaterTemp::Freezing => "shower-temp-freezing",
            WaterTemp::Boiling => "shower-temp-boiling",
        }
    }
}

#[derive(Component, Debug, Clone, PartialEq, Default)]
pub struct Shower {
    pub on: bool,
    pub temp: WaterTemp,
    /// The water and mist shown, if any.
    pub water: Option<Entity>,
    pub mist: Option<Entity>,
    /// When the mist comes, or goes, ds.
    pub mist_at: Option<f32>,
    pub mist_gone_at: Option<f32>,
    /// Who stood under it last tick.
    pub under: Vec<Entity>,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Mirror {
    pub shattered: bool,
}

/// Washing at a sink: hands, or the item held. Done if they stay put.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Washing {
    pub sink: Entity,
    pub item: Option<Entity>,
    pub from: TilePos,
    pub done_at: f32,
}

/// Turning a shower's temperature valve with a wrench.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct TurningValve {
    pub shower: Entity,
    pub from: TilePos,
    pub done_at: f32,
}

pub fn spawn_plumbing(world: &mut World, proto: &str, pos: TilePos, id: NetId) -> Entity {
    let mut thing = world.spawn((GridPos(pos), id, Appearance::new(proto, "")));
    match proto {
        SINK | KITCHEN_SINK => {
            thing.insert(Sink::default());
        }
        TOILET => {
            thing.insert(Toilet);
        }
        SHOWER => {
            thing.insert(Shower::default());
        }
        _ => {
            thing.insert(Mirror::default());
        }
    }
    thing.id()
}

fn name(world: &World, e: Entity) -> String {
    world
        .get::<DisplayName>(e)
        .map_or_else(String::new, |n| n.0.clone())
}

fn sprite(world: &World, e: Entity) -> String {
    world
        .get::<Appearance>(e)
        .map_or_else(String::new, |a| a.sprite.clone())
}

fn tell(world: &mut World, mob: Entity, msg: LocMsg) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(msg);
    }
}

/// Seen by everyone around but `mob`, who is told `own` instead.
fn seen(world: &mut World, at: TilePos, msg: LocMsg, mob: Entity, own: Option<LocMsg>) {
    let except = own.is_some().then_some(mob);
    world.resource_mut::<VisibleEvents>().0.push((at, msg, except));
    if let Some(own) = own {
        tell(world, mob, own);
    }
}

fn at(world: &World, e: Entity) -> Option<TilePos> {
    world.get::<GridPos>(e).map(|p| p.0)
}

/// A click on a sink, shower, toilet or mirror in reach, with whatever is in the active hand.
pub fn click(world: &mut World, mob: Entity, target: Entity) {
    let (Some(here), Some(there)) = (at(world, mob), at(world, target)) else {
        return;
    };
    if !in_reach(here, there) {
        return;
    }
    let held = world.get::<Hands>(mob).and_then(Hands::active_item);
    if world.get::<Sink>(target).is_some() {
        use_sink(world, mob, target, held, here);
    } else if world.get::<Shower>(target).is_some() {
        use_shower(world, mob, target, held, here);
    } else if world.get::<Mirror>(target).is_some()
        && let Some(item) = held
    {
        hit_mirror(world, mob, target, item, there);
    }
}

/// Luna `sink/attack_hand` and `attackby`.
fn use_sink(world: &mut World, mob: Entity, sink: Entity, held: Option<Entity>, here: TilePos) {
    if world.get::<Sink>(sink).is_some_and(|s| s.busy) {
        tell(world, mob, LocMsg::new("sink-busy"));
        return;
    }
    let now = world.resource::<WorldTime>().deciseconds();
    let Some(item) = held else {
        tell(world, mob, LocMsg::new("sink-start-hands"));
        start_washing(world, mob, sink, None, here, now);
        return;
    };
    let proto = sprite(world, item);
    let water = if proto == "bucket" {
        Some(BUCKET_WATER)
    } else if crate::chemistry::is_glass(&proto) {
        Some(GLASS_WATER)
    } else {
        None
    };
    if let Some(amount) = water {
        if let Some(mut reagents) = world.get_mut::<crate::chemistry::Reagents>(item) {
            reagents.add("water", amount);
        }
        let msg = LocMsg::new("sink-fills")
            .arg("name", name(world, mob))
            .arg("item", proto.clone())
            .arg("sink", sprite(world, sink));
        let own = LocMsg::new("sink-fills-self")
            .arg("item", proto)
            .arg("sink", sprite(world, sink));
        seen(world, here, msg, mob, Some(own));
        return;
    }
    if world.get::<Item>(item).is_none() {
        return;
    }
    tell(world, mob, LocMsg::new("sink-start-item").arg("item", proto));
    start_washing(world, mob, sink, Some(item), here, now);
}

fn start_washing(
    world: &mut World,
    mob: Entity,
    sink: Entity,
    item: Option<Entity>,
    here: TilePos,
    now: f32,
) {
    if let Some(mut s) = world.get_mut::<Sink>(sink) {
        s.busy = true;
    }
    world.entity_mut(mob).insert(Washing {
        sink,
        item,
        from: here,
        done_at: now + WASH_DS,
    });
}

/// Luna `shower/attack_hand` and `attackby`: an empty hand turns it on or off, a wrench turns the
/// temperature valve, an analyzer reads it.
fn use_shower(world: &mut World, mob: Entity, shower: Entity, held: Option<Entity>, here: TilePos) {
    let now = world.resource::<WorldTime>().deciseconds();
    let Some(item) = held else {
        let Some(mut s) = world.get_mut::<Shower>(shower) else {
            return;
        };
        s.on = !s.on;
        let on = s.on;
        // Whoever's under it feels it at once.
        s.under.clear();
        if on {
            let ready = s.mist.is_some();
            s.mist_at = Some(if ready { now } else { now + MIST_AFTER_DS });
            s.mist_gone_at = None;
        } else {
            s.mist_at = None;
            s.mist_gone_at = s.mist.map(|_| now + MIST_LINGERS_DS);
        }
        return;
    };
    match sprite(world, item).as_str() {
        "atmos-analyzer" => {
            let temp = world.get::<Shower>(shower).map(|s| s.temp).unwrap_or_default();
            tell(world, mob, LocMsg::new("shower-reads").msg("temp", temp.id()));
        }
        "wrench" => {
            tell(world, mob, LocMsg::new("shower-valve-start"));
            world.entity_mut(mob).insert(TurningValve {
                shower,
                from: here,
                done_at: now + VALVE_DS,
            });
        }
        _ => {}
    }
}

/// Luna `mirror/attackby`: twice the item's force in percent to shatter it.
fn hit_mirror(world: &mut World, mob: Entity, mirror: Entity, item: Entity, there: TilePos) {
    if world.get::<Mirror>(mirror).is_some_and(|m| m.shattered) {
        world
            .resource_mut::<SoundEvents>()
            .play(SHATTERED_HIT, there, 70.0);
        return;
    }
    let force = world.get::<Item>(item).map_or(0.0, |i| i.force);
    let smash = world.resource_mut::<GameRng>().prob(force * 2.0);
    let id = if smash { "mirror-smashed" } else { "mirror-hit" };
    let msg = LocMsg::new(id)
        .arg("name", name(world, mob))
        .arg("item", sprite(world, item));
    world.resource_mut::<VisibleEvents>().0.push((there, msg, None));
    if smash {
        shatter(world, mirror);
    } else {
        world.resource_mut::<SoundEvents>().play(GLASS_HIT, there, 70.0);
    }
}

/// Luna `mirror/shatter`: broken for good. Shots and anything that breaks it call this.
pub fn shatter(world: &mut World, mirror: Entity) {
    let Some(mut m) = world.get_mut::<Mirror>(mirror) else {
        return;
    };
    if m.shattered {
        return;
    }
    m.shattered = true;
    world.entity_mut(mirror).insert(Appearance::new(MIRROR, "broken"));
    if let Some(there) = at(world, mirror) {
        world.resource_mut::<SoundEvents>().play(SHATTER, there, 70.0);
    }
}

/// A grab used on a toilet (Luna `toilet/attackby` with a grab): whoever's held, if held tightly
/// enough and on the toilet's tile, is slammed into it. Its lid never lifts, so it's never a
/// swirlie.
pub fn grab_on_toilet(world: &mut World, mob: Entity, victim: Entity, tight: bool, toilet: Entity) {
    let (Some(here), Some(there)) = (at(world, mob), at(world, toilet)) else {
        return;
    };
    if !in_reach(here, there) {
        return;
    }
    if !tight || at(world, victim) != Some(there) {
        tell(world, mob, LocMsg::new("toilet-tighter-grip"));
        return;
    }
    if let Some(mut body) = world.get_mut::<Body>(victim) {
        body.take_damage_spread(SLAM_BRUTE, 0.0);
    }
    let msg = LocMsg::new("toilet-slam")
        .arg("name", name(world, mob))
        .arg("victim", name(world, victim));
    let own = LocMsg::new("toilet-slam-self").arg("victim", name(world, victim));
    seen(world, there, msg, mob, Some(own));
}

/// Every tick: washing and valves finish, showers run.
pub fn run_plumbing(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    finish_washing(world, now);
    finish_valves(world, now);
    run_showers(world, now);
}

fn finish_washing(world: &mut World, now: f32) {
    let mut q = world.query::<(Entity, &Washing)>();
    let due: Vec<(Entity, Washing)> = q
        .iter(world)
        .filter(|(_, w)| now >= w.done_at)
        .map(|(e, w)| (e, *w))
        .collect();
    for (mob, wash) in due {
        world.entity_mut(mob).remove::<Washing>();
        if let Some(mut s) = world.get_mut::<Sink>(wash.sink) {
            s.busy = false;
        }
        // Moved away, or put the thing down: nothing.
        if at(world, mob) != Some(wash.from) {
            continue;
        }
        let held = world.get::<Hands>(mob).and_then(Hands::active_item);
        let sink = sprite(world, wash.sink);
        match wash.item {
            None => {
                let msg = LocMsg::new("sink-washed-hands")
                    .arg("name", name(world, mob))
                    .arg("sink", sink);
                seen(world, wash.from, msg, mob, None);
            }
            Some(item) if held == Some(item) => {
                let msg = LocMsg::new("sink-washed-item")
                    .arg("name", name(world, mob))
                    .arg("item", sprite(world, item))
                    .arg("sink", sink.clone());
                let own = LocMsg::new("sink-washed-item-self")
                    .arg("item", sprite(world, item))
                    .arg("sink", sink);
                seen(world, wash.from, msg, mob, Some(own));
            }
            Some(_) => {}
        }
    }
}

fn finish_valves(world: &mut World, now: f32) {
    let mut q = world.query::<(Entity, &TurningValve)>();
    let due: Vec<(Entity, TurningValve)> = q
        .iter(world)
        .filter(|(_, v)| now >= v.done_at)
        .map(|(e, v)| (e, *v))
        .collect();
    for (mob, valve) in due {
        world.entity_mut(mob).remove::<TurningValve>();
        let held = world.get::<Hands>(mob).and_then(Hands::active_item);
        if at(world, mob) != Some(valve.from) || held.is_none_or(|h| sprite(world, h) != "wrench") {
            continue;
        }
        if let Some(mut s) = world.get_mut::<Shower>(valve.shower) {
            s.temp = s.temp.next();
        }
        let msg = LocMsg::new("shower-adjusted").arg("name", name(world, mob));
        let own = LocMsg::new("shower-adjusted-self");
        seen(world, valve.from, msg, mob, Some(own));
    }
}

fn run_showers(world: &mut World, now: f32) {
    let life_tick = world.resource::<WorldTime>().tick.0.is_multiple_of(LIFE_TICKS);
    let mut q = world.query::<(Entity, &Shower, &GridPos, Option<&Facing>)>();
    let showers: Vec<(Entity, Shower, TilePos, Option<Facing>)> = q
        .iter(world)
        .map(|(e, s, p, f)| (e, s.clone(), p.0, f.copied()))
        .collect();
    if showers.is_empty() {
        return;
    }
    let mut mobs = world.query_filtered::<(Entity, &GridPos), (With<Mob>, With<Lungs>)>();
    let people: Vec<(Entity, TilePos)> = mobs.iter(world).map(|(e, p)| (e, p.0)).collect();
    for (shower, mut s, pos, facing) in showers {
        // The falling water, while it's on.
        match (s.on, s.water) {
            (true, None) => s.water = Some(effect(world, SHOWER_WATER, pos, facing)),
            (false, Some(water)) => {
                world.entity_mut(water).despawn();
                s.water = None;
            }
            _ => {}
        }
        // The mist, unless the water's freezing.
        if s.on && s.temp == WaterTemp::Freezing {
            if let Some(mist) = s.mist.take() {
                world.entity_mut(mist).despawn();
            }
        } else if s.mist_at.is_some_and(|t| now >= t) {
            s.mist_at = None;
            if s.mist.is_none() {
                s.mist = Some(effect(world, MIST, pos, None));
            }
        }
        if s.mist_gone_at.is_some_and(|t| now >= t) {
            s.mist_gone_at = None;
            if let Some(mist) = s.mist.take() {
                world.entity_mut(mist).despawn();
            }
        }
        // Who's under it: a newcomer feels it at once, everyone every 2 s.
        let under: Vec<Entity> = people
            .iter()
            .filter(|(_, p)| *p == pos)
            .map(|(e, _)| *e)
            .collect();
        if s.on {
            for mob in &under {
                if life_tick || !s.under.contains(mob) {
                    check_heat(world, *mob, s.temp);
                }
            }
        }
        s.under = under;
        if let Ok(mut e) = world.get_entity_mut(shower) {
            e.insert(s);
        }
    }
}

fn effect(world: &mut World, proto: &str, pos: TilePos, facing: Option<Facing>) -> Entity {
    let id = world.resource_mut::<crate::objects::NetIds>().alloc();
    let mut e = world.spawn((GridPos(pos), id, Appearance::new(proto, "")));
    if let Some(facing) = facing {
        e.insert(facing);
    }
    e.id()
}

/// Luna `shower/check_heat`: freezing water chills by 80 K (to 80 K at least), boiling water
/// warms by 35 K (to 500 K at most) and burns.
fn check_heat(world: &mut World, mob: Entity, temp: WaterTemp) {
    let id = match temp {
        WaterTemp::Normal => return,
        WaterTemp::Freezing => {
            if let Some(mut lungs) = world.get_mut::<Lungs>(mob) {
                lungs.body_temperature = (lungs.body_temperature - 80.0).max(80.0);
            }
            "shower-freezing"
        }
        WaterTemp::Boiling => {
            if let Some(mut lungs) = world.get_mut::<Lungs>(mob) {
                lungs.body_temperature = (lungs.body_temperature + 35.0).min(500.0);
            }
            if let Some(mut body) = world.get_mut::<Body>(mob) {
                body.take_damage_spread(0.0, 5.0);
            }
            "shower-searing"
        }
    };
    tell(world, mob, LocMsg::new(id));
}
