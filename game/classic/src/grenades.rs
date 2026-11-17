//! Grenades: the flashbang and the EMP grenade (Luna's `grenade/flashbang` and `grenade/emp`,
//! `weapons/grenades.dm`), and their boxes. Spec: `docs/reference/grenades.md`.
//!
//! - **Priming**: used in the hand, a grenade is primed and goes off after its delay. A clumsy
//!   person sets it off in half a second.
//! - **Throwing**: clicked at somewhere out of reach, it is primed if it wasn't, dropped and
//!   flies there a tile every 0.3 s, stopping at walls and anything dense. Picked up, it stops.
//! - **Flashbang**: a bang that knocks down, deafens and dazzles everyone who can see it, worst
//!   close by, and can hurt eyes and ears for good. A screwdriver sets 3 or 10 seconds.
//! - **EMP grenade**: an electromagnetic pulse, and radios, flashes, doors and cameras near it go
//!   wrong.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, Dense, Dir, GridPos, MapLayer, Opaque, Slot, TilePos};

use crate::actions::Outbox;
use crate::atmos::StationAtmos;
use crate::changeling::Deafened;
use crate::chemistry::Reagents;
use crate::combat::{COUNT_DS, Paralyzed};
use crate::equipment::{Equipment, WornBy};
use crate::flashes::Weakened;
use crate::genetics::{Gene, Mutations};
use crate::hands::{Hands, HeldBy};
use crate::health::Body;
use crate::movement::Mob;
use crate::rng::GameRng;
use crate::sound::SoundEvents;
use crate::storage::InStorage;
use crate::time::WorldTime;

pub const FLASHBANG: &str = "flashbang";
pub const EMP_GRENADE: &str = "emp-grenade";
pub const FLASHBANG_BOX: &str = "flashbang-box";
pub const EMP_BOX: &str = "emp-grenade-box";
/// A chemical grenade's casing, and the ready-made ones.
pub const CASING: &str = "grenade-casing";
pub const METAL_FOAM_GRENADE: &str = "metal-foam-grenade";
pub const CLEANER_GRENADE: &str = "cleaner-grenade";
pub const TEARGAS_GRENADE: &str = "teargas-grenade";
/// A chemical grenade's delay, ds (Luna `spawn(30)`), and how far what's left in it splashes.
pub const CHEM_DS: f32 = 30.0;
pub const SPLASH_RANGE: i32 = 3;
/// What a casing holds once its beakers are emptied into it (Luna `reagents(1000)`).
pub const CASING_HOLDS: f32 = 1000.0;

/// A flashbang's delay, short and long, and an EMP grenade's, ds (Luna `det_time`).
pub const FLASHBANG_SHORT_DS: f32 = 30.0;
pub const FLASHBANG_LONG_DS: f32 = 100.0;
pub const EMP_DS: f32 = 50.0;
/// How soon a grenade goes off in clumsy hands, ds (Luna `spawn(5)`).
pub const CLUMSY_DS: f32 = 5.0;
/// Time between a thrown grenade's steps, ds (Luna `walk_towards(src, t, 3)`).
pub const STEP_DS: f32 = 3.0;
/// How far a bang reaches anyone who can see it (Luna's `viewers`, `world.view`).
pub const SIGHT: i32 = 7;
/// How far an EMP grenade reaches (Luna `world.view - 1`).
pub const EMP_RANGE: i32 = 6;
/// Eye and ear damage past which they start to fail (Luna's `eye_stat` 20, `ear_damage` 15).
pub const EYES_BURN: f32 = 20.0;
pub const EARS_RING_BADLY: f32 = 15.0;
pub const EARS_RING: f32 = 5.0;
/// Ear damage under this heals by 0.05 a life tick (Luna `carbon/life`).
pub const EARS_HEAL_UNDER: f32 = 25.0;
pub const EARS_HEAL: f32 = 0.05;
/// How long an EMP grenade leaves a door live and a camera dark, ds (Luna `spawn(300)`, `spawn(900)`).
pub const DOOR_LIVE_DS: f32 = 300.0;
pub const CAMERA_DARK_DS: f32 = 900.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Flashbang,
    Emp,
    Chem,
}

