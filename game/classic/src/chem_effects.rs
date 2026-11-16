//! What chemicals do outside a container: smoke and foam from reactions, the foamed metal walls
//! metal foam leaves, and chemicals touching someone's skin (Luna's `chem_smoke_spread`,
//! `foam_spread`, `structure/foamedmetal` and the reagents' `reaction_mob`). Spec:
//! `docs/reference/grenades.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_simulation::atmos::Cell;
use rs13_world::{Appearance, Dense, Dir, DisplayName, GridPos, MapLayer, NetId, Opaque, Slot, TilePos};

use crate::actions::Outbox;
use crate::atmos::StationAtmos;
use crate::chemistry::{Effect, Reaction, Reagents};
use crate::combat::{COUNT_DS, VisibleEvents};
use crate::equipment::{Equipment, WornBy};
use crate::flashes::Weakened;
use crate::genetics::{Gene, Mutations};
use crate::hands::{Hands, HeldBy};
use crate::health::{Body, Zone};
use crate::movement::Mob;
use crate::pulling::Stepped;
use crate::rng::GameRng;
use crate::sound::SoundEvents;
use crate::time::WorldTime;

pub const SMOKE: &str = "chem-smoke";
pub const FOAM: &str = "foam";
pub const METAL_FOAM: &str = "metal-foam";
pub const FOAMED_METAL: &str = "foamed-metal";

/// How many puffs a smoke reaction makes, twice over a second apart (Luna `set_up(holder, 10)`,
/// `start()`, `sleep(10)`, `start()`).
pub const PUFFS: u32 = 10;
pub const SECOND_PUFFS_DS: f32 = 10.0;
/// A puff drifts a tile a second (Luna `sleep(10)`, `step`).
pub const DRIFT_DS: f32 = 10.0;
/// Chances a puff touches what's next to it as it drifts, and whoever walks into it.
pub const DRIFT_TOUCH: f32 = 70.0;
pub const ENTER_TOUCH: f32 = 90.0;
/// When foam spreads, ds after it appears, and how long until it sets or melts away
/// (Luna `spawn(3 + metal*3)`, `spawn(120)` and `sleep(30)`, `sleep(5)`).
pub const FOAM_SPREAD_DS: f32 = 3.0;
pub const FOAM_LIFE_DS: f32 = 150.0;
pub const FOAM_DISSOLVE_DS: f32 = 5.0;
/// Foam melts in heat above this, K (Luna's `temperature_expose`).
pub const FOAM_MELTS_K: f64 = 475.0;
/// How long foam takes you off your feet, counts (Luna `Stun(5)`).
pub const FOAM_SLIP_COUNTS: f32 = 5.0;

/// A drifting puff of smoke, and the chemicals in it.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Smoke {
    pub reagents: Reagents,
    pub dir: Dir,
    pub steps_left: u8,
    pub next_ds: f32,
    pub ends_ds: f32,
}

/// The second round of puffs a smoke reaction makes.
#[derive(Resource, Debug, Default)]
pub struct PendingSmoke(pub Vec<(TilePos, Reagents, f32)>);

/// Foam, or metal foam: how much further it spreads, what it carries, and when it next acts.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Foam {
    pub amount: i32,
    /// 0 for foam, 1 aluminium, 2 iron.
    pub metal: u8,
    pub carried: Vec<String>,
    pub spread_ds: Option<f32>,
    pub sets_ds: f32,
    pub gone_ds: Option<f32>,
}

/// A foamed metal wall, and what the air there was before it.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct FoamedMetal {
    pub metal: u8,
}

/// Foamed metal walls standing, so the air comes back when one goes, however it goes.
#[derive(Resource, Debug, Default)]
pub struct FoamWalls(pub Vec<(Entity, TilePos, Cell)>);

fn tell(world: &mut World, mob: Entity, msg: LocMsg) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(msg);
    }
}

fn spawn_effect(world: &mut World, sprite: &str, state: &str, at: TilePos) -> Entity {
    let id: NetId = world.resource_mut::<crate::objects::NetIds>().alloc();
    world
        .spawn((GridPos(at), id, Appearance::new(sprite, state)))
        .id()
}

