//! Devices: the remote signaler, timer, proximity sensor and igniter, the two-part assemblies made
//! of them, tank bombs, and the tank transfer valve (Luna's `radio/signaler`, `devices/timer.dm`,
//! `proxy_sensor.dm`, `igniter.dm`, `obj/assemblies.dm`, `tank.dm`, `transfer_valve`). Spec:
//! `docs/reference/devices.md`.
//!
//! - **Signalers** send on a frequency and code; every other signaler on both gets it, wherever it
//!   is.
//! - **Timers** count down in machine cycles; **proximity sensors**, armed and on the floor, go
//!   off when someone steps by or when dropped; **igniters** light plasma in the air.
//! - Put together: a timer, sensor or signaler sets off an igniter, or a timer or sensor sets off
//!   a signaler. A secured igniter assembly on a plasma tank, the valve bored with a welder, is a
//!   bomb.
//! - **The transfer valve** holds two tanks and a device; its signal opens the valve and the
//!   first tank's gas joins the second.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, GridPos, Hand, MobStat, NetId, ObjectSpec, PickListView, TilePos};

use crate::actions::{Outbox, in_reach};
use crate::atmos::StationAtmos;
use crate::combat::VisibleEvents;
use crate::equipment::{TankGas, WornBy};
use crate::filing::{PendingPicks, PickListOut};
use crate::hands::{Hands, HeldBy};
use crate::health::Vitals;
use crate::machines::MACHINE_TICKS;
use crate::pulling::Stepped;
use crate::rng::GameRng;
use crate::time::WorldTime;
use crate::wires::PanelViewer;

pub const SIGNALER: &str = "signaler";
pub const TIMER: &str = "timer";
pub const PROX: &str = "prox-sensor";
pub const IGNITER: &str = "igniter-device";
pub const VALVE: &str = "transfer-valve";
pub const PLASMA_TANK: &str = "plasma-tank";
/// Luna's mapped timer and igniter, already put together.
pub const TIMER_IGNITER: &str = "timer-igniter";

/// Frequencies run 144.1 to 148.9, odd tenths only (Luna `sanitize_frequency`).
pub const FREQ_MIN: u16 = 1441;
pub const FREQ_MAX: u16 = 1489;
pub const FREQ_DEFAULT: u16 = 1457;
pub const CODE_DEFAULT: u8 = 30;
/// A signaler sends at most this often, ds (Luna `TRANSMISSION_DELAY`).
pub const SEND_DS: f32 = 5.0;
/// The longest a timer runs, in cycles (Luna's 600, shown as 10:00).
pub const TIME_MAX: u16 = 600;
/// A welded tank bomb goes off nine times in ten.
pub const BOMB_CHANCE: f32 = 90.0;
/// After a valve has been worked by its device, it waits this long before the next signal, ds.
pub const VALVE_LOCKOUT_DS: f32 = 50.0;
/// Our tanks hold twice the moles Luna's do (its gas constant is doubled), so a bomb's fuel is
/// halved to keep Luna's blasts.
const LUNA_MOLES: f64 = 0.5;
/// How hot an igniter's spark is, K (Luna `hotspot_expose(1000, …)`).
const SPARK_K: f64 = 1000.0;

/// A remote signaler.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Signaler {
    pub frequency: u16,
    pub code: u8,
    /// Opened with a screwdriver: it can be attached (Luna `b_stat`).
    pub open: bool,
    pub last_sent: Option<f32>,
}

impl Default for Signaler {
    fn default() -> Self {
        Self {
            frequency: FREQ_DEFAULT,
            code: CODE_DEFAULT,
            open: false,
            last_sent: None,
        }
    }
}

/// A timer: cycles left, and whether it's counting.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Timer {
    pub time: u16,
    pub timing: bool,
}

/// A proximity sensor, and whether it was carried last tick (dropping an armed one sets it off).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProxSensor {
    pub armed: bool,
    pub carried: bool,
}

/// An igniter: ready to fire, or open to be attached (Luna `status`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct IgniterDevice {
    pub ready: bool,
}

/// Two devices put together: what sets it off and what that sets off, and for a bomb, its tank.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Assembly {
    pub trigger: Entity,
    pub other: Entity,
    pub tank: Option<Entity>,
    pub secured: bool,
    /// A bomb's tank valve bored through (Luna's ptank `status`).
    pub welded: bool,
}

/// The tank transfer valve.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct Valve {
    pub tanks: [Option<Entity>; 2],
    pub device: Option<Entity>,
    pub open: bool,
    /// Its device can't work it again until then, ds.
    pub locked_until: f32,
}

/// On a part: the assembly or valve it's in.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartOf(pub Entity);

