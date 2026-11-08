//! Changelings: absorb the crew, wear their faces, evolve powers with what you take (Luna's
//! `gamemodes/changeling`). Spec: `docs/reference/changeling.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{
    Appearance, ChangelingPanelView, ChangelingPower, DisplayName, GridPos, HumanLook, MobStat, NetId,
    TilePos,
};

use crate::actions::Outbox;
use crate::chemistry::Reagents;
use crate::combat::{Muted, Paralyzed, VisibleEvents};
use crate::flashes::Weakened;
use crate::grabs::Grab;
use crate::hands::Hands;
use crate::health::{Body, Vitals};
use crate::movement::Mob;
use crate::objects::PlayerControlled;
use crate::rng::GameRng;
use crate::round::{Phase, Round};
use crate::time::WorldTime;
use crate::traitor::{Objective, WantsTraitor, achieved, select_changeling_objectives};

pub const MODE: &str = "changeling";
/// What a changeling starts with (Luna's `datum/changeling`).
const START_CHEMS: f32 = 20.0;
const START_STORAGE: f32 = 50.0;
const START_RATE: f32 = 0.5;
const START_POINTS: i32 = 5;
/// How often chemicals and genes recover, ds: a life tick.
const LIFE_TICK_DS: f32 = 20.0;
/// How long each of absorbing's three stages takes, ds.
const ABSORB_STAGE_DS: f32 = 150.0;
/// How long a dead changeling leaves the round undecided, ds: Luna's 600 ticks of 2 s.
const DEAD_ENDS_ROUND_DS: f32 = 12_000.0;
/// How long until stasis wakes, between these, ds.
const STASIS_MIN_DS: i32 = 800;
const STASIS_MAX_DS: i32 = 2000;
/// How long a mimicked voice or a digital camouflage takes to use up a unit, ds.
const UPKEEP_DS: f32 = 40.0;
/// How long a sting's deafness lasts, ds, and its silence, per Luna's life ticks of 2 s.
const DEAF_DS: f32 = 300.0;
const SILENCE_DS: f32 = 1000.0;
const DEATH_SILENCE_DS: f32 = 400.0;
const DEATH_PARALYSIS_DS: f32 = 200.0;
const PARALYSIS_STING_DS: f32 = 400.0;
const GREEK: [&str; 24] = [
    "Alpha", "Beta", "Gamma", "Delta", "Epsilon", "Zeta", "Eta", "Theta", "Iota", "Kappa", "Lambda", "Mu",
    "Nu", "Xi", "Omicron", "Pi", "Rho", "Sigma", "Tau", "Upsilon", "Phi", "Chi", "Psi", "Omega",
];

/// A power: its id, what it costs in genome points, and what it costs to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerDef {
    pub id: &'static str,
    pub cost: u8,
    pub chems: u8,
    /// Who or what it's used on: `Target` someone near, `Dna` one of ours, or `None`.
    pub aim: Aim,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aim {
    None,
    Target,
    Dna,
    TargetDna,
    Hive,
    /// Bought and working from then on.
    Passive,
    /// Takes a name from the chat line.
    Text,
}

const fn power(id: &'static str, cost: u8, chems: u8, aim: Aim) -> PowerDef {
    PowerDef { id, cost, chems, aim }
}

/// Luna's powers, in the order of its list. Its Hallucination Sting is missing: nothing here hallucinates.
pub const POWERS: [PowerDef; 22] = [
    power("absorb", 0, 0, Aim::None),
    power("transform", 0, 5, Aim::Dna),
    power("stasis", 0, 20, Aim::None),
    power("hive-channel", 0, 10, Aim::Dna),
    power("hive-absorb", 0, 20, Aim::Hive),
    power("lesser-form", 1, 1, Aim::None),
    power("panacea", 1, 25, Aim::None),
    power("deaf-sting", 1, 5, Aim::Target),
    power("blind-sting", 2, 20, Aim::Target),
    power("silence-sting", 2, 10, Aim::Target),
    power("mimic-voice", 3, 0, Aim::Text),
    power("extract-dna", 4, 40, Aim::Target),
    power("transformation-sting", 3, 40, Aim::TargetDna),
    power("paralysis-sting", 3, 30, Aim::Target),
    power("death-sting", 10, 40, Aim::Target),
    power("boost-range", 2, 10, Aim::None),
    power("epinephrine", 4, 45, Aim::None),
    power("chem-synth", 4, 0, Aim::Passive),
    power("adv-chem-synth", 8, 0, Aim::Passive),
    power("engorged-glands", 4, 0, Aim::Passive),
    power("digital-camo", 3, 0, Aim::None),
    power("rapid-regen", 8, 30, Aim::None),
];

fn def(line: usize) -> Option<PowerDef> {
    POWERS.get(line).copied()
}

/// One genome: who they were, and how they looked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dna {
    pub name: String,
    pub look: HumanLook,
}