/// A grenade: what it does, how long it waits once primed, and when it goes off.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Grenade {
    pub kind: Kind,
    pub delay_ds: f32,
    pub goes_off_ds: Option<f32>,
}

/// A thrown grenade on its way: the tiles still ahead, and when it next moves.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Flying {
    pub path: Vec<TilePos>,
    pub next_ds: f32,
}

/// How hurt a mob's eyes are (Luna `eye_stat`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct EyeDamage(pub f32);

/// How hurt a mob's ears are (Luna `ear_damage`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct EarDamage(pub f32);

/// A camera an EMP grenade put out until then, ds.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct CameraDark(pub f32);

/// A chemical grenade's casing as it's put together (Luna `chem_grenade`'s `stage`): 0 empty, 1 with
/// its timer-igniter, 2 locked; and the beakers in it.
#[derive(Component, Debug, Clone, PartialEq, Default)]
pub struct ChemGrenade {
    pub stage: u8,
    pub beakers: Vec<Entity>,
}

pub fn is_grenade(sprite: &str) -> bool {
    matches!(sprite, FLASHBANG | EMP_GRENADE) || is_chem_grenade(sprite)
}

pub fn is_chem_grenade(sprite: &str) -> bool {
    matches!(
        sprite,
        CASING | METAL_FOAM_GRENADE | CLEANER_GRENADE | TEARGAS_GRENADE
    )
}

type Fill = &'static [(&'static str, f32)];

/// What each ready-made chemical grenade's two beakers hold (Luna's `chem_grenade` kinds).
fn ready_made(proto: &str) -> Option<[Fill; 2]> {
    Some(match proto {
        METAL_FOAM_GRENADE => [
            &[("aluminium", 30.0)],
            &[("foaming_agent", 10.0), ("pacid", 10.0)],
        ],
        CLEANER_GRENADE => [
            &[("fluorosurfactant", 30.0)],
            &[("water", 10.0), ("cleaner", 10.0)],
        ],
        TEARGAS_GRENADE => [
            &[("condensedcapsaicin", 25.0), ("potassium", 25.0)],
            &[("phosphorus", 25.0), ("sugar", 25.0)],
        ],
        _ => return None,
    })
}

/// The parts a newly spawned grenade needs.
pub fn equip_item(world: &mut World, item: Entity, proto: &str) {
    let (kind, delay_ds) = match proto {
        FLASHBANG => (Kind::Flashbang, FLASHBANG_SHORT_DS),
        EMP_GRENADE => (Kind::Emp, EMP_DS),
        _ if is_chem_grenade(proto) => (Kind::Chem, CHEM_DS),
        _ => return,
    };
    if kind == Kind::Chem {
        let mut casing = ChemGrenade::default();
        if let Some(fills) = ready_made(proto) {
            let tile = where_is(world, item).unwrap_or(TilePos::new(0, 0, 0));
            for fill in fills {
                let Ok(beaker) =
                    crate::objects::spawn_object(world, &rs13_world::ObjectSpec::new("beaker"), tile)
                else {
                    continue;
                };
                if let Some(mut r) = world.get_mut::<Reagents>(beaker) {
                    for (id, amount) in fill {
                        r.add(id, *amount);
                    }
                }
                world
                    .entity_mut(beaker)
                    .remove::<GridPos>()
                    .insert(crate::devices::PartOf(item));
                casing.beakers.push(beaker);
            }
            casing.stage = 2;
            if let Some(mut look) = world.get_mut::<Appearance>(item) {
                look.state = "locked".to_owned();
            }
        }
        world
            .entity_mut(item)
            .insert((casing, Reagents::new(CASING_HOLDS)));
    }
    world.entity_mut(item).insert(Grenade {
        kind,
        delay_ds,
        goes_off_ds: None,
    });
}

fn tell(world: &mut World, mob: Entity, msg: LocMsg) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(msg);
    }
}

