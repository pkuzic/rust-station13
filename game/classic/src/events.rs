//! Random events (Luna's `WorkInProgress/Events`): every 1000 seconds of a round one is picked at
//! random, announced, run for a while and announced over. Spec: `docs/reference/events.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, GridPos, MapLayer, TilePos};

use crate::actions::Outbox;
use crate::doors::Airlock;
use crate::genetics::Radiation;
use crate::hands::{HeldBy, Item};
use crate::lights::{Light, LightStatus};
use crate::machines::{Canister, Switch};
use crate::movement::Mob;
use crate::objects::NetIds;
use crate::power::Apc;
use crate::rng::GameRng;
use crate::round::{Phase, Round};
use crate::time::WorldTime;
use crate::wires::{WireFn, Wiring};

/// How often an event comes, ds (Luna's `sleep(10000)`), and how often a running one acts.
pub const EVERY_DS: f32 = 10_000.0;
pub const TICK_DS: f32 = 10.0;
/// A wormhole lasts 30 s (Luna's portal deletes itself then).
pub const WORMHOLE_DS: f32 = 300.0;
pub const WORMHOLE: &str = "wormhole";

/// One of Luna's events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    ElectricalStorm,
    GasLeak,
    IonStorm,
    MeteorShower,
    PortalStorm,
    PowerOffline,
    RadiationBelt,
    ViralInfection,
}

impl EventKind {
    pub const ALL: [EventKind; 8] = [
        EventKind::ElectricalStorm,
        EventKind::GasLeak,
        EventKind::IonStorm,
        EventKind::MeteorShower,
        EventKind::PortalStorm,
        EventKind::PowerOffline,
        EventKind::RadiationBelt,
        EventKind::ViralInfection,
    ];

    pub fn id(self) -> &'static str {
        match self {
            EventKind::ElectricalStorm => "electrical-storm",
            EventKind::GasLeak => "gas-leak",
            EventKind::IonStorm => "ion-storm",
            EventKind::MeteorShower => "meteor-shower",
            EventKind::PortalStorm => "portal-storm",
            EventKind::PowerOffline => "power-offline",
            EventKind::RadiationBelt => "radiation-belt",
            EventKind::ViralInfection => "viral-infection",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.id() == id)
    }

    /// Seconds it runs (Luna `Lifetime`: 30 to 120, some fixed).
    fn lifetime(self, rng: &mut GameRng) -> u32 {
        match self {
            EventKind::GasLeak | EventKind::RadiationBelt => 10,
            // Over at once: the wormholes come by themselves, and an epidemic has no virus yet.
            EventKind::PortalStorm | EventKind::ViralInfection => 0,
            _ => rng.range(30, 120) as u32,
        }
    }
}

/// The event running now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Active {
    pub kind: EventKind,
    /// Seconds it runs, and has run.
    pub lifetime: u32,
    pub active_for: u32,
    next_tick: f32,
}

#[derive(Resource, Debug, Clone, PartialEq)]
pub struct RandomEvents {
    /// Events come at all (Luna `EventsOn`, the admins' "Toggle random events").
    pub on: bool,
    /// When the next is due, ds, once a round is under way.
    pub next_at: Option<f32>,
    pub active: Option<Active>,
    /// Wormholes yet to open: where, and when.
    pub wormholes: Vec<(TilePos, f32)>,
}

impl Default for RandomEvents {
    fn default() -> Self {
        Self {
            on: true,
            next_at: None,
            active: None,
            wormholes: Vec::new(),
        }
    }
}

/// A wormhole: anyone and anything loose that comes onto it is thrown out near `target`.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Wormhole {
    pub target: TilePos,
    pub until: f32,
    /// What was on it last tick.
    pub on_it: Vec<Entity>,
}

fn tell_everyone(world: &mut World, msg: &LocMsg) {
    let mut people = world.query::<&mut Outbox>();
    for mut outbox in people.iter_mut(world) {
        outbox.0.push(msg.clone());
    }
}

/// Luna's `command_alert`: a message to everyone from Central Command, under a heading.
fn alert(world: &mut World, title: &str, text: &str) {
    tell_everyone(
        world,
        &LocMsg::new("command-alert").msg("title", title).msg("text", text),
    );
}

fn playing(world: &World) -> bool {
    world
        .get_resource::<Round>()
        .is_some_and(|r| matches!(r.phase, Phase::Playing { .. }))
}