fn near(a: TilePos, b: TilePos, range: i32) -> bool {
    a.z == b.z && (a.x - b.x).abs().max((a.y - b.y).abs()) <= range
}

/// Everyone in reach of a tile.
fn mobs_near(world: &mut World, at: TilePos, range: i32) -> Vec<Entity> {
    let mut mobs = world.query_filtered::<(Entity, &GridPos), (With<Body>, With<Mob>)>();
    mobs.iter(world)
        .filter(|(_, p)| near(p.0, at, range))
        .map(|(e, _)| e)
        .collect()
}

/// Carry out a reaction's effect where it happened.
pub fn carry_out(world: &mut World, at: TilePos, reaction: &Reaction) {
    match reaction.effect {
        Effect::None => {}
        Effect::Smoke => {
            world
                .resource_mut::<SoundEvents>()
                .play(crate::sound::SMOKE, at, 50.0);
            puff(world, at, &reaction.carried);
            let later = world.resource::<WorldTime>().deciseconds() + SECOND_PUFFS_DS;
            world
                .get_resource_or_init::<PendingSmoke>()
                .0
                .push((at, reaction.carried.clone(), later));
        }
        Effect::Foam => {
            for mob in mobs_near(world, at, 5) {
                tell(world, mob, LocMsg::new("foam-bubbles"));
                tell(world, mob, LocMsg::new("foam-spews"));
            }
            let carried = reaction.carried.amounts.keys().cloned().collect();
            start_foam(world, at, foam_amount(reaction.created), 0, carried);
        }
        Effect::MetalFoam(metal) => {
            for mob in mobs_near(world, at, 5) {
                tell(world, mob, LocMsg::new("foam-spews-metal"));
            }
            start_foam(world, at, foam_amount(reaction.created), metal, Vec::new());
        }
    }
}

/// Luna `foam_spread/set_up`: how far foam spreads from what was made.
fn foam_amount(created: f32) -> i32 {
    (created / 3.0).sqrt().round() as i32
}

// --- Smoke ---

/// Ten puffs, each with a tenth of the chemicals, drifting off every which way.
fn puff(world: &mut World, at: TilePos, carried: &Reagents) {
    let now = world.resource::<WorldTime>().deciseconds();
    let mut share = Reagents::new(carried.max.max(500.0));
    for (r, a) in &carried.amounts {
        share.add(r, a / PUFFS as f32);
    }
    let dirs: Vec<Dir> = (-1..=1)
        .flat_map(|dx| (-1..=1).map(move |dy| (dx, dy)))
        .filter_map(|(dx, dy)| Dir::from_offset(dx, dy))
        .collect();
    for _ in 0..PUFFS {
        let (dir, steps, life, cap) = {
            let mut rng = world.resource_mut::<GameRng>();
            let dir = dirs[rng.range(0, dirs.len() as i32 - 1) as usize];
            let steps = [0u8, 1, 1, 1, 2, 2, 2, 3][rng.range(0, 7) as usize];
            (dir, steps, rng.range(160, 180) as f32, rng.range(210, 230) as f32)
        };
        let ends_ds = (now + f32::from(steps) * DRIFT_DS + life).min(now + cap);
        let e = spawn_effect(world, SMOKE, "", at);
        world.entity_mut(e).insert((
            Opaque,
            Smoke {
                reagents: share.clone(),
                dir,
                steps_left: steps,
                next_ds: now + DRIFT_DS,
                ends_ds,
            },
        ));
    }
}

fn open_tile(world: &mut World, at: TilePos) -> bool {
    let mut maps = world.query::<&MapLayer>();
    maps.iter(world)
        .find(|m| m.0.z == at.z)
        .is_some_and(|m| !m.0.is_solid(at))
}