/// The parts a newly spawned device needs.
pub fn equip_item(world: &mut World, item: Entity, proto: &str) {
    match proto {
        SIGNALER => {
            world.entity_mut(item).insert(Signaler::default());
        }
        TIMER => {
            world.entity_mut(item).insert(Timer::default());
        }
        PROX => {
            world.entity_mut(item).insert(ProxSensor::default());
        }
        IGNITER => {
            world.entity_mut(item).insert(IgniterDevice { ready: true });
        }
        VALVE => {
            world.entity_mut(item).insert(Valve::default());
        }
        TIMER_IGNITER => {
            // Luna's mapped ones come with their timer and igniter inside.
            let tile = at(world, item).unwrap_or(TilePos::new(0, 0, 0));
            let (Some(timer), Some(igniter)) = (new_item(world, TIMER, tile), new_item(world, IGNITER, tile))
            else {
                return;
            };
            put_in(world, timer, item);
            put_in(world, igniter, item);
            world.entity_mut(igniter).insert(IgniterDevice { ready: false });
            world.entity_mut(item).insert(Assembly {
                trigger: timer,
                other: igniter,
                tank: None,
                secured: false,
                welded: false,
            });
        }
        _ => {}
    }
}

// --- Helpers ---

fn at(world: &World, e: Entity) -> Option<TilePos> {
    world.get::<GridPos>(e).map(|p| p.0)
}

/// The thing a part is in, all the way out.
fn outermost(world: &World, thing: Entity) -> Entity {
    let mut e = thing;
    for _ in 0..4 {
        match world.get::<PartOf>(e) {
            Some(p) => e = p.0,
            None => break,
        }
    }
    e
}

/// Where a thing is: its tile, or its holder's or wearer's, or that of what it's part of.
pub fn where_is(world: &World, thing: Entity) -> Option<TilePos> {
    let top = outermost(world, thing);
    at(world, top)
        .or_else(|| world.get::<HeldBy>(top).and_then(|h| at(world, h.0)))
        .or_else(|| world.get::<WornBy>(top).and_then(|w| at(world, w.0)))
}

fn sprite(world: &World, e: Entity) -> Option<String> {
    world.get::<Appearance>(e).map(|a| a.sprite.clone())
}

fn tell(world: &mut World, mob: Entity, id: &str) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(LocMsg::new(id));
    }
}

fn tell_msg(world: &mut World, mob: Entity, msg: LocMsg) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(msg);
    }
}

fn beep(world: &mut World, thing: Entity) {
    if let Some(tile) = where_is(world, thing) {
        world
            .resource_mut::<VisibleEvents>()
            .0
            .push((tile, LocMsg::new("device-beep"), None));
    }
}

fn awake(world: &World, mob: Entity) -> bool {
    world
        .get::<Vitals>(mob)
        .is_none_or(|v| v.stat == MobStat::Conscious)
}

/// The hand holding an item, if any.
fn hand_of(world: &World, mob: Entity, item: Entity) -> Option<Hand> {
    let hands = world.get::<Hands>(mob)?;
    [Hand::Left, Hand::Right]
        .into_iter()
        .find(|h| hands.get(*h) == Some(item))
}

/// Take a thing out of wherever it is: off the floor, out of a hand, off a back.
fn take(world: &mut World, thing: Entity) {
    if let Some(holder) = world.get::<HeldBy>(thing).map(|h| h.0)
        && let Some(hand) = hand_of(world, holder, thing)
        && let Some(mut hands) = world.get_mut::<Hands>(holder)
    {
        *hands.slot_mut(hand) = None;
    }
    if let Some(wearer) = world.get::<WornBy>(thing).map(|w| w.0)
        && let Some(mut equipment) = world.get_mut::<crate::equipment::Equipment>(wearer)
    {
        equipment.0.retain(|_, item| *item != thing);
    }
    world
        .entity_mut(thing)
        .remove::<(GridPos, HeldBy, WornBy, PartOf)>();
}

fn put_in(world: &mut World, part: Entity, into: Entity) {
    take(world, part);
    world.entity_mut(part).insert(PartOf(into));
}

/// Out onto the floor at a place.
fn drop_at(world: &mut World, thing: Entity, tile: TilePos) {
    take(world, thing);
    world.entity_mut(thing).insert(GridPos(tile));
}

fn new_item(world: &mut World, id: &str, tile: TilePos) -> Option<Entity> {
    crate::objects::spawn_object(world, &ObjectSpec::new(id), tile).ok()
}

/// Put an item in a mob's hand, replacing what was there.
fn into_hand(world: &mut World, mob: Entity, hand: Hand, item: Entity) {
    world.entity_mut(item).remove::<GridPos>().insert(HeldBy(mob));
    if let Some(mut hands) = world.get_mut::<Hands>(mob) {
        *hands.slot_mut(hand) = Some(item);
    }
}