/// Being absorbed, by stages.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Absorbing {
    pub victim: Entity,
    pub stage: u8,
    pub until: f32,
    from: TilePos,
    their_tile: TilePos,
}

#[derive(Component, Debug, Clone, PartialEq)]
pub struct Changeling {
    pub dna: Vec<Dna>,
    pub absorbed: u32,
    pub chems: f32,
    pub storage: f32,
    pub rate: f32,
    pub sting_range: i32,
    pub id_name: String,
    pub genetic_damage: f32,
    pub points: i32,
    pub powers: Vec<&'static str>,
    pub mimic: Option<String>,
    pub camo: bool,
    pub absorbing: Option<Absorbing>,
    /// In stasis: when they wake.
    pub stasis_until: Option<f32>,
    pub lesser: bool,
    pub objectives: Vec<Objective>,
    pub regen_left: u8,
    next_life: f32,
    next_upkeep: f32,
    dead_since: Option<f32>,
}

impl Changeling {
    fn new(id_name: String) -> Self {
        Self {
            dna: Vec::new(),
            absorbed: 0,
            chems: START_CHEMS,
            storage: START_STORAGE,
            rate: START_RATE,
            sting_range: 1,
            id_name,
            genetic_damage: 0.0,
            points: START_POINTS,
            powers: POWERS.iter().filter(|p| p.cost == 0).map(|p| p.id).collect(),
            mimic: None,
            camo: false,
            absorbing: None,
            stasis_until: None,
            lesser: false,
            objectives: Vec::new(),
            regen_left: 0,
            next_life: 0.0,
            next_upkeep: 0.0,
            dead_since: None,
        }
    }

    pub fn has(&self, id: &str) -> bool {
        self.powers.contains(&id)
    }
}

/// A sting's deafness, until this time, ds.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Deafened(pub f32);

/// A sting's silence, until this time, ds.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Silenced(pub f32);

/// A drained husk: unclonable (Luna's `HUSK`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Husked;

/// Has the changeling's window open.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LingViewer;

/// What their window shows.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct LingPanelOut(pub Option<ChangelingPanelView>);

/// DNA on the airwaves, and whether the round's been decided.
#[derive(Resource, Debug, Default, Clone, PartialEq)]
pub struct LingState {
    pub hive: Vec<Dna>,
    pub dead_end: bool,
}

/// What's been said on the hive channel this tick.
#[derive(Resource, Debug, Default)]
pub struct HiveLines(pub Vec<LocMsg>);

/// What's asked of the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LingOp {
    Open,
    Buy(u8),
    Use {
        power: u8,
        target: Option<Entity>,
        dna: Option<u8>,
    },
    Mimic(String),
    GiveUp,
}

fn tell(world: &mut World, mob: Entity, msg: LocMsg) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(msg);
    }
}

fn say_around(world: &mut World, at: TilePos, msg: LocMsg) {
    world.resource_mut::<VisibleEvents>().0.push((at, msg, None));
}

fn name_of(world: &World, mob: Entity) -> String {
    world
        .get::<DisplayName>(mob)
        .map_or_else(String::new, |name| name.0.clone())
}

fn dna_of(world: &World, mob: Entity) -> Dna {
    Dna {
        name: name_of(world, mob),
        look: world.get::<HumanLook>(mob).copied().unwrap_or_default(),
    }
}

/// Become a changeling, as one (Luna's `make_changeling`).
pub fn make_changeling(world: &mut World, mob: Entity) {
    let look = world.get::<HumanLook>(mob).copied().unwrap_or_default();
    let i = world.resource_mut::<GameRng>().range(0, GREEK.len() as i32 - 1) as usize;
    let honorific = if look.female { "Ms." } else { "Mr." };
    let mut ling = Changeling::new(format!("{honorific} {}", GREEK[i]));
    ling.dna.push(dna_of(world, mob));
    world.entity_mut(mob).insert(ling);
}