fn clumsy(world: &World, mob: Entity) -> bool {
    world
        .get::<Mutations>(mob)
        .is_some_and(|m| m.0.contains(&Gene::Clumsy))
}

/// Where a thing is: its tile, or that of whoever holds or wears it, or of what it's packed in.
pub fn where_is(world: &World, thing: Entity) -> Option<TilePos> {
    let mut e = thing;
    for _ in 0..4 {
        if let Some(p) = world.get::<GridPos>(e) {
            return Some(p.0);
        }
        e = if let Some(h) = world.get::<HeldBy>(e) {
            h.0
        } else if let Some(w) = world.get::<WornBy>(e) {
            w.0
        } else if let Some(s) = world.get::<InStorage>(e) {
            s.0
        } else {
            return crate::devices::where_is(world, e);
        };
    }
    None
}

/// The mob carrying a thing, in a hand, worn or packed away.
fn carrier(world: &World, thing: Entity) -> Option<Entity> {
    let mut e = thing;
    for _ in 0..4 {
        if world.get::<Mob>(e).is_some() {
            return Some(e);
        }
        e = world
            .get::<HeldBy>(e)
            .map(|h| h.0)
            .or_else(|| world.get::<WornBy>(e).map(|w| w.0))
            .or_else(|| world.get::<InStorage>(e).map(|s| s.0))?;
    }
    None
}

/// Prime it, if it isn't already: off after its delay, or half a second in clumsy hands.
/// Whether it was primed now.
fn prime(world: &mut World, mob: Entity, grenade: Entity, sound: bool) -> bool {
    let Some(state) = world.get::<Grenade>(grenade).copied() else {
        return false;
    };
    if state.goes_off_ds.is_some() {
        return false;
    }
    let chem = state.kind == Kind::Chem;
    if chem && world.get::<ChemGrenade>(grenade).is_none_or(|c| c.stage < 2) {
        return false;
    }
    let item = world
        .get::<Appearance>(grenade)
        .map(|a| a.sprite.clone())
        .unwrap_or_default();
    let sound = sound || chem;
    let now = world.resource::<WorldTime>().deciseconds();
    let delay = if !chem && clumsy(world, mob) {
        tell(world, mob, LocMsg::new("grenade-clumsy"));
        CLUMSY_DS
    } else {
        tell(
            world,
            mob,
            LocMsg::new("grenade-primed")
                .arg("item", item)
                .num("seconds", (state.delay_ds / 10.0).round() as i64),
        );
        state.delay_ds
    };
    if let Some(mut g) = world.get_mut::<Grenade>(grenade) {
        g.goes_off_ds = Some(now + delay);
    }
    if let Some(mut look) = world.get_mut::<Appearance>(grenade) {
        look.state = "active".to_owned();
    }
    if sound && let Some(at) = where_is(world, grenade) {
        world
            .resource_mut::<SoundEvents>()
            .play(crate::sound::ARM_BOMB, at, 75.0);
    }
    true
}

/// A grenade used in the hand (Luna `attack_self`): primed.
pub fn use_in_hand(world: &mut World, mob: Entity, grenade: Entity) {
    prime(world, mob, grenade, false);
}

/// A screwdriver on a flashbang (Luna `flashbang/attackby`): 3 or 10 seconds.
pub fn set_delay(world: &mut World, mob: Entity, grenade: Entity) {
    let Some(mut g) = world.get_mut::<Grenade>(grenade) else {
        return;
    };
    if g.kind != Kind::Flashbang {
        return;
    }
    g.delay_ds = if g.delay_ds == FLASHBANG_LONG_DS {
        FLASHBANG_SHORT_DS
    } else {
        FLASHBANG_LONG_DS
    };
    let seconds = (g.delay_ds / 10.0).round() as i64;
    tell(world, mob, LocMsg::new("flashbang-delay").num("seconds", seconds));
}