fn despawn_all(world: &mut World, thing: Entity) {
    let parts: Vec<Entity> = {
        let mut q = world.query::<(Entity, &PartOf)>();
        q.iter(world)
            .filter(|(_, p)| p.0 == thing)
            .map(|(e, _)| e)
            .collect()
    };
    for part in parts {
        despawn_all(world, part);
    }
    take(world, thing);
    if let Ok(e) = world.get_entity_mut(thing) {
        e.despawn();
    }
}

// --- What an assembly is ---

/// The name of a trigger or other part in an assembly's sprite (Luna's `assemblies.dmi`).
fn word(world: &World, part: Entity) -> &'static str {
    if world.get::<Timer>(part).is_some() {
        "timer"
    } else if world.get::<ProxSensor>(part).is_some() {
        "prox"
    } else if world.get::<Signaler>(part).is_some() {
        "radio"
    } else {
        "igniter"
    }
}

/// The sprite of an assembly with these parts.
fn assembly_sprite(world: &World, a: &Assembly) -> String {
    let base = format!("{}-{}", word(world, a.trigger), word(world, a.other));
    if a.tank.is_some() {
        format!("{base}-tank")
    } else {
        base
    }
}

/// The state its trigger shows: 0 idle, 1 running or armed, 2 nearly out of time.
fn trigger_state(world: &World, trigger: Entity) -> Option<u8> {
    if let Some(t) = world.get::<Timer>(trigger) {
        return Some(match (t.timing, t.time) {
            (false, _) => 0,
            (true, n) if n < 5 => 2,
            _ => 1,
        });
    }
    world.get::<ProxSensor>(trigger).map(|p| u8::from(p.armed))
}

fn refresh_look(world: &mut World, thing: Entity) {
    let Some(a) = world.get::<Assembly>(thing).copied() else {
        return;
    };
    let id = assembly_sprite(world, &a);
    // A signaler-triggered igniter has one look; a timer driving a signaler never changes its.
    let state = match (word(world, a.trigger), word(world, a.other)) {
        ("radio", _) => String::new(),
        ("timer", "radio") => "0".to_owned(),
        _ => trigger_state(world, a.trigger).unwrap_or(0).to_string(),
    };
    if world
        .get::<Appearance>(thing)
        .is_none_or(|l| l.sprite != id || l.state != state)
    {
        world.entity_mut(thing).insert(Appearance::new(&id, &state));
    }
}

/// Whether a sprite is one of ours, clicked or used.
pub fn is_device(sprite: &str) -> bool {
    matches!(sprite, SIGNALER | TIMER | PROX | IGNITER | VALVE | TIMER_IGNITER) || is_assembly_sprite(sprite)
}

fn is_assembly_sprite(sprite: &str) -> bool {
    let parts: Vec<&str> = sprite.split('-').collect();
    matches!(
        parts.as_slice(),
        ["timer" | "prox" | "radio", "igniter" | "radio"] | ["timer" | "prox" | "radio", "igniter", "tank"]
    )
}

/// Whether a secured igniter assembly in hand used on this sprite makes a bomb.
pub fn is_bomb_tank(sprite: &str) -> bool {
    sprite == PLASMA_TANK
}

// --- Using things ---

/// A device, assembly or valve used in the hand: its window, or for an igniter, a spark.
pub fn use_in_hand(world: &mut World, mob: Entity, thing: Entity) {
    if !awake(world, mob) {
        return;
    }
    if let Some(i) = world.get::<IgniterDevice>(thing).copied() {
        if i.ready {
            ignite_at(world, thing);
        }
        return;
    }
    world
        .entity_mut(mob)
        .insert((PanelViewer(thing), PickListOut::default()));
}

/// Something in hand (`tool`) used on one of ours, or a secured assembly on a plasma tank.
pub fn use_on(world: &mut World, mob: Entity, tool: Entity, target: Entity) {
    if !awake(world, mob) || tool == target {
        return;
    }
    let tool_sprite = sprite(world, tool).unwrap_or_default();
    let target_sprite = sprite(world, target).unwrap_or_default();
    match tool_sprite.as_str() {
        "screwdriver" => return screwdriver(world, mob, target),
        "wrench" => return wrench(world, target),
        "welder" => return weld(world, mob, target),
        _ => {}
    }
    if world.get::<Valve>(target).is_some() {
        return attach_to_valve(world, mob, tool, target);
    }
    if is_bomb_tank(&target_sprite) {
        return make_bomb(world, tool, target);
    }
    combine(world, mob, tool, target);
}