/// Pick the changeling and brief them (Luna's `post_setup`). False if there's nobody to pick.
pub fn setup_round(world: &mut World) -> bool {
    let mut crew = world.query_filtered::<(
        Entity,
        Option<&crate::jobs::HasJob>,
        Option<&DisplayName>,
        Has<WantsTraitor>,
    ), With<PlayerControlled>>();
    let mut all: Vec<(Entity, String, String, bool)> = crew
        .iter(world)
        .map(|(e, j, n, w)| {
            (
                e,
                n.map_or_else(String::new, |n| n.0.clone()),
                j.map_or_else(String::new, |j| j.0.clone()),
                w,
            )
        })
        .collect();
    all.sort_by_key(|(e, ..)| *e);
    if all.is_empty() {
        return false;
    }
    let wanting: Vec<Entity> = all.iter().filter(|(.., w)| *w).map(|(e, ..)| *e).collect();
    let pool: Vec<Entity> = if wanting.is_empty() {
        all.iter().map(|(e, ..)| *e).collect()
    } else {
        wanting
    };
    let i = world.resource_mut::<GameRng>().range(0, pool.len() as i32 - 1) as usize;
    let mob = pool[i];
    make_changeling(world, mob);
    let others: Vec<(Entity, String, String)> = all
        .iter()
        .filter(|(e, ..)| *e != mob)
        .map(|(e, n, j, _)| (*e, n.clone(), j.clone()))
        .collect();
    let objectives = {
        let mut rng = world.resource_mut::<GameRng>();
        select_changeling_objectives(&mut rng, &others)
    };
    tell(world, mob, LocMsg::new("ling-you"));
    tell(world, mob, LocMsg::new("ling-howto"));
    for (i, objective) in objectives.iter().enumerate() {
        tell(world, mob, objective.line(i + 1, "none"));
    }
    if let Some(mut ling) = world.get_mut::<Changeling>(mob) {
        ling.objectives = objectives;
    }
    true
}

fn is_alive(world: &World, mob: Entity) -> bool {
    world.get::<Vitals>(mob).is_some_and(|v| v.stat != MobStat::Dead)
}

fn stat_of(world: &World, mob: Entity) -> MobStat {
    world.get::<Vitals>(mob).map_or(MobStat::Conscious, |v| v.stat)
}

fn require(world: &mut World, mob: Entity, power: &PowerDef) -> bool {
    let Some(ling) = world.get::<Changeling>(mob) else {
        return false;
    };
    let (chems, damage) = (ling.chems, ling.genetic_damage);
    let stat = stat_of(world, mob);
    // What's allowed when, as Luna's `changeling_power` calls.
    let works_out_cold = matches!(power.id, "stasis" | "epinephrine" | "rapid-regen" | "panacea");
    let works_dead = power.id == "stasis";
    let able = match stat {
        MobStat::Conscious => true,
        MobStat::Unconscious => works_out_cold,
        MobStat::Dead => works_dead,
        MobStat::Ghost => false,
    };
    if !able {
        tell(world, mob, LocMsg::new("ling-incapacitated"));
        return false;
    }
    if chems < f32::from(power.chems) {
        tell(
            world,
            mob,
            LocMsg::new("ling-need-chems").num("value", i64::from(power.chems)),
        );
        return false;
    }
    // Only changing form minds the state of their genes.
    if matches!(power.id, "transform" | "lesser-form") && damage > 0.0 {
        tell(world, mob, LocMsg::new("ling-genes"));
        return false;
    }
    true
}

fn spend(world: &mut World, mob: Entity, amount: u8) {
    if let Some(mut ling) = world.get_mut::<Changeling>(mob) {
        ling.chems = (ling.chems - f32::from(amount)).max(0.0);
    }
}

/// Look like someone: their name and looks.
fn wear_dna(world: &mut World, mob: Entity, dna: &Dna) {
    let sprite = if dna.look.female { "human-f" } else { "human" };
    if let Some(mut look) = world.get_mut::<Appearance>(mob) {
        look.sprite = sprite.to_owned();
    }
    world
        .entity_mut(mob)
        .insert((dna.look, DisplayName(dna.name.clone())));
}

fn start_absorbing(world: &mut World, mob: Entity) {
    let grab = world
        .get::<Hands>(mob)
        .and_then(Hands::active_item)
        .and_then(|item| world.get::<Grab>(item).map(|grab| grab.victim));
    let Some(victim) = grab else {
        tell(world, mob, LocMsg::new("ling-need-grab"));
        return;
    };
    if world.get::<HumanLook>(victim).is_none()
        && world
            .get::<Appearance>(victim)
            .is_none_or(|a| !a.sprite.starts_with("human"))
    {
        tell(world, mob, LocMsg::new("ling-incompatible"));
        return;
    }
    if world.get::<Husked>(victim).is_some() {
        tell(world, mob, LocMsg::new("ling-ruined"));
        return;
    }
    if world
        .get::<Changeling>(mob)
        .is_some_and(|l| l.absorbing.is_some())
    {
        tell(world, mob, LocMsg::new("ling-busy"));
        return;
    }
    let (Some(from), Some(their_tile)) = (
        world.get::<GridPos>(mob).map(|p| p.0),
        world.get::<GridPos>(victim).map(|p| p.0),
    ) else {
        return;
    };
    let now = world.resource::<WorldTime>().deciseconds();
    tell(world, mob, LocMsg::new("ling-absorb-1"));
    if let Some(mut ling) = world.get_mut::<Changeling>(mob) {
        ling.absorbing = Some(Absorbing {
            victim,
            stage: 1,
            until: now + ABSORB_STAGE_DS,
            from,
            their_tile,
        });
    }
}