/// A grenade clicked at somewhere out of reach (Luna `afterattack`): primed if it wasn't, then
/// dropped and sent flying there.
pub fn throw(world: &mut World, mob: Entity, grenade: Entity, to: TilePos) {
    let Some(at) = world.get::<GridPos>(mob).map(|p| p.0) else {
        return;
    };
    if to.z != at.z || world.get::<HeldBy>(grenade).map(|h| h.0) != Some(mob) {
        return;
    }
    let primed = prime(world, mob, grenade, true);
    // A chemical grenade is only thrown as it's primed (Luna `chem_grenade/afterattack`).
    let chem = world
        .get::<Grenade>(grenade)
        .is_some_and(|g| g.kind == Kind::Chem);
    if chem && !primed {
        return;
    }
    if let Some(dir) = Dir::from_offset((to.x - at.x).signum(), (to.y - at.y).signum())
        && let Some(mut state) = world.get_mut::<Mob>(mob)
    {
        state.facing = Some(dir);
    }
    if let Some(mut hands) = world.get_mut::<Hands>(mob) {
        if hands.left == Some(grenade) {
            hands.left = None;
        }
        if hands.right == Some(grenade) {
            hands.right = None;
        }
    }
    let now = world.resource::<WorldTime>().deciseconds();
    let path: Vec<TilePos> = crate::speech::line(at, to).skip(1).collect();
    world.entity_mut(grenade).remove::<HeldBy>().insert((
        GridPos(at),
        Flying {
            path,
            next_ds: now + STEP_DS,
        },
    ));
}

/// What stops a thrown grenade: walls, and anything dense but tables and racks.
fn blocked(world: &mut World, at: TilePos) -> bool {
    let solid = {
        let mut maps = world.query::<&MapLayer>();
        maps.iter(world)
            .find(|m| m.0.z == at.z)
            .is_none_or(|m| m.0.is_solid(at))
    };
    if solid {
        return true;
    }
    let mut dense = world.query_filtered::<(
        &GridPos,
        Has<crate::furniture::Table>,
        Has<crate::furniture::Rack>,
    ), With<Dense>>();
    dense
        .iter(world)
        .any(|(p, table, rack)| p.0 == at && !table && !rack)
}

/// Move thrown grenades along, and set off those whose time is up.
pub fn run_grenades(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    let mut flying = world.query::<(Entity, &Flying, Has<GridPos>)>();
    let moving: Vec<(Entity, Flying, bool)> = flying
        .iter(world)
        .map(|(e, f, on_floor)| (e, f.clone(), on_floor))
        .collect();
    for (grenade, mut flight, on_floor) in moving {
        // Picked up (Luna `attack_hand`'s `walk(src, null)`), or arrived.
        if !on_floor || flight.path.is_empty() {
            world.entity_mut(grenade).remove::<Flying>();
            continue;
        }
        if now < flight.next_ds {
            continue;
        }
        let next = flight.path.remove(0);
        if blocked(world, next) {
            world.entity_mut(grenade).remove::<Flying>();
            continue;
        }
        flight.next_ds = now + STEP_DS;
        world.entity_mut(grenade).insert((GridPos(next), flight));
    }
    let mut grenades = world.query::<(Entity, &Grenade)>();
    let due: Vec<(Entity, Kind)> = grenades
        .iter(world)
        .filter(|(_, g)| g.goes_off_ds.is_some_and(|t| now >= t))
        .map(|(e, g)| (e, g.kind))
        .collect();
    for (grenade, kind) in due {
        go_off(world, grenade, kind);
    }
    restore_cameras(world, now);
}

/// Take a thing out of a hand, off a body or out of a box.
fn take(world: &mut World, thing: Entity) {
    if let Some(holder) = world.get::<HeldBy>(thing).map(|h| h.0)
        && let Some(mut hands) = world.get_mut::<Hands>(holder)
    {
        if hands.left == Some(thing) {
            hands.left = None;
        }
        if hands.right == Some(thing) {
            hands.right = None;
        }
    }
    if let Some(wearer) = world.get::<WornBy>(thing).map(|w| w.0)
        && let Some(mut equipment) = world.get_mut::<Equipment>(wearer)
    {
        equipment.0.retain(|_, item| *item != thing);
    }
    if let Some(container) = world.get::<InStorage>(thing).map(|s| s.0)
        && let Some(mut storage) = world.get_mut::<crate::storage::Storage>(container)
    {
        storage.items.retain(|item| *item != thing);
    }
}