fn screwdriver(world: &mut World, mob: Entity, target: Entity) {
    let loose = world.get::<PartOf>(target).is_none();
    if loose && let Some(mut s) = world.get_mut::<Signaler>(target) {
        s.open = !s.open;
        let open = s.open;
        tell(world, mob, if open { "signaler-open" } else { "signaler-closed" });
        return;
    }
    if let Some(mut i) = world.get_mut::<IgniterDevice>(target) {
        i.ready = !i.ready;
        let ready = i.ready;
        tell(
            world,
            mob,
            if ready {
                "igniter-ready"
            } else {
                "igniter-attachable"
            },
        );
        return;
    }
    let Some(mut a) = world.get::<Assembly>(target).copied() else {
        return;
    };
    if a.tank.is_some() {
        return;
    }
    a.secured = !a.secured;
    world.entity_mut(target).insert(a);
    if let Some(mut i) = world.get_mut::<IgniterDevice>(a.other) {
        i.ready = a.secured;
    }
    for part in [a.trigger, a.other] {
        if let Some(mut s) = world.get_mut::<Signaler>(part) {
            s.open = !a.secured;
        }
    }
    let what = match (word(world, a.trigger), word(world, a.other)) {
        ("timer", "igniter") => "timer",
        ("prox", "igniter") => "prox-igniter",
        ("radio", "igniter") => "radio-igniter",
        ("timer", _) => "signaler",
        _ => "prox",
    };
    let id = format!(
        "assembly-{}-{what}",
        if a.secured { "secured" } else { "unsecured" }
    );
    tell(world, mob, &id);
}

/// Taking an unsecured assembly apart, or an unwelded bomb off its tank.
fn wrench(world: &mut World, target: Entity) {
    let Some(a) = world.get::<Assembly>(target).copied() else {
        return;
    };
    let Some(tile) = where_is(world, target) else {
        return;
    };
    match a.tank {
        Some(tank) if !a.welded => {
            // Back to the two-part assembly, and the tank on the floor.
            drop_at(world, tank, tile);
            world.entity_mut(target).insert(Assembly {
                tank: None,
                secured: false,
                ..a
            });
            refresh_look(world, target);
        }
        None if !a.secured => {
            drop_at(world, a.trigger, tile);
            drop_at(world, a.other, tile);
            despawn_all(world, target);
        }
        _ => {}
    }
}

fn weld(world: &mut World, mob: Entity, target: Entity) {
    let Some(mut a) = world.get::<Assembly>(target).copied() else {
        return;
    };
    if a.tank.is_none() {
        return;
    }
    // Luna doesn't check the tool is lit.
    a.welded = !a.welded;
    world.entity_mut(target).insert(a);
    tell(world, mob, if a.welded { "bomb-welded" } else { "bomb-unwelded" });
}

/// Two loose devices put together into an assembly, in the hand that held the tool.
fn combine(world: &mut World, mob: Entity, tool: Entity, target: Entity) {
    if world.get::<PartOf>(target).is_some() || world.get::<Assembly>(target).is_some() {
        return;
    }
    let open_signaler = |e: Entity| world.get::<Signaler>(e).is_some_and(|s| s.open);
    let open_igniter = |e: Entity| world.get::<IgniterDevice>(e).is_some_and(|i| !i.ready);
    let timer_or_prox = |e: Entity| world.get::<Timer>(e).is_some() || world.get::<ProxSensor>(e).is_some();
    // (trigger, other)
    let made = if open_signaler(tool) && open_igniter(target) {
        Some((tool, target))
    } else if open_signaler(tool) && timer_or_prox(target) {
        Some((target, tool))
    } else if timer_or_prox(tool) && open_igniter(target) {
        Some((tool, target))
    } else {
        None
    };
    let Some((trigger, other)) = made else {
        return;
    };
    let Some(hand) = hand_of(world, mob, tool) else {
        return;
    };
    let Some(tile) = where_is(world, mob) else {
        return;
    };
    let Some(assembly) = new_item(world, TIMER_IGNITER, tile) else {
        return;
    };
    // A fresh mapped-style assembly brings parts of its own: replace them.
    if let Some(old) = world.get::<Assembly>(assembly).copied() {
        despawn_all(world, old.trigger);
        despawn_all(world, old.other);
    }
    put_in(world, trigger, assembly);
    put_in(world, other, assembly);
    world.entity_mut(assembly).insert(Assembly {
        trigger,
        other,
        tank: None,
        secured: false,
        welded: false,
    });
    into_hand(world, mob, hand, assembly);
    refresh_look(world, assembly);
}