/// Whether something not dense can move from one tile onto the next: no wall, nothing dense but
/// people, no window or window door between.
fn can_enter(world: &mut World, from: TilePos, to: TilePos) -> bool {
    if !open_tile(world, to) {
        return false;
    }
    let mut dense = world.query_filtered::<&GridPos, (With<Dense>, Without<Body>)>();
    if dense.iter(world).any(|p| p.0 == to) {
        return false;
    }
    let edge = (from, to);
    !(world.resource::<crate::windows::PaneEdges>().0.contains(&edge)
        || world
            .resource::<crate::windoors::WindoorEdges>()
            .0
            .contains(&edge))
}

pub fn run_smoke(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    let due: Vec<(TilePos, Reagents)> = {
        let mut pending = world.get_resource_or_init::<PendingSmoke>();
        let (due, wait): (Vec<_>, Vec<_>) = pending.0.drain(..).partition(|(_, _, t)| now >= *t);
        pending.0 = wait;
        due.into_iter().map(|(at, r, _)| (at, r)).collect()
    };
    for (at, carried) in due {
        puff(world, at, &carried);
    }
    let mut puffs = world.query::<(Entity, &Smoke, &GridPos)>();
    let all: Vec<(Entity, Smoke, TilePos)> = puffs.iter(world).map(|(e, s, p)| (e, s.clone(), p.0)).collect();
    if all.is_empty() {
        return;
    }
    // Whoever walks into a puff.
    let entered: Vec<(Entity, TilePos)> = world.resource::<Stepped>().0.clone();
    for (mob, to) in entered {
        if let Some((_, smoke, _)) = all.iter().find(|(_, _, p)| *p == to)
            && world.get::<Body>(mob).is_some()
            && world.resource_mut::<GameRng>().prob(ENTER_TOUCH)
        {
            touch_all(world, mob, &smoke.reagents);
        }
    }
    for (e, mut smoke, at) in all {
        if now >= smoke.ends_ds {
            world.entity_mut(e).despawn();
            continue;
        }
        if smoke.steps_left == 0 || now < smoke.next_ds {
            continue;
        }
        smoke.steps_left -= 1;
        smoke.next_ds = now + DRIFT_DS;
        let to = at.step(smoke.dir);
        let moved = can_enter(world, at, to);
        let here = if moved { to } else { at };
        if moved {
            for mob in mobs_near(world, here, 1) {
                if world.resource_mut::<GameRng>().prob(DRIFT_TOUCH) {
                    touch_all(world, mob, &smoke.reagents);
                }
            }
        }
        world.entity_mut(e).insert((GridPos(here), smoke));
    }
}

// --- Foam ---

/// Luna `foam_spread/start`: foam where it's mixed, or more for foam already there.
fn start_foam(world: &mut World, at: TilePos, amount: i32, metal: u8, carried: Vec<String>) {
    let mut foams = world.query::<(&GridPos, &mut Foam)>();
    if let Some((_, mut foam)) = foams.iter_mut(world).find(|(p, _)| p.0 == at) {
        foam.amount += amount;
        return;
    }
    let carried = if metal == 0 && carried.is_empty() {
        vec!["water".to_owned()]
    } else {
        carried
    };
    new_foam(world, at, amount, metal, carried);
}

fn new_foam(world: &mut World, at: TilePos, amount: i32, metal: u8, carried: Vec<String>) {
    let now = world.resource::<WorldTime>().deciseconds();
    world
        .resource_mut::<SoundEvents>()
        .play(crate::sound::BUBBLES, at, 80.0);
    let sprite = if metal > 0 { METAL_FOAM } else { FOAM };
    let e = spawn_effect(world, sprite, "", at);
    world.entity_mut(e).insert(Foam {
        amount,
        metal,
        carried,
        spread_ds: Some(now + FOAM_SPREAD_DS * (1.0 + f32::from(metal))),
        sets_ds: now + FOAM_LIFE_DS,
        gone_ds: None,
    });
}

const CARDINALS: [Dir; 4] = [Dir::North, Dir::East, Dir::South, Dir::West];