/// Set a grenade off, wherever it is, and it's gone.
pub fn go_off(world: &mut World, grenade: Entity, kind: Kind) {
    if kind == Kind::Chem {
        chem_go_off(world, grenade);
        return;
    }
    let at = where_is(world, grenade);
    let holder = carrier(world, grenade);
    take(world, grenade);
    world.entity_mut(grenade).despawn();
    let Some(at) = at else {
        return;
    };
    match kind {
        Kind::Flashbang => bang(world, at, holder),
        Kind::Emp => emp(world, at),
        Kind::Chem => {}
    }
}

/// A chemical grenade going off (Luna `chem_grenade/explode`): with nothing in its beakers it only
/// clicks and can be primed again; otherwise they're emptied into it, mixing as they go, and what
/// doesn't react splashes on everyone close by.
fn chem_go_off(world: &mut World, grenade: Entity) {
    let at = where_is(world, grenade);
    let beakers = world
        .get::<ChemGrenade>(grenade)
        .map(|c| c.beakers.clone())
        .unwrap_or_default();
    let full = beakers
        .iter()
        .any(|b| world.get::<Reagents>(*b).is_some_and(|r| r.total() > 0.0));
    if !full {
        if let Some(at) = at {
            world
                .resource_mut::<SoundEvents>()
                .play(crate::sound::SCREWDRIVER, at, 50.0);
        }
        if let Some(mut g) = world.get_mut::<Grenade>(grenade) {
            g.goes_off_ds = None;
        }
        if let Some(mut look) = world.get_mut::<Appearance>(grenade) {
            look.state = "locked".to_owned();
        }
        return;
    }
    if let Some(at) = at {
        world
            .resource_mut::<SoundEvents>()
            .play(crate::sound::BAMF, at, 50.0);
    }
    for beaker in &beakers {
        let total = world.get::<Reagents>(*beaker).map_or(0.0, Reagents::total);
        crate::chemistry::transfer(world, *beaker, grenade, total);
    }
    let left = world.get::<Reagents>(grenade).cloned().unwrap_or_default();
    take(world, grenade);
    for beaker in beakers {
        world.entity_mut(beaker).despawn();
    }
    world.entity_mut(grenade).despawn();
    let Some(at) = at else {
        return;
    };
    if left.total() > 0.0 {
        for (mob, _) in viewers(world, at, SPLASH_RANGE) {
            crate::chem_effects::touch_all(world, mob, &left);
        }
    }
}

/// Something used on a chemical grenade's casing (Luna `chem_grenade/attackby`): a timer-igniter
/// first, then up to two beakers with something in them, then a screwdriver locks it.
pub fn use_on_casing(world: &mut World, mob: Entity, tool: Entity, casing: Entity) {
    let Some(state) = world.get::<ChemGrenade>(casing).cloned() else {
        return;
    };
    let Some(sprite) = world.get::<Appearance>(tool).map(|a| a.sprite.clone()) else {
        return;
    };
    let at = where_is(world, casing);
    let click = |world: &mut World| {
        if let Some(at) = at {
            world
                .resource_mut::<SoundEvents>()
                .play(crate::sound::SCREWDRIVER, at, 25.0);
        }
    };
    match (state.stage, sprite.as_str()) {
        (0, crate::devices::TIMER_IGNITER) => {
            tell(
                world,
                mob,
                LocMsg::new("casing-add-assembly").arg("item", sprite.clone()),
            );
            click(world);
            take(world, tool);
            crate::devices::destroy(world, tool);
            set_casing(world, casing, 1, "assembled");
        }
        (1, "screwdriver") => {
            if state.beakers.is_empty() {
                tell(world, mob, LocMsg::new("casing-needs-beaker"));
            } else {
                tell(world, mob, LocMsg::new("casing-locked"));
                click(world);
                set_casing(world, casing, 2, "locked");
            }
        }
        (1, glass) if crate::chemistry::is_glass(glass) => {
            if state.beakers.len() >= 2 {
                tell(world, mob, LocMsg::new("casing-full"));
            } else if world.get::<Reagents>(tool).is_none_or(|r| r.total() <= 0.0) {
                tell(
                    world,
                    mob,
                    LocMsg::new("casing-beaker-empty").arg("item", sprite.clone()),
                );
            } else {
                tell(
                    world,
                    mob,
                    LocMsg::new("casing-add-beaker").arg("item", sprite.clone()),
                );
                take(world, tool);
                world
                    .entity_mut(tool)
                    .remove::<(GridPos, HeldBy)>()
                    .insert(crate::devices::PartOf(casing));
                if let Some(mut c) = world.get_mut::<ChemGrenade>(casing) {
                    c.beakers.push(tool);
                }
            }
        }
        _ => {}
    }
}