/// Start an event now, unless one is running (Luna `SpawnEvent`, and the admins' "Force Event").
/// False if one was.
pub fn start(world: &mut World, kind: EventKind) -> bool {
    world.init_resource::<RandomEvents>();
    if world.resource::<RandomEvents>().active.is_some() {
        return false;
    }
    let now = world.resource::<WorldTime>().deciseconds();
    let lifetime = kind.lifetime(&mut world.resource_mut::<GameRng>());
    world.resource_mut::<RandomEvents>().active = Some(Active {
        kind,
        lifetime,
        active_for: 0,
        next_tick: now,
    });
    announce(world, kind, now);
    true
}

/// One at random.
pub fn start_random(world: &mut World) -> bool {
    let i = world
        .resource_mut::<GameRng>()
        .range(0, EventKind::ALL.len() as i32 - 1) as usize;
    start(world, EventKind::ALL[i])
}

fn announce(world: &mut World, kind: EventKind, now: f32) {
    match kind {
        EventKind::ElectricalStorm => alert(world, "alert-anomaly", "event-storm-start"),
        EventKind::GasLeak => gas_leak(world),
        // Luna's ion storm changes only an AI's laws, and there's no AI yet. Its epidemic
        // needs viruses.
        EventKind::IonStorm | EventKind::ViralInfection => {}
        EventKind::MeteorShower => alert(world, "alert-meteor", "event-meteors-start"),
        EventKind::PortalStorm => {
            alert(world, "alert-anomaly", "event-portals-start");
            plan_wormholes(world, now);
        }
        EventKind::PowerOffline => alert(world, "alert-maintenance", "event-grid-check"),
        EventKind::RadiationBelt => alert(world, "alert-medical", "event-radiation-start"),
    }
}

fn finish(world: &mut World, kind: EventKind) {
    match kind {
        EventKind::ElectricalStorm => alert(world, "alert-anomaly", "event-storm-end"),
        EventKind::MeteorShower => alert(world, "alert-meteor", "event-meteors-end"),
        EventKind::RadiationBelt => alert(world, "alert-medical", "event-radiation-end"),
        _ => {}
    }
}

/// Every tick: the next event when it's due, the running one each second, and wormholes.
pub fn run_events(world: &mut World) {
    world.init_resource::<RandomEvents>();
    let now = world.resource::<WorldTime>().deciseconds();
    if playing(world) {
        let due = {
            let mut events = world.resource_mut::<RandomEvents>();
            let next = *events.next_at.get_or_insert(now + EVERY_DS);
            if now >= next {
                events.next_at = Some(now + EVERY_DS);
                events.on
            } else {
                false
            }
        };
        if due {
            start_random(world);
        }
    } else {
        world.resource_mut::<RandomEvents>().next_at = None;
    }
    if let Some(active) = world.resource::<RandomEvents>().active
        && now >= active.next_tick
    {
        if active.active_for >= active.lifetime {
            world.resource_mut::<RandomEvents>().active = None;
            finish(world, active.kind);
        } else {
            tick(world, active.kind);
            if let Some(a) = world.resource_mut::<RandomEvents>().active.as_mut() {
                a.active_for += 1;
                a.next_tick = now + TICK_DS;
            }
        }
    }
    run_wormholes(world, now);
}

fn tick(world: &mut World, kind: EventKind) {
    match kind {
        EventKind::ElectricalStorm => {
            for _ in 0..2 {
                if world.resource_mut::<GameRng>().prob(50.0) {
                    blow_light(world);
                }
            }
            if world.resource_mut::<GameRng>().prob(20.0) {
                disrupt_apc(world);
            }
            if world.resource_mut::<GameRng>().prob(20.0) {
                disable_door(world);
            }
        }
        EventKind::MeteorShower => {
            if world.resource_mut::<GameRng>().prob(40.0) {
                crate::meteor::meteor_wave(world);
            }
        }
        EventKind::RadiationBelt => irradiate(world),
        _ => {}
    }
}

fn pick<T: Copy>(world: &mut World, from: &[T]) -> Option<T> {
    if from.is_empty() {
        return None;
    }
    let i = world.resource_mut::<GameRng>().range(0, from.len() as i32 - 1) as usize;
    Some(from[i])
}

