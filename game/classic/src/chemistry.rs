//! Chemistry (Luna's `reagents`): things that hold chemicals, syringes and pills that move them
//! into people, and what they do in the bloodstream. Spec: `docs/reference/chemistry.md`.

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, DisplayName, GridPos, MobStat};

use crate::actions::Outbox;
use crate::combat::VisibleEvents;
use crate::flashes::Weakened;
use crate::food::Nutrition;
use crate::hands::Hands;
use crate::health::{Body, Vitals, Zone};
use crate::machines::MACHINE_TICKS;
use crate::time::WorldTime;

pub const SYRINGE: &str = "syringe";
/// How much of each reagent the body uses up each 2 s (Luna `REAGENTS_METABOLISM`).
pub const METABOLISM: f32 = 0.4;
/// What a syringe moves at a time, and a beaker or bottle pours, units.
pub const SYRINGE_DOSE: f32 = 5.0;
pub const POUR: f32 = 10.0;
/// What a hypospray gives at a time, units.
pub const HYPO_DOSE: f32 = 5.0;
/// What the bloodstream can hold, units (Luna `reagents = new(1000)`).
pub const BLOODSTREAM: f32 = 1000.0;
/// How long injecting someone else takes, ds (Luna `do_after(user, 30)`).
pub const INJECT_DS: f32 = 30.0;

/// Chemicals in something: a beaker, or someone's blood.
#[derive(Component, Debug, Clone, PartialEq, Default)]
pub struct Reagents {
    pub max: f32,
    pub amounts: BTreeMap<String, f32>,
}

impl Reagents {
    pub fn new(max: f32) -> Self {
        Self {
            max,
            amounts: BTreeMap::new(),
        }
    }

    pub fn with(max: f32, reagent: &str, amount: f32) -> Self {
        let mut r = Self::new(max);
        r.add(reagent, amount);
        r
    }

    pub fn total(&self) -> f32 {
        self.amounts.values().sum()
    }

    pub fn get(&self, reagent: &str) -> f32 {
        self.amounts.get(reagent).copied().unwrap_or(0.0)
    }

    /// Add as much as fits; how much went in.
    pub fn add(&mut self, reagent: &str, amount: f32) -> f32 {
        let room = (self.max - self.total()).max(0.0);
        let amount = amount.min(room);
        if amount > 0.0 {
            *self.amounts.entry(reagent.to_owned()).or_insert(0.0) += amount;
        }
        amount
    }

    pub fn remove(&mut self, reagent: &str, amount: f32) {
        if let Some(have) = self.amounts.get_mut(reagent) {
            *have -= amount;
            if *have <= 0.001 {
                self.amounts.remove(reagent);
            }
        }
    }

    /// Luna `trans_to`: an even share of everything, up to `amount`. What was taken.
    pub fn take(&mut self, amount: f32) -> Vec<(String, f32)> {
        let total = self.total();
        if total <= 0.0 {
            return Vec::new();
        }
        let share = (amount / total).min(1.0);
        let taken: Vec<(String, f32)> = self.amounts.iter().map(|(r, a)| (r.clone(), a * share)).collect();
        for (r, a) in &taken {
            self.remove(r, *a);
        }
        taken
    }
}

/// What mixes into what (Luna's `chemical_reaction`s for the medicines we have): what it takes,
/// in parts, what it makes and how much for those parts.
const RECIPES: [(&[(&str, f32)], &str, f32); 10] = [
    (
        &[("oxygen", 1.0), ("carbon", 1.0), ("sugar", 1.0)],
        "inaprovaline",
        3.0,
    ),
    (
        &[("silicon", 1.0), ("potassium", 1.0), ("nitrogen", 1.0)],
        "anti_toxin",
        3.0,
    ),
    (&[("silicon", 1.0), ("carbon", 1.0)], "kelotane", 2.0),
    (&[("inaprovaline", 1.0), ("carbon", 1.0)], "bicaridine", 2.0),
    (
        &[("inaprovaline", 1.0), ("anti_toxin", 1.0)],
        "tricordrazine",
        2.0,
    ),
    (
        &[("oxygen", 1.0), ("phosphorus", 1.0), ("kelotane", 1.0)],
        "dermaline",
        3.0,
    ),
    (&[("oxygen", 2.0), ("plasma", 0.1)], "dexalin", 1.0),
    (
        &[("dexalin", 1.0), ("carbon", 1.0), ("iron", 1.0)],
        "dexalinp",
        3.0,
    ),
    (
        &[("sugar", 1.0), ("lithium", 1.0), ("water", 1.0)],
        "synaptizine",
        3.0,
    ),
    (
        &[("dexalin", 1.0), ("water", 1.0), ("oxygen", 1.0)],
        "cryoxadone",
        3.0,
    ),
];