pub fn run_foam(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    let mut foams = world.query::<(Entity, &Foam, &GridPos)>();
    let all: Vec<(Entity, Foam, TilePos)> = foams.iter(world).map(|(e, f, p)| (e, f.clone(), p.0)).collect();
    for (e, mut foam, at) in all {
        if let Some(gone) = foam.gone_ds {
            if now >= gone {
                melt(world, e, &foam, at);
            }
            continue;
        }
        // Luna `foam/process`: spread to the open sides, one less each time.
        if foam.spread_ds.is_some_and(|t| now >= t) {
            foam.spread_ds = None;
            foam.amount -= 1;
            if foam.amount >= 0 {
                for dir in CARDINALS {
                    let to = at.step(dir);
                    if !can_enter(world, at, to) {
                        continue;
                    }
                    let mut there = world.query::<(&GridPos, &Foam)>();
                    if there.iter(world).any(|(p, _)| p.0 == to) {
                        continue;
                    }
                    new_foam(world, to, foam.amount, foam.metal, foam.carried.clone());
                }
            }
        }
        let hot = foam.metal == 0
            && world
                .get_resource::<StationAtmos>()
                .map(|a| a.0.air_at(at).temperature)
                .is_some_and(|t| t > FOAM_MELTS_K)
            && {
                let t = world.resource::<StationAtmos>().0.air_at(at).temperature;
                world.resource_mut::<GameRng>().prob((t - FOAM_MELTS_K) as f32)
            };
        if now >= foam.sets_ds || hot {
            if foam.metal > 0 && !hot {
                foamed_metal(world, at, foam.metal);
            }
            foam.gone_ds = Some(now + FOAM_DISSOLVE_DS);
            if let Some(mut look) = world.get_mut::<Appearance>(e) {
                look.state = "dissolve".to_owned();
            }
        }
        world.entity_mut(e).insert(foam);
    }
    restore_air(world);
}

/// Plain foam gone: a unit of each of its chemicals onto whoever is there (Luna `foam/Del`).
fn melt(world: &mut World, foam_entity: Entity, foam: &Foam, at: TilePos) {
    world.entity_mut(foam_entity).despawn();
    if foam.metal > 0 {
        return;
    }
    let mut reagents = Reagents::new(10.0);
    for r in &foam.carried {
        reagents.add(r, 1.0);
    }
    for mob in mobs_near(world, at, 0) {
        touch_all(world, mob, &reagents);
    }
}

fn foamed_metal(world: &mut World, at: TilePos, metal: u8) {
    let state = if metal == 1 { "" } else { "iron" };
    let e = spawn_effect(world, FOAMED_METAL, state, at);
    world.entity_mut(e).insert((Dense, Opaque, FoamedMetal { metal }));
    let was = world.get_resource::<StationAtmos>().map(|a| a.0.cell(at));
    if let Some(was) = was {
        world.resource_mut::<StationAtmos>().0.set_cell(at, Cell::Solid);
        world.get_resource_or_init::<FoamWalls>().0.push((e, at, was));
    }
}

/// Air back where a foamed metal wall went.
fn restore_air(world: &mut World) {
    let walls = std::mem::take(&mut world.get_resource_or_init::<FoamWalls>().0);
    let mut kept = Vec::new();
    for (e, at, was) in walls {
        if world.get::<FoamedMetal>(e).is_some() {
            kept.push((e, at, was));
        } else if let Some(mut atmos) = world.get_resource_mut::<StationAtmos>() {
            atmos.0.set_cell(at, was);
        }
    }
    world.resource_mut::<FoamWalls>().0 = kept;
}

/// Plain foam underfoot: those who walk into it slip, unless their shoes don't (Luna `foam/HasEntered`).
pub fn slip_on_foam(world: &mut World) {
    let entered: Vec<(Entity, TilePos)> = world.resource::<Stepped>().0.clone();
    if entered.is_empty() {
        return;
    }
    let mut foams = world.query::<(&GridPos, &Foam)>();
    let wet: Vec<TilePos> = foams
        .iter(world)
        .filter(|(_, f)| f.metal == 0)
        .map(|(p, _)| p.0)
        .collect();
    let now = world.resource::<WorldTime>().deciseconds();
    for (mob, to) in entered {
        if !wet.contains(&to) || world.get::<Body>(mob).is_none() || crate::slips::no_slip(world, mob) {
            continue;
        }
        world
            .entity_mut(mob)
            .remove::<crate::pulling::Pulling>()
            .insert(Weakened(now + FOAM_SLIP_COUNTS * COUNT_DS));
        tell(world, mob, LocMsg::new("slipped").arg("item", FOAM));
        world
            .resource_mut::<SoundEvents>()
            .play(crate::sound::SLIP, to, 50.0);
    }
}