/// A secured igniter assembly on a plasma tank: a bomb, in the hand that held the assembly.
fn make_bomb(world: &mut World, tool: Entity, tank: Entity) {
    let Some(a) = world.get::<Assembly>(tool).copied() else {
        return;
    };
    if !a.secured || a.tank.is_some() || world.get::<IgniterDevice>(a.other).is_none() {
        return;
    }
    put_in(world, tank, tool);
    world.entity_mut(tool).insert(Assembly {
        tank: Some(tank),
        welded: false,
        ..a
    });
    refresh_look(world, tool);
}

fn attach_to_valve(world: &mut World, mob: Entity, item: Entity, valve: Entity) {
    let Some(mut v) = world.get::<Valve>(valve).copied() else {
        return;
    };
    if world.get::<TankGas>(item).is_some() {
        let Some(slot) = v.tanks.iter().position(Option::is_none) else {
            tell(world, mob, "tvalve-tanks-full");
            return;
        };
        v.tanks[slot] = Some(item);
        put_in(world, item, valve);
        world.entity_mut(valve).insert(v);
        tell(
            world,
            mob,
            if slot == 0 {
                "tvalve-tank-attached"
            } else {
                "tvalve-tank-attached-second"
            },
        );
        return;
    }
    let device = world.get::<Signaler>(item).is_some()
        || world.get::<Timer>(item).is_some()
        || world.get::<ProxSensor>(item).is_some();
    if !device {
        return;
    }
    if v.device.is_some() {
        tell(world, mob, "tvalve-device-full");
        return;
    }
    v.device = Some(item);
    let name = sprite(world, item).unwrap_or_default();
    put_in(world, item, valve);
    world.entity_mut(valve).insert(v);
    tell_msg(
        world,
        mob,
        LocMsg::new("tvalve-device-attached").arg("item", name),
    );
}

// --- Signals ---

/// An igniter's spark where it is (Luna `hotspot_expose(1000, 1000)`): it lights plasma in the air.
fn ignite_at(world: &mut World, igniter: Entity) {
    let Some(tile) = where_is(world, igniter) else {
        return;
    };
    if let Some(mut atmos) = world.get_resource_mut::<StationAtmos>() {
        atmos.0.expose(tile, SPARK_K);
    }
}

/// A signaler sends what it's set to (Luna `send_signal`).
pub fn send(world: &mut World, signaler: Entity) {
    let now = world.resource::<WorldTime>().deciseconds();
    let Some(s) = world.get::<Signaler>(signaler).copied() else {
        return;
    };
    if s.last_sent.is_some_and(|t| now < t + SEND_DS) {
        return;
    }
    if let Some(mut s) = world.get_mut::<Signaler>(signaler) {
        s.last_sent = Some(now);
    }
    let mut q = world.query::<(Entity, &Signaler)>();
    let hearing: Vec<Entity> = q
        .iter(world)
        .filter(|(e, o)| *e != signaler && o.frequency == s.frequency && o.code == s.code)
        .map(|(e, _)| e)
        .collect();
    for receiver in hearing {
        // A signaler in nothing at all (a stray part) hears nothing.
        if where_is(world, receiver).is_none() {
            continue;
        }
        beep(world, receiver);
        fired(world, receiver);
    }
}

/// A part has gone off: a timer run out, a sensor sensed, a signaler heard its signal. What it's
/// part of does what it does; a loose timer or sensor beeps.
fn fired(world: &mut World, part: Entity) {
    let Some(master) = world.get::<PartOf>(part).map(|p| p.0) else {
        if world.get::<Signaler>(part).is_none() {
            beep(world, part);
        }
        return;
    };
    if world.get::<Valve>(master).is_some() {
        return valve_signal(world, master);
    }
    let Some(a) = world.get::<Assembly>(master).copied() else {
        return;
    };
    if part != a.trigger {
        return;
    }
    if world.get::<Signaler>(a.other).is_some() {
        // A timer or sensor setting off its signaler.
        send(world, a.other);
        return;
    }
    beep(world, master);
    match a.tank {
        None => {
            if a.secured && world.get::<IgniterDevice>(a.other).is_some_and(|i| i.ready) {
                ignite_at(world, a.other);
            }
        }
        Some(tank) => bomb_goes(world, master, a, tank),
    }
}

/// Luna's ptank `receive_signal`: welded, the tank goes up nine times in ten; unwelded, its gas
/// is let out.
fn bomb_goes(world: &mut World, bomb: Entity, a: Assembly, tank: Entity) {
    if a.welded {
        if world.resource_mut::<GameRng>().prob(BOMB_CHANCE) {
            ignite_tank(world, bomb, tank);
        }
        return;
    }
    // Let out where it lies; held, it's lost (Luna's mob `assume_air`).
    let on_floor = at(world, bomb);
    let gas = world.get_mut::<TankGas>(tank).map(|mut g| {
        let mix = g.mix;
        g.mix = rs13_simulation::atmos::GasMix::default();
        mix
    });
    if let (Some(tile), Some(gas)) = (on_floor, gas)
        && let Some(mut atmos) = world.get_resource_mut::<StationAtmos>()
    {
        atmos.0.add_at(tile, &gas);
    }
    if let Some(mut p) = world.get_mut::<ProxSensor>(a.trigger) {
        p.armed = false;
    }
    refresh_look(world, bomb);
}