/// Luna `handle_reactions`: mix whatever can mix, until nothing more does.
pub fn react(reagents: &mut Reagents) {
    for _ in 0..RECIPES.len() {
        let mut any = false;
        for (needs, makes, yields) in RECIPES {
            let batches = needs
                .iter()
                .map(|(r, parts)| reagents.get(r) / parts)
                .fold(f32::INFINITY, f32::min);
            if batches <= 0.001 {
                continue;
            }
            for (r, parts) in needs {
                reagents.remove(r, parts * batches);
            }
            *reagents.amounts.entry(makes.to_owned()).or_insert(0.0) += yields * batches;
            any = true;
        }
        if !any {
            break;
        }
    }
}

/// A syringe: it draws or it injects (Luna `mode`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Syringe {
    pub injecting: bool,
}

/// Someone injecting someone else: done at this time, if both stay put.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Injecting {
    pub target: Entity,
    pub syringe: Entity,
    pub done_at: f32,
    pub from: rs13_world::TilePos,
    pub target_from: rs13_world::TilePos,
}

/// What holds chemicals, by its prototype: its size and what it starts with.
fn holds(proto: &str) -> Option<(f32, &'static [(&'static str, f32)])> {
    Some(match proto {
        "beaker" => (50.0, &[]),
        "beaker-large" => (100.0, &[]),
        "bottle-inaprovaline" => (30.0, &[("inaprovaline", 30.0)]),
        "bottle-antitoxin" => (30.0, &[("anti_toxin", 30.0)]),
        "bottle-toxin" => (30.0, &[("toxin", 30.0)]),
        "bottle-stoxin" => (30.0, &[("stoxin", 30.0)]),
        "bottle-made" => (30.0, &[]),
        "pill-made" => (100.0, &[]),
        "pill-inaprovaline" => (100.0, &[("inaprovaline", 30.0)]),
        "pill-antitox" => (100.0, &[("anti_toxin", 25.0)]),
        "pill-kelotane" => (100.0, &[("kelotane", 30.0)]),
        "pill-gold" => (100.0, &[("gold", 20.0)]),
        "pill-silver" => (100.0, &[("silver", 20.0)]),
        "syringe" => (15.0, &[]),
        "syringe-inaprovaline" => (15.0, &[("inaprovaline", 15.0)]),
        "syringe-antitoxin" => (15.0, &[("anti_toxin", 15.0)]),
        "hypospray" => (30.0, &[("tricordrazine", 30.0)]),
        "dropper" => (5.0, &[]),
        "bucket" => (70.0, &[]),
        _ => return None,
    })
}

pub fn is_syringe(proto: &str) -> bool {
    proto == SYRINGE || proto.starts_with("syringe-")
}

pub fn is_pill(proto: &str) -> bool {
    proto.starts_with("pill-")
}

pub fn is_hypo(proto: &str) -> bool {
    proto == "hypospray"
}

/// Beakers, bottles and buckets, which pour.
pub fn is_glass(proto: &str) -> bool {
    proto.starts_with("beaker") || proto.starts_with("bottle-") || proto == "bucket"
}

pub fn is_chem_tool(proto: &str) -> bool {
    is_syringe(proto) || is_pill(proto) || is_hypo(proto) || is_glass(proto)
}