/// The end of absorbing: their genes are ours, and they're a husk.
fn finish_absorbing(world: &mut World, mob: Entity, victim: Entity) {
    let dna = dna_of(world, victim);
    let theirs = world.get::<Changeling>(victim).cloned();
    if let Some(mut ling) = world.get_mut::<Changeling>(mob) {
        if !ling.dna.contains(&dna) {
            ling.dna.push(dna.clone());
        }
        ling.chems += 10.0;
        ling.points += 2;
        ling.absorbed += 1;
        // Another changeling: everything they had (Luna's "steal all their loot").
        if let Some(theirs) = &theirs {
            for genome in &theirs.dna {
                if !ling.dna.contains(genome) {
                    ling.dna.push(genome.clone());
                    ling.absorbed += 1;
                }
            }
            for power in &theirs.powers {
                if !ling.powers.contains(power) {
                    ling.powers.push(*power);
                    match *power {
                        "chem-synth" | "adv-chem-synth" => ling.rate *= 2.0,
                        "engorged-glands" => ling.storage += 25.0,
                        _ => {}
                    }
                }
            }
            ling.chems += theirs.chems;
            ling.points += theirs.points;
        }
    }
    if theirs.is_some() {
        world.entity_mut(victim).remove::<Changeling>();
    }
    let name = name_of(world, victim);
    tell(world, mob, LocMsg::new("ling-absorbed").arg("name", name.clone()));
    tell(world, victim, LocMsg::new("ling-you-absorbed"));
    if let Some(at) = world.get::<GridPos>(mob).map(|p| p.0) {
        let msg = LocMsg::new("ling-sucks")
            .arg("name", name_of(world, mob))
            .arg("victim", name);
        say_around(world, at, msg);
    }
    if let Some(mut vitals) = world.get_mut::<Vitals>(victim) {
        vitals.stat = MobStat::Dead;
    }
    let look = HumanLook {
        skin: -120,
        hair: 0,
        facial: 0,
        ..world.get::<HumanLook>(victim).copied().unwrap_or_default()
    };
    world.entity_mut(victim).insert((Husked, look));
}

/// Knocked out, chem-free and all: the full heal of stasis (Luna's `changeling_fakedeath`).
fn wake_from_stasis(world: &mut World, mob: Entity) {
    let enough = world.get::<Changeling>(mob).is_some_and(|l| l.chems >= 20.0);
    if !enough {
        tell(world, mob, LocMsg::new("ling-stasis-failed"));
        return;
    }
    spend(world, mob, 20);
    world
        .entity_mut(mob)
        .insert(Body::default())
        .remove::<(Paralyzed, Weakened)>();
    if let Some(mut vitals) = world.get_mut::<Vitals>(mob) {
        vitals.stat = MobStat::Conscious;
    }
    if let Some(mut blood) = world.get_mut::<Reagents>(mob) {
        blood.amounts.clear();
    }
    tell(world, mob, LocMsg::new("ling-regenerated"));
    if let Some(at) = world.get::<GridPos>(mob).map(|p| p.0) {
        let msg = LocMsg::new("ling-wakes").arg("name", name_of(world, mob));
        say_around(world, at, msg);
    }
}

fn near_enough(world: &World, mob: Entity, target: Entity, range: i32) -> bool {
    match (world.get::<GridPos>(mob), world.get::<GridPos>(target)) {
        (Some(a), Some(b)) => a.0.z == b.0.z && (a.0.x - b.0.x).abs().max((a.0.y - b.0.y).abs()) <= range,
        _ => false,
    }
}

/// A sting's chems and the checks all stings share. True if the target's affected.
fn sting(world: &mut World, mob: Entity, power: &PowerDef, target: Entity) -> bool {
    let range = world.get::<Changeling>(mob).map_or(1, |l| l.sting_range);
    if !near_enough(world, mob, target, range) || target == mob {
        return false;
    }
    spend(world, mob, power.chems);
    if let Some(mut ling) = world.get_mut::<Changeling>(mob) {
        ling.sting_range = 1;
    }
    let name = name_of(world, target);
    tell(world, mob, LocMsg::new("ling-stung").arg("name", name));
    // Other changelings only feel a prick (Luna's `changeling_sting`).
    if world.get::<Changeling>(target).is_some() {
        tell(world, target, LocMsg::new("ling-prick"));
        return false;
    }
    true
}