/// Luna `tank/plasma/ignite`: a blast by how much fuel and how hot, or if too cool, the gas let
/// out and lit.
fn ignite_tank(world: &mut World, bomb: Entity, tank: Entity) {
    let Some(tile) = where_is(world, bomb) else {
        return;
    };
    let Some(gas) = world.get::<TankGas>(tank).map(|g| g.mix) else {
        return;
    };
    use rs13_simulation::atmos::Gas;
    let fuel = (gas.get(Gas::Plasma) + gas.get(Gas::Oxygen) / 6.0) * LUNA_MOLES;
    let celsius = gas.temperature - 273.15;
    let blast = if celsius > 400.0 {
        let s = fuel / 10.0;
        Some((s, 2.0 * s, 3.0 * s, 4.0 * s))
    } else if celsius > 250.0 {
        let s = fuel / 15.0;
        Some((0.0, s, 2.0 * s, 3.0 * s))
    } else if celsius > 100.0 {
        let s = fuel / 20.0;
        Some((0.0, 0.0, s, 3.0 * s))
    } else {
        None
    };
    despawn_all(world, bomb);
    match blast {
        Some((dev, heavy, light, flash)) => {
            // Luna's ranges are fractions, and a tile is hit while its distance is under them.
            let under = |r: f64| r.ceil() as i32 - 1;
            crate::explosions::explode(
                world,
                tile,
                dev.ceil() as i32,
                under(heavy),
                under(light),
                under(flash),
            );
        }
        None => {
            if let Some(mut atmos) = world.get_resource_mut::<StationAtmos>() {
                atmos.0.add_at(tile, &gas);
                atmos.0.expose(tile, SPARK_K);
            }
        }
    }
}

/// Luna `transfer_valve/receive_signal`: the valve is worked, then rests five seconds.
fn valve_signal(world: &mut World, valve: Entity) {
    let now = world.resource::<WorldTime>().deciseconds();
    let Some(v) = world.get::<Valve>(valve).copied() else {
        return;
    };
    if now < v.locked_until {
        return;
    }
    toggle_valve(world, valve);
    if let Some(mut v) = world.get_mut::<Valve>(valve) {
        v.locked_until = now + VALVE_LOCKOUT_DS;
    }
}

/// Open or shut. Opened with both tanks on, the first tank's gas goes into the second, which takes
/// in its volume too (Luna `merge_gases`).
fn toggle_valve(world: &mut World, valve: Entity) {
    let Some(mut v) = world.get::<Valve>(valve).copied() else {
        return;
    };
    v.open = !v.open;
    world.entity_mut(valve).insert(v);
    if !v.open {
        return;
    }
    let (Some(one), Some(two)) = (v.tanks[0], v.tanks[1]) else {
        return;
    };
    let Some((mix, volume)) = world.get_mut::<TankGas>(one).map(|mut g| {
        let out = (g.mix, g.volume);
        g.mix = rs13_simulation::atmos::GasMix::default();
        out
    }) else {
        return;
    };
    if let Some(mut g) = world.get_mut::<TankGas>(two) {
        g.mix.merge(&mix);
        g.volume += volume;
    }
}

// --- Every tick ---

/// Armed sensors on the floor feel steps on or beside their tile (Luna `HasProximity`), right
/// after the steps are taken.
pub fn sense_steps(world: &mut World) {
    let steps: Vec<TilePos> = {
        let stepped = world.resource::<Stepped>().0.clone();
        stepped.iter().filter_map(|(mob, _)| at(world, *mob)).collect()
    };
    if steps.is_empty() {
        return;
    }
    let mut q = world.query::<(Entity, &ProxSensor)>();
    let armed: Vec<Entity> = q.iter(world).filter(|(_, p)| p.armed).map(|(e, _)| e).collect();
    for sensor in armed {
        let top = outermost(world, sensor);
        if at(world, top).is_some_and(|here| steps.iter().any(|s| in_reach(*s, here))) {
            fired(world, sensor);
        }
    }
}