/// Fit the components a new container needs.
pub fn fit(entity: &mut EntityWorldMut, proto: &str) {
    let Some((max, start)) = holds(proto) else {
        return;
    };
    let mut reagents = Reagents::new(max);
    for (reagent, amount) in start {
        reagents.add(reagent, *amount);
    }
    let filled = reagents.total();
    entity.insert(reagents);
    if is_syringe(proto) {
        let injecting = filled > 0.0;
        entity.insert((
            Syringe { injecting },
            Appearance::new(proto, syringe_look(injecting, filled)),
        ));
    }
}

/// Luna `syringe/update_icon`: drawing or injecting, by the five units.
fn syringe_look(injecting: bool, filled: f32) -> String {
    let fifths = ((filled / 5.0).ceil() * 5.0) as i32;
    format!("{}{fifths}", if injecting { "i" } else { "d" })
}

fn tell(world: &mut World, mob: Entity, msg: LocMsg) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(msg);
    }
}

fn update_syringe(world: &mut World, syringe: Entity) {
    let Some(filled) = world.get::<Reagents>(syringe).map(Reagents::total) else {
        return;
    };
    let Some(state) = world.get::<Syringe>(syringe).copied() else {
        return;
    };
    if let Some(mut look) = world.get_mut::<Appearance>(syringe) {
        look.state = syringe_look(state.injecting, filled);
    }
}

/// Move up to `amount` from one holder to another; how much went.
pub fn transfer(world: &mut World, from: Entity, to: Entity, amount: f32) -> f32 {
    let room = world
        .get::<Reagents>(to)
        .map_or(0.0, |r| (r.max - r.total()).max(0.0));
    let amount = amount.min(room);
    if amount <= 0.0 {
        return 0.0;
    }
    let Some(taken) = world.get_mut::<Reagents>(from).map(|mut r| r.take(amount)) else {
        return 0.0;
    };
    let moved: f32 = taken.iter().map(|(_, a)| a).sum();
    if let Some(mut into) = world.get_mut::<Reagents>(to) {
        for (r, a) in &taken {
            into.add(r, *a);
        }
        react(&mut into);
    }
    moved
}

fn name_of(world: &World, who: Entity) -> String {
    world
        .get::<DisplayName>(who)
        .map(|n| n.0.clone())
        .unwrap_or_default()
}

/// A click with a syringe (Luna `syringe/afterattack`, `attack_self`).
pub fn use_syringe(world: &mut World, mob: Entity, syringe: Entity, target: Entity) {
    if target == syringe {
        if let Some(mut s) = world.get_mut::<Syringe>(syringe) {
            s.injecting = !s.injecting;
        }
        update_syringe(world, syringe);
        return;
    }
    let injecting = world.get::<Syringe>(syringe).is_some_and(|s| s.injecting);
    let is_person = world.get::<Body>(target).is_some();
    if !injecting {
        if is_person {
            tell(world, mob, LocMsg::new("syringe-no-blood"));
            return;
        }
        if world.get::<Reagents>(target).is_none() {
            return;
        }
        let moved = transfer(world, target, syringe, SYRINGE_DOSE);
        let id = if moved > 0.0 {
            "syringe-drawn"
        } else {
            "syringe-nothing"
        };
        tell(world, mob, LocMsg::new(id).num("value", moved.round() as i64));
        if world.get::<Reagents>(syringe).is_some_and(|r| r.total() >= r.max)
            && let Some(mut s) = world.get_mut::<Syringe>(syringe)
        {
            s.injecting = true;
        }
    } else if world.get::<Reagents>(syringe).is_some_and(|r| r.total() <= 0.0) {
        tell(world, mob, LocMsg::new("syringe-empty"));
    } else if is_person && target != mob {
        let (Some(from), Some(target_from)) = (
            world.get::<GridPos>(mob).map(|p| p.0),
            world.get::<GridPos>(target).map(|p| p.0),
        ) else {
            return;
        };
        let now = world.resource::<WorldTime>().deciseconds();
        world.entity_mut(mob).insert(Injecting {
            target,
            syringe,
            done_at: now + INJECT_DS,
            from,
            target_from,
        });
        let msg = LocMsg::new("syringe-injecting")
            .arg("name", name_of(world, mob))
            .arg("victim", name_of(world, target));
        world.resource_mut::<VisibleEvents>().0.push((from, msg, None));
    } else if world.get::<Reagents>(target).is_some() {
        let moved = transfer(world, syringe, target, SYRINGE_DOSE);
        tell(
            world,
            mob,
            LocMsg::new("syringe-injected").num("value", moved.round() as i64),
        );
        if world.get::<Reagents>(syringe).is_some_and(|r| r.total() <= 0.0)
            && let Some(mut s) = world.get_mut::<Syringe>(syringe)
        {
            s.injecting = false;
        }
    }
    update_syringe(world, syringe);
}