/// A foamed metal wall hit by hand or with something (Luna `foamedmetal/attack_hand`, `attackby`).
pub fn hit_foamed_metal(world: &mut World, mob: Entity, held: Option<Entity>, wall: Entity) {
    let Some(metal) = world.get::<FoamedMetal>(wall).map(|f| i32::from(f.metal)) else {
        return;
    };
    let chance = match held {
        None if world
            .get::<Mutations>(mob)
            .is_some_and(|m| m.0.contains(&Gene::Hulk)) =>
        {
            100
        }
        None => 75 - metal * 25,
        Some(item) => {
            let force = world.get::<crate::hands::Item>(item).map_or(0.0, |i| i.force);
            (force * 20.0) as i32 - metal * 25
        }
    };
    if world.resource_mut::<GameRng>().prob(chance as f32) {
        let at = world.get::<GridPos>(wall).map(|p| p.0);
        world.entity_mut(wall).despawn();
        restore_air(world);
        tell(world, mob, LocMsg::new("foamed-metal-smashed"));
        if let Some(at) = at {
            let name = world
                .get::<DisplayName>(mob)
                .map(|n| n.0.clone())
                .unwrap_or_default();
            world.resource_mut::<VisibleEvents>().0.push((
                at,
                LocMsg::new("foamed-metal-smashed-seen").arg("name", name),
                Some(mob),
            ));
        }
    } else {
        let id = if held.is_some() {
            "foamed-metal-no-effect"
        } else {
            "foamed-metal-bounce"
        };
        tell(world, mob, LocMsg::new(id));
    }
}

// --- On the skin ---

fn worn(world: &World, mob: Entity, slot: Slot) -> Option<(Entity, String)> {
    let item = world.get::<Equipment>(mob)?.get(slot)?;
    let sprite = world.get::<Appearance>(item)?.sprite.clone();
    Some((item, sprite))
}

/// Masks and hats over the eyes and mouth (Luna's `MASKCOVERSEYES`, `MASKCOVERSMOUTH`).
fn covers_eyes(sprite: &str) -> bool {
    matches!(
        sprite,
        "gas-mask"
            | "space-helmet"
            | "welding-helmet"
            | "riot-helmet"
            | "terror-helmet"
            | "wizard-hardsuit-helmet"
    )
}

fn covers_mouth(sprite: &str) -> bool {
    matches!(
        sprite,
        "gas-mask" | "breath-mask" | "space-helmet" | "terror-helmet" | "wizard-hardsuit-helmet"
    )
}

fn is_human(world: &World, mob: Entity) -> bool {
    world.get::<crate::monkeys::Monkey>(mob).is_none()
}

/// The face can only be disfigured once (Luna `face_dmg`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Disfigured;

/// Every chemical in `reagents` on someone's skin (Luna's `reaction(M, TOUCH)`).
pub fn touch_all(world: &mut World, mob: Entity, reagents: &Reagents) {
    let ids: Vec<String> = reagents.amounts.keys().cloned().collect();
    for id in ids {
        touch(world, mob, &id);
    }
}

/// One chemical on someone's skin (Luna's `reaction_mob` with `TOUCH`).
pub fn touch(world: &mut World, mob: Entity, reagent: &str) {
    match reagent {
        "condensedcapsaicin" => pepper(world, mob),
        "pacid" => acid(world, mob, true),
        "sacid" => acid(world, mob, false),
        _ => {}
    }
}