/// Timers count down each machine cycle, and armed sensors go off when dropped.
pub fn run_devices(world: &mut World) {
    let mut q = world.query::<(Entity, &ProxSensor)>();
    let sensors: Vec<(Entity, ProxSensor)> = q.iter(world).map(|(e, p)| (e, *p)).collect();
    for (sensor, p) in sensors {
        let top = outermost(world, sensor);
        let on_floor = at(world, top);
        let carried = on_floor.is_none();
        let dropped = p.carried && !carried;
        if p.carried != carried
            && let Some(mut s) = world.get_mut::<ProxSensor>(sensor)
        {
            s.carried = carried;
        }
        if p.armed && dropped {
            fired(world, sensor);
        }
    }
    if !world.resource::<WorldTime>().tick.0.is_multiple_of(MACHINE_TICKS) {
        return;
    }
    let mut q = world.query::<(Entity, &Timer)>();
    let running: Vec<(Entity, Timer)> = q
        .iter(world)
        .filter(|(_, t)| t.timing)
        .map(|(e, t)| (e, *t))
        .collect();
    for (timer, t) in running {
        if t.time > 0 {
            if let Some(mut t) = world.get_mut::<Timer>(timer) {
                t.time -= 1;
            }
        } else {
            if let Some(mut t) = world.get_mut::<Timer>(timer) {
                *t = Timer::default();
            }
            fired(world, timer);
        }
        if let Some(master) = world.get::<PartOf>(timer).map(|p| p.0) {
            refresh_look(world, master);
        }
    }
}

// --- Windows ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Line {
    Info,
    Send,
    Freq(i16),
    Code(i16),
    ToggleTiming,
    Time(i16),
    ToggleArmed,
    TakeTank(usize),
    TakeDevice,
    ToggleValve,
}

fn clock(cycles: u16) -> String {
    format!("{}:{}", cycles / 60, cycles % 60)
}

fn device_lines(world: &World, part: Entity, out: &mut Vec<(String, Line)>) {
    if let Some(s) = world.get::<Signaler>(part) {
        out.push(("msg:signaler-send".to_owned(), Line::Send));
        let freq = format!("{}.{}", s.frequency / 10, s.frequency % 10);
        out.push((format!("msg:signaler-frequency:{freq}"), Line::Info));
        for (id, step) in [("down-more", -10), ("down", -2), ("up", 2), ("up-more", 10)] {
            out.push((format!("msg:signaler-freq-{id}"), Line::Freq(step)));
        }
        out.push((format!("msg:signaler-code:{}", s.code), Line::Info));
        for (id, step) in [("down-more", -5), ("down", -1), ("up", 1), ("up-more", 5)] {
            out.push((format!("msg:signaler-code-{id}"), Line::Code(step)));
        }
    } else if let Some(t) = world.get::<Timer>(part) {
        let id = if t.timing {
            "timer-timing"
        } else {
            "timer-not-timing"
        };
        out.push((format!("msg:{id}:{}", clock(t.time)), Line::ToggleTiming));
        for (id, step) in [("down-more", -30), ("down", -1), ("up", 1), ("up-more", 30)] {
            out.push((format!("msg:timer-{id}"), Line::Time(step)));
        }
    } else if let Some(p) = world.get::<ProxSensor>(part) {
        let id = if p.armed { "prox-armed" } else { "prox-unarmed" };
        out.push((format!("msg:{id}"), Line::ToggleArmed));
    }
}

fn lines(world: &World, thing: Entity) -> Vec<(String, Line)> {
    let mut out = Vec::new();
    if let Some(a) = world.get::<Assembly>(thing) {
        device_lines(world, a.trigger, &mut out);
        if world.get::<Signaler>(a.other).is_some() {
            device_lines(world, a.other, &mut out);
        }
    } else if let Some(v) = world.get::<Valve>(thing) {
        for (i, tank) in v.tanks.iter().enumerate() {
            let n = i + 1;
            match tank {
                Some(_) => out.push((format!("msg:tvalve-tank-{n}"), Line::TakeTank(i))),
                None => out.push((format!("msg:tvalve-tank-{n}-none"), Line::Info)),
            }
        }
        match v.device {
            Some(_) => out.push(("msg:tvalve-device".to_owned(), Line::TakeDevice)),
            None => out.push(("msg:tvalve-device-none".to_owned(), Line::Info)),
        }
        let id = if v.open { "tvalve-open" } else { "tvalve-closed" };
        out.push((format!("msg:{id}"), Line::ToggleValve));
    } else {
        device_lines(world, thing, &mut out);
    }
    out
}

/// Whether the filing cabinets should leave this viewer's list alone.
pub fn keeps_own_list(sprite: &str) -> bool {
    is_device(sprite)
}