/// A click with a pill: swallowed whole, by you or whoever you feed it to (Luna `pill/attack`).
pub fn use_pill(world: &mut World, mob: Entity, pill: Entity, target: Entity) {
    if world.get::<Body>(target).is_none() {
        return;
    }
    if let Some(mut hands) = world.get_mut::<Hands>(mob) {
        let active = hands.active;
        if hands.get(active) == Some(pill) {
            *hands.slot_mut(active) = None;
        }
    }
    transfer(world, pill, target, 1000.0);
    world.entity_mut(pill).despawn();
    let id = if target == mob {
        "pill-swallowed"
    } else {
        "pill-fed"
    };
    let msg = LocMsg::new(id).arg("victim", name_of(world, target));
    tell(world, mob, msg);
}

/// A click with a hypospray: into someone, at once (Luna `hypospray/attack`).
pub fn use_hypo(world: &mut World, mob: Entity, hypo: Entity, target: Entity) {
    if world.get::<Body>(target).is_none() {
        return;
    }
    let moved = transfer(world, hypo, target, HYPO_DOSE);
    let id = if moved > 0.0 {
        "hypo-injected"
    } else {
        "syringe-empty"
    };
    tell(world, mob, LocMsg::new(id).num("value", moved.round() as i64));
}

/// A click with a beaker or bottle on another holder: it pours (Luna `glass/afterattack`).
pub fn pour(world: &mut World, mob: Entity, glass: Entity, target: Entity) {
    if world.get::<Reagents>(target).is_none() || world.get::<Body>(target).is_some() {
        return;
    }
    let moved = transfer(world, glass, target, POUR);
    let id = if moved > 0.0 {
        "glass-poured"
    } else {
        "glass-nothing"
    };
    tell(world, mob, LocMsg::new(id).num("value", moved.round() as i64));
}

/// What looking at a holder tells (Luna's "It contains N units").
pub fn examine(world: &World, target: Entity) -> Option<LocMsg> {
    let reagents = world.get::<Reagents>(target)?;
    let total = reagents.total();
    Some(if total > 0.0 {
        LocMsg::new("reagents-contains").num("value", total.round() as i64)
    } else {
        LocMsg::new("reagents-empty")
    })
}

/// Heal this much in all, from the worst hurt parts first (Luna `heal_organ_damage`).
fn heal_parts(body: &mut Body, mut brute: f32, mut burn: f32) {
    for zone in Zone::ALL {
        let organ = body.organ(zone);
        let (b, f) = (organ.brute.min(brute), organ.burn.min(burn));
        if b > 0.0 || f > 0.0 {
            body.heal(zone, b, f);
            brute -= b;
            burn -= f;
        }
    }
}