fn pepper(world: &mut World, mob: Entity) {
    if !is_human(world, mob) {
        return;
    }
    let mask = worn(world, mob, Slot::Mask).map(|(_, s)| s);
    let head = worn(world, mob, Slot::Head).map(|(_, s)| s);
    let glasses = worn(world, mob, Slot::Glasses).is_some();
    let eyes =
        glasses || mask.as_deref().is_some_and(covers_eyes) || head.as_deref().is_some_and(covers_eyes);
    let mouth = mask.as_deref().is_some_and(covers_mouth) || head.as_deref().is_some_and(covers_mouth);
    match (eyes, mouth) {
        (true, true) => {}
        (false, true) => {
            crate::eyes::blur(world, mob, 4.0);
            crate::eyes::blind(world, mob, 1.0);
            knock_down(world, mob, 2.0);
            drop_held(world, mob);
        }
        (true, false) => crate::eyes::blur(world, mob, 3.0),
        (false, false) => {
            crate::eyes::blur(world, mob, 8.0);
            crate::eyes::blind(world, mob, 2.0);
            knock_down(world, mob, 5.0);
            drop_held(world, mob);
        }
    }
}

fn knock_down(world: &mut World, mob: Entity, counts: f32) {
    let until = world.resource::<WorldTime>().deciseconds() + counts * COUNT_DS;
    let until = world.get::<Weakened>(mob).map_or(until, |w| w.0.max(until));
    world.entity_mut(mob).insert(Weakened(until));
}

/// What's in the active hand falls to the floor.
fn drop_held(world: &mut World, mob: Entity) {
    let Some(at) = world.get::<GridPos>(mob).map(|p| p.0) else {
        return;
    };
    let Some(item) = world.get_mut::<Hands>(mob).and_then(|mut hands| {
        let hand = hands.active;
        hands.slot_mut(hand).take()
    }) else {
        return;
    };
    world.entity_mut(item).remove::<HeldBy>().insert(GridPos(at));
}

/// Something worn melts away.
fn melt_worn(world: &mut World, mob: Entity, item: Entity) {
    if let Some(mut equipment) = world.get_mut::<Equipment>(mob) {
        equipment.0.retain(|_, e| *e != item);
    }
    if world.get::<WornBy>(item).is_some() {
        world.entity_mut(item).despawn();
    }
}

/// Polytrinic (`strong`) or sulphuric acid on the skin.
fn acid(world: &mut World, mob: Entity, strong: bool) {
    if !is_human(world, mob) {
        if strong && let Some(mut body) = world.get_mut::<Body>(mob) {
            body.take_damage_spread(0.0, 22.0);
        }
        return;
    }
    let disfigured = world.get::<Disfigured>(mob).is_some();
    if strong && disfigured {
        if let Some(mut body) = world.get_mut::<Body>(mob) {
            body.take_damage_spread(0.0, 22.0);
        }
        return;
    }
    let mask = worn(world, mob, Slot::Mask).map(|(e, _)| e);
    let head = worn(world, mob, Slot::Head).map(|(e, _)| e);
    if strong {
        if let Some(mask) = mask {
            melt_worn(world, mob, mask);
            tell(world, mob, LocMsg::new("acid-mask-melts"));
        }
        if let Some(head) = head {
            melt_worn(world, mob, head);
            tell(world, mob, LocMsg::new("acid-helmet-melts"));
        }
        disfigure(world, mob, 20.0);
        return;
    }
    // Sulphuric acid: a mask or hat takes it instead.
    if let Some(mask) = mask {
        melt_worn(world, mob, mask);
        tell(world, mob, LocMsg::new("acid-mask-protects"));
        return;
    }
    if let Some(head) = head {
        melt_worn(world, mob, head);
        tell(world, mob, LocMsg::new("acid-helmet-protects"));
        return;
    }
    if !disfigured && world.resource_mut::<GameRng>().prob(20.0) {
        disfigure(world, mob, 12.0);
    } else if let Some(mut body) = world.get_mut::<Body>(mob) {
        body.take_damage_spread(0.0, 14.0);
    }
}

fn disfigure(world: &mut World, mob: Entity, brute: f32) {
    if let Some(mut body) = world.get_mut::<Body>(mob) {
        body.take_damage(Zone::Head, brute, 0.0);
    }
    world.entity_mut(mob).insert(Disfigured);
    tell(world, mob, LocMsg::new("acid-disfigured"));
}