/// A working light fixture blows.
fn blow_light(world: &mut World) {
    let mut q = world.query::<(Entity, &Light)>();
    let ok: Vec<Entity> = q
        .iter(world)
        .filter(|(_, l)| l.status == LightStatus::Ok)
        .map(|(e, _)| e)
        .collect();
    if let Some(light) = pick(world, &ok)
        && let Some(mut l) = world.get_mut::<Light>(light)
    {
        l.status = LightStatus::Broken;
    }
}

/// A working area power controller may trip its breaker.
fn disrupt_apc(world: &mut World) {
    let mut q = world.query::<(Entity, &Apc)>();
    let working: Vec<Entity> = q
        .iter(world)
        .filter(|(_, a)| a.operating)
        .map(|(e, _)| e)
        .collect();
    let Some(apc) = pick(world, &working) else {
        return;
    };
    if world.resource_mut::<GameRng>().prob(40.0)
        && let Some(mut a) = world.get_mut::<Apc>(apc)
    {
        a.operating = false;
    }
}

/// An airlock's bolt wire is pulsed, then two others at random (not the bolts).
fn disable_door(world: &mut World) {
    const OTHERS: [WireFn; 8] = [
        WireFn::IdScan,
        WireFn::MainPower1,
        WireFn::MainPower2,
        WireFn::BackupPower1,
        WireFn::BackupPower2,
        WireFn::OpenDoor,
        WireFn::AiControl,
        WireFn::Electrify,
    ];
    let mut q = world.query_filtered::<Entity, (With<Airlock>, With<Wiring>)>();
    let doors: Vec<Entity> = q.iter(world).collect();
    let Some(door) = pick(world, &doors) else {
        return;
    };
    let mut wires = vec![WireFn::Bolts];
    for _ in 0..2 {
        if let Some(w) = pick(world, &OTHERS) {
            wires.push(w);
        }
    }
    let now = world.resource::<WorldTime>().deciseconds();
    let mut q = world.query::<(&mut Wiring, &mut Airlock)>();
    if let Ok((mut wiring, mut airlock)) = q.get_mut(world, door) {
        for wire in wires {
            wiring.pulse(wire, &mut airlock, now);
        }
    }
}

/// Luna's gas leak: one to three canisters of air, oxygen or nitrogen open their valves to between
/// one and ten atmospheres; one in ten bursts and lets out everything.
fn gas_leak(world: &mut World) {
    let mut q = world.query::<(Entity, &Appearance, &Canister)>();
    let leaky: Vec<Entity> = q
        .iter(world)
        .filter(|(_, a, _)| {
            matches!(
                a.sprite.as_str(),
                "canister" | "empty-canister" | "oxygen-canister" | "nitrogen-canister"
            )
        })
        .map(|(e, ..)| e)
        .collect();
    let n = world.resource_mut::<GameRng>().range(1, 3);
    let atm = rs13_simulation::atmos::gas::ONE_ATMOSPHERE;
    for _ in 0..n {
        let Some(can) = pick(world, &leaky) else {
            return;
        };
        let (burst, kpa) = {
            let mut rng = world.resource_mut::<GameRng>();
            (rng.prob(10.0), atm + rng.range(0, 900) as f64 / 100.0 * atm)
        };
        // Burst: nothing holds the gas back.
        let target = if burst { 1.0e9 } else { kpa };
        world.entity_mut(can).insert((
            Switch(true),
            Canister {
                release_target: target,
            },
        ));
    }
}

/// Luna's radiation belt, each second: everyone gets 1 to 5 rads, now and then 1 to 10 more, and
/// a person, rarely, a gene changed.
fn irradiate(world: &mut World) {
    let mut q = world.query_filtered::<(Entity, &Appearance), With<Mob>>();
    let mobs: Vec<(Entity, bool, bool)> = q
        .iter(world)
        .map(|(e, a)| {
            let monkey = crate::monkey_round::is_monkey(a);
            (e, !monkey && crate::genetics::is_person(world, e), monkey)
        })
        .collect();
    for (mob, person, monkey) in mobs {
        if !person && !monkey {
            continue;
        }
        let mut add = {
            let mut rng = world.resource_mut::<GameRng>();
            let mut add = rng.range(1, 5) as f32;
            if rng.prob(5.0) {
                add += rng.range(1, 10) as f32;
            }
            add
        };
        if person && world.resource_mut::<GameRng>().prob(2.0) {
            if world.resource_mut::<GameRng>().prob(50.0) {
                crate::genetics::random_bad(world, mob);
            } else {
                crate::genetics::random_good(world, mob);
            }
            crate::genetics::powers::check(world, mob, true);
        }
        add += world.get::<Radiation>(mob).map_or(0.0, |r| r.0);
        world.entity_mut(mob).insert(Radiation(add));
    }
}