fn use_power(world: &mut World, mob: Entity, line: u8, target: Option<Entity>, dna: Option<u8>) {
    let Some(power) = def(usize::from(line)) else {
        return;
    };
    if !world.get::<Changeling>(mob).is_some_and(|l| l.has(power.id)) || !require(world, mob, &power) {
        return;
    }
    let now = world.resource::<WorldTime>().deciseconds();
    let chosen = dna.and_then(|i| {
        world
            .get::<Changeling>(mob)
            .and_then(|l| l.dna.get(usize::from(i)).cloned())
    });
    match power.id {
        "absorb" => start_absorbing(world, mob),
        "transform" => {
            let Some(chosen) = chosen else { return };
            spend(world, mob, power.chems);
            if let Some(mut ling) = world.get_mut::<Changeling>(mob) {
                ling.genetic_damage = 30.0;
                ling.lesser = false;
            }
            wear_dna(world, mob, &chosen);
            if let Some(at) = world.get::<GridPos>(mob).map(|p| p.0) {
                let msg = LocMsg::new("ling-transforms").arg("name", chosen.name);
                say_around(world, at, msg);
            }
        }
        "lesser-form" => {
            spend(world, mob, power.chems);
            crate::jobs::drop_everything(world, mob);
            if let Some(mut ling) = world.get_mut::<Changeling>(mob) {
                ling.genetic_damage = 30.0;
                ling.lesser = true;
            }
            if let Some(mut look) = world.get_mut::<Appearance>(mob) {
                look.sprite = crate::monkeys::MONKEY.to_owned();
            }
            world.entity_mut(mob).remove::<HumanLook>();
            tell(world, mob, LocMsg::new("ling-genes-cry"));
        }
        "stasis" => {
            let wait = world
                .resource_mut::<GameRng>()
                .range(STASIS_MIN_DS, STASIS_MAX_DS) as f32;
            if let Some(mut ling) = world.get_mut::<Changeling>(mob) {
                ling.stasis_until = Some(now + wait);
            }
            tell(world, mob, LocMsg::new("ling-stasis"));
            if is_alive(world, mob) {
                world.entity_mut(mob).insert(Paralyzed(now + wait));
            }
        }
        "hive-channel" => {
            let Some(chosen) = chosen else { return };
            world.init_resource::<LingState>();
            if world.resource::<LingState>().hive.contains(&chosen) {
                tell(world, mob, LocMsg::new("ling-hive-has"));
                return;
            }
            spend(world, mob, power.chems);
            let name = chosen.name.clone();
            world.resource_mut::<LingState>().hive.push(chosen);
            tell(world, mob, LocMsg::new("ling-hive-sent").arg("name", name));
        }
        "hive-absorb" => {
            world.init_resource::<LingState>();
            let bank = world.resource::<LingState>().hive.clone();
            let known = world
                .get::<Changeling>(mob)
                .map_or_else(Vec::new, |l| l.dna.clone());
            let new: Vec<Dna> = bank.into_iter().filter(|d| !known.contains(d)).collect();
            let Some(picked) = new.get(usize::from(dna.unwrap_or(0))).cloned() else {
                tell(world, mob, LocMsg::new("ling-hive-none"));
                return;
            };
            spend(world, mob, power.chems);
            let name = picked.name.clone();
            if let Some(mut ling) = world.get_mut::<Changeling>(mob) {
                ling.dna.push(picked);
            }
            tell(world, mob, LocMsg::new("ling-hive-got").arg("name", name));
        }
        "panacea" => {
            spend(world, mob, power.chems);
            if let Some(mut body) = world.get_mut::<Body>(mob) {
                body.tox = 0.0;
                body.oxy = 0.0;
            }
            tell(world, mob, LocMsg::new("ling-cleansed"));
        }
        "boost-range" => {
            spend(world, mob, power.chems);
            if let Some(mut ling) = world.get_mut::<Changeling>(mob) {
                ling.sting_range = 2;
            }
            tell(world, mob, LocMsg::new("ling-range"));
        }
        "epinephrine" => {
            spend(world, mob, power.chems);
            world.entity_mut(mob).remove::<(Paralyzed, Weakened)>();
            if let Some(mut vitals) = world.get_mut::<Vitals>(mob) {
                vitals.stat = MobStat::Conscious;
            }
            if let Some(mut blood) = world.get_mut::<Reagents>(mob) {
                blood.add("synaptizine", 20.0);
            }
            tell(world, mob, LocMsg::new("ling-arise"));
        }
        "digital-camo" => {
            let on = world.get::<Changeling>(mob).is_some_and(|l| !l.camo);
            if let Some(mut ling) = world.get_mut::<Changeling>(mob) {
                ling.camo = on;
            }
            tell(
                world,
                mob,
                LocMsg::new(if on { "ling-camo-on" } else { "ling-camo-off" }),
            );
        }
        "rapid-regen" => {
            spend(world, mob, power.chems);
            if let Some(mut ling) = world.get_mut::<Changeling>(mob) {
                ling.regen_left = 10;
            }
        }
        "mimic-voice" => {
            // Only ever stops it here: `LingOp::Mimic` starts it, with the name.
            if let Some(mut ling) = world.get_mut::<Changeling>(mob)
                && ling.mimic.take().is_some()
            {
                tell(world, mob, LocMsg::new("ling-voice-back"));
            }
        }
        "deaf-sting"
        | "blind-sting"
        | "silence-sting"
        | "paralysis-sting"
        | "death-sting"
        | "extract-dna"
        | "transformation-sting" => {
            let Some(target) = target else { return };
            if power.id == "transformation-sting" && chosen.is_none() {
                return;
            }
            if !sting(world, mob, &power, target) {
                return;
            }
            match power.id {
                "deaf-sting" => {
                    world.entity_mut(target).insert(Deafened(now + DEAF_DS));
                    tell(world, target, LocMsg::new("ling-ears-pop"));
                }
                "blind-sting" => {
                    // Luna: blind for 20 life ticks, blurred for 40, nearsighted for 30 s.
                    crate::eyes::blind(world, target, 20.0);
                    crate::eyes::blur(world, target, 40.0);
                    crate::eyes::nearsight(world, target, 300.0);
                    tell(world, target, LocMsg::new("ling-eyes-burn"));
                }
                "silence-sting" => {
                    world
                        .entity_mut(target)
                        .insert((Silenced(now + SILENCE_DS), Muted));
                }
                "paralysis-sting" => {
                    world
                        .entity_mut(target)
                        .insert(Weakened(now + PARALYSIS_STING_DS));
                    tell(world, target, LocMsg::new("ling-muscles"));
                }
                "death-sting" => {
                    world.entity_mut(target).insert((
                        Silenced(now + DEATH_SILENCE_DS),
                        Muted,
                        Paralyzed(now + DEATH_PARALYSIS_DS),
                    ));
                    if let Some(mut blood) = world.get_mut::<Reagents>(target) {
                        blood.add("lexorin", 40.0);
                    }
                    tell(world, target, LocMsg::new("ling-chest"));
                }
                "extract-dna" => {
                    let theirs = dna_of(world, target);
                    if let Some(mut ling) = world.get_mut::<Changeling>(mob)
                        && !ling.dna.contains(&theirs)
                    {
                        ling.dna.push(theirs);
                    }
                }
                "transformation-sting" => {
                    if world.get::<Husked>(target).is_some() {
                        tell(world, mob, LocMsg::new("ling-sting-useless"));
                        return;
                    }
                    if let Some(chosen) = chosen {
                        wear_dna(world, target, &chosen);
                        if let Some(at) = world.get::<GridPos>(target).map(|p| p.0) {
                            let msg = LocMsg::new("ling-transforms").arg("name", chosen.name);
                            say_around(world, at, msg);
                        }
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
}

/// Something asked of the changeling's window.
pub fn command(world: &mut World, mob: Entity, op: LingOp) {
    if world.get::<Changeling>(mob).is_none() {
        return;
    }
    match op {
        LingOp::Open => {
            if world.get::<LingViewer>(mob).is_some() {
                world.entity_mut(mob).remove::<LingViewer>();
            } else {
                world
                    .entity_mut(mob)
                    .insert((LingViewer, LingPanelOut::default()));
            }
        }
        LingOp::Buy(line) => {
            let Some(power) = def(usize::from(line)) else {
                return;
            };
            let Some(mut ling) = world.get_mut::<Changeling>(mob) else {
                return;
            };
            if ling.has(power.id) || ling.points < i32::from(power.cost) {
                return;
            }
            ling.points -= i32::from(power.cost);
            ling.powers.push(power.id);
            match power.id {
                "chem-synth" | "adv-chem-synth" => ling.rate *= 2.0,
                "engorged-glands" => ling.storage += 25.0,
                _ => {}
            }
        }
        LingOp::Use { power, target, dna } => use_power(world, mob, power, target, dna),
        LingOp::Mimic(name) => {
            let Some(mut ling) = world.get_mut::<Changeling>(mob) else {
                return;
            };
            if !ling.has("mimic-voice") {
                return;
            }
            let Some(clean) = crate::names::sanitize_name(&name) else {
                return;
            };
            ling.mimic = Some(clean.clone());
            tell(world, mob, LocMsg::new("ling-voice").arg("name", clean));
        }
        LingOp::GiveUp => {
            if !is_alive(world, mob) {
                world.entity_mut(mob).remove::<(Changeling, LingViewer)>();
            }
        }
    }
}

/// Chemicals coming back, absorbing's stages, stasis, upkeep and stings wearing off.
pub fn run_changelings(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    let mut lings = world.query_filtered::<Entity, With<Changeling>>();
    let list: Vec<Entity> = lings.iter(world).collect();
    for mob in list {
        let Some(mut ling) = world.get_mut::<Changeling>(mob) else {
            continue;
        };
        if now >= ling.next_life {
            ling.next_life = now + LIFE_TICK_DS;
            ling.chems = (ling.chems + ling.rate).clamp(0.0, ling.storage);
            ling.genetic_damage = (ling.genetic_damage - 1.0).max(0.0);
            if ling.regen_left > 0 {
                ling.regen_left -= 1;
                if let Some(mut body) = world.get_mut::<Body>(mob) {
                    body.take_damage_spread(-10.0, -10.0);
                    body.tox = (body.tox - 10.0).max(0.0);
                    body.oxy = (body.oxy - 10.0).max(0.0);
                }
            }
        }
        let Some(mut ling) = world.get_mut::<Changeling>(mob) else {
            continue;
        };
        if (ling.camo || ling.mimic.is_some()) && now >= ling.next_upkeep {
            ling.next_upkeep = now + UPKEEP_DS;
            ling.chems = (ling.chems - 1.0).max(0.0);
        }
        let stasis = ling.stasis_until.filter(|at| now >= *at);
        if stasis.is_some() {
            ling.stasis_until = None;
            wake_from_stasis(world, mob);
        }
        run_absorbing(world, mob, now);
        // Dead too long ends the round (Luna's `changelingdeathticker`).
        let dead = !is_alive(world, mob);
        let Some(mut ling) = world.get_mut::<Changeling>(mob) else {
            continue;
        };
        let since = if dead { ling.dead_since.or(Some(now)) } else { None };
        ling.dead_since = since;
        if since.is_some_and(|at| now - at >= DEAD_ENDS_ROUND_DS) && mode_is_ling(world) {
            world.get_resource_or_init::<LingState>().dead_end = true;
        }
    }
    let mut deaf = world.query::<(Entity, &Deafened)>();
    let over: Vec<Entity> = deaf
        .iter(world)
        .filter(|(_, d)| now >= d.0)
        .map(|(e, _)| e)
        .collect();
    for mob in over {
        world.entity_mut(mob).remove::<Deafened>();
    }
    let mut quiet = world.query::<(Entity, &Silenced)>();
    let over: Vec<Entity> = quiet
        .iter(world)
        .filter(|(_, s)| now >= s.0)
        .map(|(e, _)| e)
        .collect();
    for mob in over {
        world.entity_mut(mob).remove::<(Silenced, Muted)>();
    }
    // The hive channel reaches changelings and the dead.
    let lines = std::mem::take(&mut world.resource_mut::<HiveLines>().0);
    if !lines.is_empty() {
        let mut hearers =
            world.query_filtered::<&mut Outbox, Or<(With<Changeling>, With<rs13_world::Ghost>)>>();
        for mut outbox in hearers.iter_mut(world) {
            outbox.0.extend(lines.iter().cloned());
        }
    }
    // The deaf hear no one.
    let mut outboxes = world.query_filtered::<&mut Outbox, With<Deafened>>();
    for mut outbox in outboxes.iter_mut(world) {
        outbox
            .0
            .retain(|msg| !matches!(msg.id.as_str(), "say" | "whisper" | "radio"));
    }
}

fn run_absorbing(world: &mut World, mob: Entity, now: f32) {
    let Some(state) = world.get::<Changeling>(mob).and_then(|l| l.absorbing) else {
        return;
    };
    let still = world.get::<GridPos>(mob).is_some_and(|p| p.0 == state.from)
        && world
            .get::<GridPos>(state.victim)
            .is_some_and(|p| p.0 == state.their_tile)
        && stat_of(world, mob) == MobStat::Conscious;
    if !still {
        tell(world, mob, LocMsg::new("ling-interrupted"));
        if let Some(mut ling) = world.get_mut::<Changeling>(mob) {
            ling.absorbing = None;
        }
        return;
    }
    if now < state.until {
        return;
    }
    let next = state.stage + 1;
    let at = state.from;
    match next {
        2 => {
            tell(world, mob, LocMsg::new("ling-absorb-2"));
            let msg = LocMsg::new("ling-proboscis").arg("name", name_of(world, mob));
            say_around(world, at, msg);
        }
        3 => {
            tell(world, mob, LocMsg::new("ling-absorb-3"));
            let msg = LocMsg::new("ling-stabs")
                .arg("name", name_of(world, mob))
                .arg("victim", name_of(world, state.victim));
            say_around(world, at, msg);
            tell(world, state.victim, LocMsg::new("ling-stab-pain"));
            if let Some(mut body) = world.get_mut::<Body>(state.victim) {
                body.take_damage_spread(40.0, 0.0);
            }
        }
        _ => {
            if let Some(mut ling) = world.get_mut::<Changeling>(mob) {
                ling.absorbing = None;
            }
            finish_absorbing(world, mob, state.victim);
            return;
        }
    }
    if let Some(mut ling) = world.get_mut::<Changeling>(mob) {
        ling.absorbing = Some(Absorbing {
            stage: next,
            until: now + ABSORB_STAGE_DS,
            ..state
        });
    }
}

fn mode_is_ling(world: &World) -> bool {
    world
        .get_resource::<Round>()
        .is_some_and(|round| matches!(round.phase, Phase::Playing { mode: MODE }))
}

/// Whether the round's over because the changeling's dead.
pub fn over(world: &World) -> bool {
    world
        .get_resource::<LingState>()
        .is_some_and(|state| state.dead_end)
}

/// How they did (Luna's `declare_completion`).
pub fn report(world: &mut World) -> Vec<LocMsg> {
    let mut lings = world.query::<(Entity, &Changeling)>();
    let list: Vec<(Entity, Vec<Objective>, u32)> = lings
        .iter(world)
        .map(|(e, l)| (e, l.objectives.clone(), l.absorbed))
        .collect();
    let mut said = Vec::new();
    for (mob, objectives, absorbed) in list {
        said.push(LocMsg::new("ling-was").arg("name", name_of(world, mob)));
        said.push(LocMsg::new("ling-genomes").num("value", i64::from(absorbed)));
        let mut won = true;
        for (i, objective) in objectives.iter().enumerate() {
            let done = achieved(world, mob, objective);
            won &= done;
            said.push(objective.line(i + 1, if done { "won" } else { "failed" }));
        }
        said.push(LocMsg::new(if won { "ling-won" } else { "ling-lost" }));
    }
    said
}

/// Keep open windows up to date.
pub fn update_ling_views(world: &mut World) {
    let mut viewers = world.query_filtered::<Entity, (With<LingViewer>, With<Changeling>)>();
    let list: Vec<Entity> = viewers.iter(world).collect();
    if list.is_empty() {
        return;
    }
    let hive: Vec<Dna> = world
        .get_resource::<LingState>()
        .map_or_else(Vec::new, |state| state.hive.clone());
    let mut people = world.query_filtered::<(Entity, &GridPos, &NetId, Option<&DisplayName>), With<Mob>>();
    let everyone: Vec<(Entity, TilePos, NetId, String)> = people
        .iter(world)
        .map(|(e, p, id, n)| (e, p.0, *id, n.map_or_else(String::new, |n| n.0.clone())))
        .collect();
    for mob in list {
        let Some(ling) = world.get::<Changeling>(mob).cloned() else {
            continue;
        };
        let at = world.get::<GridPos>(mob).map(|p| p.0);
        let nearby = at.map_or_else(Vec::new, |at| {
            everyone
                .iter()
                .filter(|(other, pos, ..)| {
                    *other != mob
                        && pos.z == at.z
                        && (pos.x - at.x).abs().max((pos.y - at.y).abs()) <= ling.sting_range
                })
                .map(|(_, _, id, name)| (*id, name.clone()))
                .collect()
        });
        let view = ChangelingPanelView {
            chems: ling.chems as u16,
            storage: ling.storage as u16,
            points: ling.points.max(0) as u16,
            genetic_damage: ling.genetic_damage as u16,
            absorbed: ling.absorbed as u16,
            powers: POWERS
                .iter()
                .map(|p| ChangelingPower {
                    id: p.id.to_owned(),
                    cost: p.cost,
                    chems: p.chems,
                    owned: ling.has(p.id),
                    passive: p.aim == Aim::Passive,
                    needs_target: matches!(p.aim, Aim::Target | Aim::TargetDna),
                    needs_dna: matches!(p.aim, Aim::Dna | Aim::TargetDna | Aim::Hive),
                    needs_text: p.aim == Aim::Text,
                })
                .collect(),
            dna: ling.dna.iter().map(|d| d.name.clone()).collect(),
            hive: hive
                .iter()
                .filter(|d| !ling.dna.contains(d))
                .map(|d| d.name.clone())
                .collect(),
            nearby,
            mimic: ling.mimic.clone(),
            dead: !is_alive(world, mob),
        };
        if let Some(mut out) = world.get_mut::<LingPanelOut>(mob)
            && out.0.as_ref() != Some(&view)
        {
            out.0 = Some(view);
        }
    }
}

/// Dropped from a viewer who's stopped watching.
pub fn clear_closed_views(mut out: Query<&mut LingPanelOut, Without<LingViewer>>) {
    for mut panel in &mut out {
        if panel.0.is_some() {
            panel.0 = None;
        }
    }
}

/// The text of a line meant for the hive channel: `:g`, on either keyboard layout.
pub fn hive_text(raw: &str) -> Option<&str> {
    let raw = raw.trim_start();
    raw.strip_prefix(":g")
        .or_else(|| raw.strip_prefix(":G"))
        .or_else(|| raw.strip_prefix(":\u{43f}"))
        .or_else(|| raw.strip_prefix(":\u{41f}"))
}

/// What a mob says on the hive channel goes to every changeling, and the ghosts (Luna `say.dm`).
pub fn hive_line(speaker: &Changeling, text: &str) -> LocMsg {
    LocMsg::new("ling-hive-say")
        .arg("name", speaker.id_name.clone())
        .arg("text", text)
}