fn set_casing(world: &mut World, casing: Entity, stage: u8, state: &str) {
    if let Some(mut c) = world.get_mut::<ChemGrenade>(casing) {
        c.stage = stage;
    }
    if let Some(mut look) = world.get_mut::<Appearance>(casing) {
        look.state = state.to_owned();
    }
}

/// Mobs that can see a tile, within `range`, awake or not.
fn viewers(world: &mut World, at: TilePos, range: i32) -> Vec<(Entity, TilePos)> {
    let mut walls = world.query_filtered::<&GridPos, (With<Opaque>, Without<Mob>)>();
    let opaque: Vec<TilePos> = walls.iter(world).map(|p| p.0).collect();
    let mut decks = world.query::<&MapLayer>();
    let Some(map) = decks.iter(world).find(|m| m.0.z == at.z).map(|m| m.0.clone()) else {
        return Vec::new();
    };
    let mut mobs = world.query_filtered::<(Entity, &GridPos), (With<Body>, With<Mob>)>();
    mobs.iter(world)
        .filter(|(_, pos)| crate::speech::can_see(&map, &opaque, at, pos.0, range))
        .map(|(mob, pos)| (mob, pos.0))
        .collect()
}

fn wearing(world: &World, mob: Entity, slot: Slot, sprite: &str) -> bool {
    world
        .get::<Equipment>(mob)
        .and_then(|e| e.get(slot))
        .and_then(|item| world.get::<Appearance>(item))
        .is_some_and(|a| a.sprite == sprite)
}

/// Sunglasses or a welding helmet.
fn eyes_covered(world: &World, mob: Entity) -> bool {
    wearing(world, mob, Slot::Glasses, "sunglasses") || wearing(world, mob, Slot::Head, "welding-helmet")
}

fn is_human(world: &World, mob: Entity) -> bool {
    world.get::<crate::monkeys::Monkey>(mob).is_none()
}

fn knock_down(world: &mut World, mob: Entity, counts: f32) {
    let until = world.resource::<WorldTime>().deciseconds() + counts * COUNT_DS;
    let until = world.get::<Weakened>(mob).map_or(until, |w| w.0.max(until));
    world.entity_mut(mob).insert(Weakened(until));
}

fn deafen(world: &mut World, mob: Entity, counts: f32) {
    let now = world.resource::<WorldTime>().deciseconds();
    let until = world.get::<Deafened>(mob).map_or(now, |d| d.0.max(now)) + counts * COUNT_DS;
    world.entity_mut(mob).insert(Deafened(until));
}

fn hurt_eyes(world: &mut World, mob: Entity, amount: f32) {
    let was = world.get::<EyeDamage>(mob).map_or(0.0, |e| e.0);
    world.entity_mut(mob).insert(EyeDamage(was + amount));
}

fn hurt_ears(world: &mut World, mob: Entity, amount: f32) {
    let was = world.get::<EarDamage>(mob).map_or(0.0, |e| e.0);
    world.entity_mut(mob).insert(EarDamage(was + amount));
}