fn floors(world: &mut World) -> Vec<TilePos> {
    let mut layers = world.query::<&MapLayer>();
    let mut out = Vec::new();
    for layer in layers.iter(world) {
        let map = &layer.0;
        for y in 0..map.height as i32 {
            for x in 0..map.width as i32 {
                let at = TilePos::new(x, y, map.z);
                if map
                    .tile(at)
                    .is_some_and(|t| !t.solid && t.id != "space" && t.id != "open-space")
                {
                    out.push(at);
                }
            }
        }
    }
    out
}

/// Luna's portal storm: a fifth of the station's floors each get a wormhole, opening at a random
/// moment over the next five minutes.
fn plan_wormholes(world: &mut World, now: f32) {
    let floors = floors(world);
    let mut planned = Vec::new();
    {
        let mut rng = world.resource_mut::<GameRng>();
        for at in floors {
            if rng.prob(20.0) {
                planned.push((at, now + 50.0 + rng.range(0, 3000) as f32));
            }
        }
    }
    world.resource_mut::<RandomEvents>().wormholes.extend(planned);
}

fn run_wormholes(world: &mut World, now: f32) {
    let opening: Vec<TilePos> = {
        let mut events = world.resource_mut::<RandomEvents>();
        if events.wormholes.is_empty() {
            Vec::new()
        } else {
            let (due, later): (Vec<_>, Vec<_>) = events.wormholes.drain(..).partition(|(_, t)| now >= *t);
            events.wormholes = later;
            due.into_iter().map(|(at, _)| at).collect()
        }
    };
    if !opening.is_empty() {
        let floors = floors(world);
        for at in opening {
            let Some(target) = pick(world, &floors) else {
                break;
            };
            let id = world.resource_mut::<NetIds>().alloc();
            world.spawn((
                GridPos(at),
                id,
                Appearance::new(WORMHOLE, ""),
                Wormhole {
                    target,
                    until: now + WORMHOLE_DS,
                    on_it: Vec::new(),
                },
            ));
        }
    }
    let mut q = world.query::<(Entity, &Wormhole, &GridPos)>();
    let holes: Vec<(Entity, Wormhole, TilePos)> =
        q.iter(world).map(|(e, w, p)| (e, w.clone(), p.0)).collect();
    if holes.is_empty() {
        return;
    }
    let mut movers = world.query_filtered::<(Entity, &GridPos, Option<&Item>, Has<Mob>), Without<HeldBy>>();
    let loose: Vec<(Entity, TilePos)> = movers
        .iter(world)
        .filter(|(_, _, item, mob)| *mob || item.is_some())
        .map(|(e, p, ..)| (e, p.0))
        .collect();
    for (hole, mut w, at) in holes {
        if now >= w.until {
            world.entity_mut(hole).despawn();
            continue;
        }
        let here: Vec<Entity> = loose.iter().filter(|(_, p)| *p == at).map(|(e, _)| *e).collect();
        for thing in &here {
            if !w.on_it.contains(thing) {
                throw_through(world, *thing, w.target);
            }
        }
        w.on_it = here;
        world.entity_mut(hole).insert(w);
    }
}

/// Out near the far end: the target or a tile next to it (Luna's `do_teleport` with precision 1).
fn throw_through(world: &mut World, thing: Entity, target: TilePos) {
    let (dx, dy) = {
        let mut rng = world.resource_mut::<GameRng>();
        (rng.range(-1, 1), rng.range(-1, 1))
    };
    let near = TilePos::new(target.x + dx, target.y + dy, target.z);
    let open = {
        let mut layers = world.query::<&MapLayer>();
        layers
            .iter(world)
            .find(|l| l.0.z == near.z)
            .and_then(|l| l.0.tile(near))
            .is_some_and(|t| !t.solid)
    };
    world
        .entity_mut(thing)
        .insert(GridPos(if open { near } else { target }));
}

/// A wormhole clicked: through it you go (Luna's portal `attack_hand`).
pub fn click_wormhole(world: &mut World, mob: Entity, hole: Entity) {
    if let Some(target) = world.get::<Wormhole>(hole).map(|w| w.target) {
        throw_through(world, mob, target);
    }
}