/// What one reagent does in someone's blood this cycle (Luna's `on_mob_life`).
fn effect(
    reagent: &str,
    body: &mut Body,
    blood: &mut Vec<(String, f32)>,
    sedate: &mut bool,
    wake: &mut bool,
    fed: &mut f32,
) {
    match reagent {
        "anti_toxin" => {
            body.tox = (body.tox - 2.0).max(0.0);
            blood.push(("toxin".to_owned(), 2.0));
            blood.push(("stoxin".to_owned(), 2.0));
        }
        "kelotane" => heal_parts(body, 0.0, 2.0),
        "dermaline" => heal_parts(body, 0.0, 3.0),
        "bicaridine" => heal_parts(body, 1.0, 0.0),
        "dexalin" => body.oxy = (body.oxy - 3.0).max(0.0),
        "dexalinp" => body.oxy = (body.oxy - (body.oxy * 0.5).max(4.0)).max(0.0),
        "tricordrazine" => {
            body.oxy = (body.oxy - 1.0).max(0.0);
            body.tox = (body.tox - 1.0).max(0.0);
            heal_parts(body, 1.0, 1.0);
        }
        "synaptizine" => *wake = true,
        "toxin" => body.tox += 2.0,
        // Luna's `lexorin`: the changeling's death sting.
        "lexorin" => body.oxy += 3.0,
        "stoxin" => *sedate = true,
        "sugar" => *fed += 1.0,
        "nutriment" => *fed += 6.0,
        _ => {}
    }
}

/// Every 2 s: what's in people's blood takes effect and is used up, and injections finish. The
/// dead don't metabolise (Luna `handle_chemicals_in_body`).
pub fn run_chemistry(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    let mut workers = world.query::<(Entity, &Injecting, &GridPos)>();
    let jobs: Vec<(Entity, Injecting, rs13_world::TilePos)> =
        workers.iter(world).map(|(e, j, p)| (e, *j, p.0)).collect();
    for (mob, job, at) in jobs {
        let target_at = world.get::<GridPos>(job.target).map(|p| p.0);
        if at != job.from || target_at != Some(job.target_from) {
            world.entity_mut(mob).remove::<Injecting>();
            continue;
        }
        if now < job.done_at {
            continue;
        }
        world.entity_mut(mob).remove::<Injecting>();
        let moved = transfer(world, job.syringe, job.target, SYRINGE_DOSE);
        tell(
            world,
            mob,
            LocMsg::new("syringe-injected").num("value", moved.round() as i64),
        );
        if world
            .get::<Reagents>(job.syringe)
            .is_some_and(|r| r.total() <= 0.0)
            && let Some(mut s) = world.get_mut::<Syringe>(job.syringe)
        {
            s.injecting = false;
        }
        update_syringe(world, job.syringe);
    }
    if !world.resource::<WorldTime>().tick.0.is_multiple_of(MACHINE_TICKS) {
        return;
    }
    let mut mobs = world.query::<(
        Entity,
        &mut Reagents,
        &mut Body,
        Option<&Vitals>,
        Option<&mut Nutrition>,
    )>();
    let mut wake_up: Vec<Entity> = Vec::new();
    let mut sedated: Vec<Entity> = Vec::new();
    for (mob, mut blood, mut body, vitals, nutrition) in mobs.iter_mut(world) {
        if blood.amounts.is_empty() || vitals.is_some_and(|v| v.stat == MobStat::Dead) {
            continue;
        }
        let names: Vec<String> = blood.amounts.keys().cloned().collect();
        let mut cleared: Vec<(String, f32)> = Vec::new();
        let (mut sedate, mut wake, mut fed) = (false, false, 0.0);
        for name in &names {
            effect(name, &mut body, &mut cleared, &mut sedate, &mut wake, &mut fed);
            blood.remove(name, METABOLISM);
        }
        for (name, amount) in cleared {
            blood.remove(&name, amount);
        }
        if let Some(mut n) = nutrition {
            n.level += fed;
        }
        if sedate {
            sedated.push(mob);
        }
        if wake {
            wake_up.push(mob);
        }
    }
    for mob in wake_up {
        world.entity_mut(mob).remove::<Weakened>();
    }
    for mob in sedated {
        world.entity_mut(mob).insert(Weakened(now + SEDATE_DS));
    }
}

/// How long a dose of sleep toxin keeps you down past each cycle it is in you, ds.
const SEDATE_DS: f32 = 40.0;