/// The flashbang going off (Luna `flashbang/prime`).
fn bang(world: &mut World, at: TilePos, holder: Option<Entity>) {
    world
        .resource_mut::<SoundEvents>()
        .play(crate::sound::BANG, at, 25.0);
    if let Some(mut atmos) = world.get_resource_mut::<StationAtmos>() {
        atmos.0.expose(at, crate::igniters::SPARK_TEMPERATURE);
    }
    for (mob, pos) in viewers(world, at, SIGHT) {
        let held = holder == Some(mob);
        let distance = (pos.x - at.x).abs().max((pos.y - at.y).abs());
        let out = world.get::<Paralyzed>(mob).is_some();
        let roll = |world: &mut World, lo: i32, hi: i32| world.resource_mut::<GameRng>().range(lo, hi) as f32;
        let prob = |world: &mut World, p: f32| world.resource_mut::<GameRng>().prob(p);
        if distance <= 2 || held {
            crate::flashes::dazzle(world, mob);
            knock_down(world, mob, 10.0);
            if prob(world, 14.0) || (held && prob(world, 70.0)) {
                let d = roll(world, 1, 10);
                hurt_ears(world, mob, d);
            } else if prob(world, 30.0) {
                let d = roll(world, 0, 5);
                hurt_ears(world, mob, d);
            }
            if !out {
                let d = roll(world, 0, 5);
                hurt_eyes(world, mob, d);
            }
            if prob(world, 10.0) {
                let d = roll(world, 0, 4);
                hurt_eyes(world, mob, d);
            }
            deafen(world, mob, 30.0);
            if held {
                let d = roll(world, 2, 5);
                hurt_eyes(world, mob, d);
                if prob(world, 60.0) && !wearing(world, mob, Slot::Ears, "earmuffs") {
                    hurt_ears(world, mob, 15.0);
                    deafen(world, mob, 60.0);
                }
            }
        } else if distance <= 5 {
            if !is_human(world, mob) {
                crate::flashes::dazzle(world, mob);
                knock_down(world, mob, 7.0);
            } else {
                deafen(world, mob, 10.0);
                if prob(world, 20.0) {
                    let d = roll(world, 0, 4);
                    hurt_ears(world, mob, d);
                }
                if !eyes_covered(world, mob) || out {
                    crate::flashes::dazzle(world, mob);
                    knock_down(world, mob, 7.0);
                } else {
                    let d = roll(world, 1, 3);
                    hurt_eyes(world, mob, d);
                }
            }
        } else {
            if !eyes_covered(world, mob) {
                crate::flashes::dazzle(world, mob);
            }
            let d = roll(world, 1, 2);
            hurt_eyes(world, mob, d);
            deafen(world, mob, 5.0);
        }
        tell(world, mob, LocMsg::new("flashbang-bang"));
        after_bang(world, mob);
    }
    // The blob shrinks back from it (Luna `blob/update`).
    let mut cells = world.query_filtered::<(Entity, &GridPos), With<crate::blob::BlobCell>>();
    let near: Vec<(Entity, i32)> = cells
        .iter(world)
        .filter(|(_, p)| p.0.z == at.z)
        .map(|(e, p)| (e, (p.0.x - at.x).abs().max((p.0.y - at.y).abs())))
        .filter(|(_, d)| *d <= 8)
        .collect();
    for (cell, d) in near {
        crate::blob::damage(world, cell, (30 / (d + 1)) as f32);
    }
}

/// Eyes and ears that have had too much: nearsighted or blind, ringing or deaf, for good.
fn after_bang(world: &mut World, mob: Entity) {
    let eyes = world.get::<EyeDamage>(mob).map_or(0.0, |e| e.0);
    let ears = world.get::<EarDamage>(mob).map_or(0.0, |e| e.0);
    if eyes >= EYES_BURN {
        tell(world, mob, LocMsg::new("flashbang-eyes-burn"));
        crate::eyes::nearsight(world, mob, f32::INFINITY);
        if world.resource_mut::<GameRng>().prob(eyes - EYES_BURN + 1.0) {
            tell(world, mob, LocMsg::new("flashbang-blinded"));
            crate::eyes::blind(world, mob, f32::INFINITY);
        }
    }
    if ears >= EARS_RING_BADLY {
        tell(world, mob, LocMsg::new("flashbang-ears-ring-badly"));
        if world.resource_mut::<GameRng>().prob(ears - 10.0 + 5.0) {
            tell(world, mob, LocMsg::new("flashbang-deafened"));
            world.entity_mut(mob).insert(Deafened(f32::INFINITY));
        }
    } else if ears >= EARS_RING {
        tell(world, mob, LocMsg::new("flashbang-ears-ring"));
    }
}