/// Held, or on the floor within reach.
fn can_work(world: &World, mob: Entity, thing: Entity) -> bool {
    if world.get::<HeldBy>(thing).is_some_and(|h| h.0 == mob) {
        return true;
    }
    at(world, mob)
        .zip(at(world, thing))
        .is_some_and(|(a, b)| in_reach(a, b))
}

/// Which part a line of an assembly's window works.
fn part_for(world: &World, thing: Entity, line: Line) -> Entity {
    let Some(a) = world.get::<Assembly>(thing) else {
        return thing;
    };
    match line {
        Line::Send | Line::Freq(_) | Line::Code(_) => {
            if world.get::<Signaler>(a.trigger).is_some() {
                a.trigger
            } else {
                a.other
            }
        }
        _ => a.trigger,
    }
}

pub fn run_picks(world: &mut World) {
    let mut q = world.query::<(Entity, &mut PendingPicks)>();
    let picks: Vec<(Entity, Vec<(Entity, usize)>)> = q
        .iter_mut(world)
        .filter(|(_, p)| !p.0.is_empty())
        .map(|(mob, mut p)| (mob, std::mem::take(&mut p.0)))
        .collect();
    for (mob, list) in picks {
        let mut others = Vec::new();
        for (target, line) in list {
            if !sprite(world, target).is_some_and(|s| is_device(&s)) {
                others.push((target, line));
                continue;
            }
            if !can_work(world, mob, target) || !awake(world, mob) {
                continue;
            }
            if let Some(chosen) = lines(world, target).get(line).map(|(_, l)| *l) {
                do_line(world, mob, target, chosen);
            }
        }
        if !others.is_empty()
            && let Some(mut pending) = world.get_mut::<PendingPicks>(mob)
        {
            pending.0.extend(others);
        }
    }
}

fn do_line(world: &mut World, mob: Entity, thing: Entity, line: Line) {
    let part = part_for(world, thing, line);
    match line {
        Line::Info => {}
        Line::Send => send(world, part),
        Line::Freq(step) => {
            if let Some(mut s) = world.get_mut::<Signaler>(part) {
                let f = (i32::from(s.frequency) + i32::from(step))
                    .clamp(i32::from(FREQ_MIN), i32::from(FREQ_MAX));
                let f = if f % 2 == 0 { f + 1 } else { f };
                s.frequency = f as u16;
            }
        }
        Line::Code(step) => {
            if let Some(mut s) = world.get_mut::<Signaler>(part) {
                s.code = (i32::from(s.code) + i32::from(step)).clamp(1, 100) as u8;
            }
        }
        Line::ToggleTiming => {
            if let Some(mut t) = world.get_mut::<Timer>(part) {
                t.timing = !t.timing;
            }
        }
        Line::Time(step) => {
            if let Some(mut t) = world.get_mut::<Timer>(part) {
                t.time = (i32::from(t.time) + i32::from(step)).clamp(0, i32::from(TIME_MAX)) as u16;
            }
        }
        Line::ToggleArmed => {
            if let Some(mut p) = world.get_mut::<ProxSensor>(part) {
                p.armed = !p.armed;
            }
        }
        Line::TakeTank(i) => {
            let tank = world.get_mut::<Valve>(thing).and_then(|mut v| v.tanks[i].take());
            if let (Some(tank), Some(tile)) = (tank, where_is(world, thing)) {
                drop_at(world, tank, tile);
            }
        }
        Line::TakeDevice => {
            let device = world.get_mut::<Valve>(thing).and_then(|mut v| v.device.take());
            if let (Some(device), Some(tile)) = (device, where_is(world, thing)) {
                drop_at(world, device, tile);
            }
        }
        Line::ToggleValve => {
            // By hand only while holding it (Luna `loc == usr`).
            if world.get::<HeldBy>(thing).is_some_and(|h| h.0 == mob) {
                toggle_valve(world, thing);
            }
        }
    }
    refresh_look(world, thing);
}

/// Keep each open window in step, and close it once out of reach.
pub fn update_views(world: &mut World) {
    let mut viewers = world.query::<(Entity, &PanelViewer)>();
    let all: Vec<(Entity, Entity)> = viewers.iter(world).map(|(m, v)| (m, v.0)).collect();
    for (mob, target) in all {
        let Some(title) = sprite(world, target).filter(|s| is_device(s)) else {
            continue;
        };
        let view = match (can_work(world, mob, target), world.get::<NetId>(target).copied()) {
            (true, Some(id)) => Some(PickListView {
                target: id,
                title,
                rows: lines(world, target).into_iter().map(|(r, _)| r).collect(),
            }),
            _ => {
                world.entity_mut(mob).remove::<PanelViewer>();
                None
            }
        };
        if let Some(mut out) = world.get_mut::<PickListOut>(mob)
            && out.0 != view
        {
            out.0 = view;
        }
    }
}