/// Ears heal a little each life tick while not too far gone (Luna `carbon/life`).
pub fn heal_ears(time: Res<WorldTime>, mut last: Local<f32>, mut ears: Query<&mut EarDamage>) {
    let now = time.deciseconds();
    if now - *last < COUNT_DS {
        return;
    }
    *last = now;
    for mut ear in &mut ears {
        if ear.0 < EARS_HEAL_UNDER && ear.0 > 0.0 {
            ear.0 = (ear.0 - EARS_HEAL).max(0.0);
        }
    }
}

/// The EMP grenade going off (Luna `grenade/emp/prime`): the pulse itself, and what it does to
/// what's near.
fn emp(world: &mut World, at: TilePos) {
    crate::explosions::empulse(world, at, EMP_RANGE, EMP_RANGE);
    if let Some(mut atmos) = world.get_resource_mut::<StationAtmos>() {
        atmos.0.expose(at, crate::igniters::SPARK_TEMPERATURE);
    }
    let now = world.resource::<WorldTime>().deciseconds();
    let near = |p: TilePos| p.z == at.z && (p.x - at.x).abs().max((p.y - at.y).abs()) <= EMP_RANGE;
    // Radios carried go dead, and flashes carried go off.
    for (mob, _) in viewers(world, at, EMP_RANGE) {
        let mut items = world.query::<(Entity, &Appearance)>();
        let carried: Vec<(Entity, String)> = items.iter(world).map(|(e, a)| (e, a.sprite.clone())).collect();
        for (item, sprite) in carried {
            if carrier(world, item) != Some(mob) {
                continue;
            }
            if let Some(mut radio) = world.get_mut::<crate::radio::Radio>(item) {
                radio.broadcasting = false;
                radio.listening = false;
            }
            if sprite == crate::flashes::FLASH {
                crate::flashes::flash_around(world, mob, item);
            }
        }
    }
    // Doors may go live for half a minute.
    let mut doors = world.query::<(Entity, &GridPos, &crate::wires::Wiring)>();
    let live: Vec<Entity> = doors
        .iter(world)
        .filter(|(_, p, w)| near(p.0) && !w.is_electrified())
        .map(|(e, ..)| e)
        .collect();
    for door in live {
        if world.resource_mut::<GameRng>().prob(40.0)
            && let Some(mut wiring) = world.get_mut::<crate::wires::Wiring>(door)
        {
            wiring.electrified = crate::wires::Electrified::Until(now + DOOR_LIVE_DS);
        }
    }
    // Cameras go dark for a minute and a half.
    let mut cameras = world.query::<(Entity, &GridPos, &crate::cameras::Camera)>();
    let dark: Vec<Entity> = cameras
        .iter(world)
        .filter(|(_, p, c)| near(p.0) && c.on)
        .map(|(e, ..)| e)
        .collect();
    for camera in dark {
        if let Some(mut c) = world.get_mut::<crate::cameras::Camera>(camera) {
            c.on = false;
        }
        world.entity_mut(camera).insert(CameraDark(now + CAMERA_DARK_DS));
    }
}

fn restore_cameras(world: &mut World, now: f32) {
    let mut dark = world.query::<(Entity, &CameraDark)>();
    let back: Vec<Entity> = dark
        .iter(world)
        .filter(|(_, d)| now >= d.0)
        .map(|(e, _)| e)
        .collect();
    for camera in back {
        world.entity_mut(camera).remove::<CameraDark>();
        if let Some(mut c) = world.get_mut::<crate::cameras::Camera>(camera) {
            c.on = true;
        }
    }
}
