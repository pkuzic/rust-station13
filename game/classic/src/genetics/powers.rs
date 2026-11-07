//! What genes do (Luna `domutcheck` and the mutations' effects in `human_life` and
//! `carbon/life`): which are on, what each gives while it lasts, and radiation.

use std::collections::BTreeSet;

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, DisplayName, GridPos, HumanLook, MobStat, Vision};

use super::dna::{self, MONKEY_BLOCK, Threshold};
use super::{Genome, Radiation, genome, layout, random_bad, random_good, random_identity, tell};
use crate::actions::{Action, PendingActions};
use crate::changeling::Deafened;
use crate::combat::{COUNT_DS, Paralyzed, VisibleEvents};
use crate::environment::LIFE_TICKS;
use crate::flashes::Weakened;
use crate::health::{Body, Vitals, Zone};
use crate::rng::GameRng;
use crate::time::WorldTime;

/// A gene of the structural enzymes, each with its block this round (Luna's `*BLOCK`s).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Gene {
    Hulk,
    Telekinesis,
    /// Luna's `FIREBLOCK`: cold resistance, and no burns.
    ColdResist,
    Xray,
    Clumsy,
    /// Luna's `FAKEBLOCK`: no gene of its own, it scrambles another.
    Fake,
    Deaf,
    Blind,
    /// Epilepsy: seizures.
    Headache,
    Cough,
    /// Tourette's.
    Twitch,
    /// Stuttering.
    Nervous,
    NoBreath,
    RemoteView,
    Regenerate,
    Run,
    RemoteTalk,
    Morph,
    Blend,
    Hallucination,
    NoPrints,
    ShockImmune,
    Small,
}

impl Gene {
    /// In the order Luna hands out their blocks.
    pub const ALL: [Gene; 23] = [
        Gene::Hulk,
        Gene::Telekinesis,
        Gene::ColdResist,
        Gene::Xray,
        Gene::Clumsy,
        Gene::Fake,
        Gene::Deaf,
        Gene::Blind,
        Gene::Headache,
        Gene::Cough,
        Gene::Twitch,
        Gene::Nervous,
        Gene::NoBreath,
        Gene::RemoteView,
        Gene::Regenerate,
        Gene::Run,
        Gene::RemoteTalk,
        Gene::Morph,
        Gene::Blend,
        Gene::Hallucination,
        Gene::NoPrints,
        Gene::ShockImmune,
        Gene::Small,
    ];

    fn threshold(self) -> Threshold {
        match self {
            Gene::Hulk | Gene::Telekinesis => Threshold::Hard,
            Gene::Xray | Gene::ColdResist => Threshold::Mid,
            _ => Threshold::Normal,
        }
    }

    /// The chance a power takes hold when its block is on but it wasn't yet; `None` for the
    /// defects, which always do.
    fn chance(self) -> Option<f32> {
        match self {
            Gene::Hulk => Some(15.0),
            Gene::Telekinesis => Some(20.0),
            Gene::Clumsy
            | Gene::Fake
            | Gene::Deaf
            | Gene::Blind
            | Gene::Headache
            | Gene::Cough
            | Gene::Twitch
            | Gene::Nervous => None,
            _ => Some(30.0),
        }
    }

    /// What its message is called: `gene-<id>`.
    pub fn id(self) -> &'static str {
        match self {
            Gene::Hulk => "hulk",
            Gene::Telekinesis => "telekinesis",
            Gene::ColdResist => "cold-resist",
            Gene::Xray => "xray",
            Gene::Clumsy => "clumsy",
            Gene::Fake => "fake",
            Gene::Deaf => "deaf",
            Gene::Blind => "blind",
            Gene::Headache => "headache",
            Gene::Cough => "cough",
            Gene::Twitch => "twitch",
            Gene::Nervous => "nervous",
            Gene::NoBreath => "no-breath",
            Gene::RemoteView => "remote-view",
            Gene::Regenerate => "regenerate",
            Gene::Run => "run",
            Gene::RemoteTalk => "remote-talk",
            Gene::Morph => "morph",
            Gene::Blend => "blend",
            Gene::Hallucination => "hallucination",
            Gene::NoPrints => "no-prints",
            Gene::ShockImmune => "shock-immune",
            Gene::Small => "small",
        }
    }
}

/// Luna's order of looking: the lesser powers first, then the rest as `domutcheck` has them.
const CHECK_ORDER: [Gene; 23] = [
    Gene::NoBreath,
    Gene::RemoteView,
    Gene::Regenerate,
    Gene::Run,
    Gene::RemoteTalk,
    Gene::Morph,
    Gene::Blend,
    Gene::Hallucination,
    Gene::NoPrints,
    Gene::ShockImmune,
    Gene::Small,
    Gene::Hulk,
    Gene::Headache,
    Gene::Fake,
    Gene::Cough,
    Gene::Clumsy,
    Gene::Twitch,
    Gene::Xray,
    Gene::Nervous,
    Gene::ColdResist,
    Gene::Blind,
    Gene::Telekinesis,
    Gene::Deaf,
];

/// The powers and defects someone has now (Luna `mutations`, `disabilities`, `sdisabilities`).
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct Mutations(pub BTreeSet<Gene>);

pub fn has(world: &World, mob: Entity, gene: Gene) -> bool {
    world.get::<Mutations>(mob).is_some_and(|m| m.0.contains(&gene))
}

/// Whether a gene's block is on in someone's enzymes.
pub fn block_on(world: &mut World, mob: Entity, gene: Gene) -> bool {
    let layout = layout(world);
    genome(world, mob).is_some_and(|g| {
        dna::is_on(
            dna::block(&g.se, layout.block(gene)),
            gene.threshold(),
            layout.block_add,
        )
    })
}

/// Luna `domutcheck`, outside a scanner.
pub fn check(world: &mut World, mob: Entity, injected: bool) {
    check_in(world, mob, injected, false);
}

/// Luna `domutcheck`: the enzymes read again. What was had stays; a power newly on takes hold
/// at its chance (always, when `injected`); defects always do. The monkey block turns a person
/// into a monkey or back, inside the scanner if they lie in one.
pub fn check_in(world: &mut World, mob: Entity, injected: bool, _in_scanner: bool) {
    let layout = layout(world);
    let Some(g) = genome(world, mob) else {
        return;
    };
    let old = world.get::<Mutations>(mob).cloned().unwrap_or_default();
    let mut now = BTreeSet::new();
    let mut fake = false;
    for gene in CHECK_ORDER {
        let on = dna::is_on(
            dna::block(&g.se, layout.block(gene)),
            gene.threshold(),
            layout.block_add,
        );
        if !on {
            continue;
        }
        if gene == Gene::Fake {
            fake = true;
            tell(world, mob, LocMsg::new("gene-fake"));
            continue;
        }
        let takes = match gene.chance() {
            None => true,
            Some(chance) => injected || old.0.contains(&gene) || world.resource_mut::<GameRng>().prob(chance),
        };
        if takes {
            now.insert(gene);
            tell(world, mob, LocMsg::new(format!("gene-{}", gene.id())));
        }
    }
    world.entity_mut(mob).insert(Mutations(now.clone()));
    if let Some(mut body) = world.get_mut::<Body>(mob) {
        body.burn_proof = now.contains(&Gene::ColdResist);
    }
    // Luna's blindness covers the screen whatever else they could see.
    if now.contains(&Gene::Blind) {
        world.entity_mut(mob).insert(Vision::Blind);
    } else if now.contains(&Gene::Xray) {
        world.entity_mut(mob).insert(Vision::Xray);
    } else {
        world.entity_mut(mob).remove::<Vision>();
    }
    if fake {
        // Something else goes wrong, or now and then right.
        let (bad, ident) = {
            let mut rng = world.resource_mut::<GameRng>();
            (rng.prob(95.0), rng.prob(50.0))
        };
        match (bad, ident) {
            (true, false) => random_bad(world, mob),
            (true, true) => random_identity(world, mob),
            (false, _) => random_good(world, mob),
        }
    }
    let monkey_on = dna::is_on(
        dna::block(&g.se, MONKEY_BLOCK),
        Threshold::Normal,
        layout.block_add,
    );
    let is_person = super::is_person(world, mob);
    let is_monkey = world
        .get::<Appearance>(mob)
        .is_some_and(crate::monkey_round::is_monkey);
    if monkey_on && is_person {
        to_monkey(world, mob);
    } else if !monkey_on && is_monkey && !is_person {
        to_person(world, mob);
    }
}

/// Luna's human to monkey: all they had falls, and the change hurts.
fn to_monkey(world: &mut World, mob: Entity) {
    crate::monkey_round::monkeyize(world, mob);
    let ue = world.get::<Genome>(mob).map(|g| g.ue.clone()).unwrap_or_default();
    world.entity_mut(mob).insert(DisplayName(format!(
        "monkey ({})",
        ue.get(1..5).unwrap_or("0000")
    )));
    hurt_by_change(world, mob);
}

/// Luna's monkey to human: looking as their identifier says, under a new name.
fn to_person(world: &mut World, mob: Entity) {
    let Some(g) = world.get::<Genome>(mob).cloned() else {
        return;
    };
    let block_add = layout(world).block_add;
    let look = dna::look_of(&g.ui, block_add, 1);
    crate::jobs::drop_everything(world, mob);
    let was = world
        .get::<crate::monkey_round::Monkeyized>(mob)
        .map(|m| m.was.clone())
        .filter(|w| !w.is_empty());
    let name = was.unwrap_or_else(|| {
        let seed = world.resource_mut::<GameRng>().range(0, i32::MAX) as u64;
        crate::names::default_name(seed)
    });
    if let Some(mut a) = world.get_mut::<Appearance>(mob) {
        a.sprite = "human".to_owned();
    }
    world
        .entity_mut(mob)
        .remove::<(crate::monkey_round::Monkeyized, crate::monkeys::Monkey)>()
        .insert((look, DisplayName(name)));
    hurt_by_change(world, mob);
}

fn hurt_by_change(world: &mut World, mob: Entity) {
    if let Some(mut body) = world.get_mut::<Body>(mob) {
        body.tox += 20.0;
        body.take_damage_spread(40.0, 0.0);
    }
}

/// Luna `updateappearance`: someone made to look as their identifier says.
pub fn update_look(world: &mut World, mob: Entity) {
    let Some(g) = world.get::<Genome>(mob).cloned() else {
        return;
    };
    let block_add = layout(world).block_add;
    let underwear = match world.get::<HumanLook>(mob) {
        Some(old) => old.underwear,
        // A person who never chose a look gets one now; a monkey keeps its shape.
        None if super::is_person(world, mob) => HumanLook::default().underwear,
        None => return,
    };
    let look = dna::look_of(&g.ui, block_add, underwear);
    world.entity_mut(mob).insert(look);
}

fn emote(world: &mut World, mob: Entity, act: &str) {
    let (Some(at), Some(name)) = (
        world.get::<GridPos>(mob).map(|p| p.0),
        world.get::<DisplayName>(mob).map(|n| n.0.clone()),
    ) else {
        return;
    };
    if let Some(msg) = crate::speech::emote(&name, act) {
        world.resource_mut::<VisibleEvents>().0.push((at, msg, None));
    }
}

fn weaken(world: &mut World, mob: Entity, counts: f32) {
    if has(world, mob, Gene::Hulk) {
        return;
    }
    let now = world.resource::<WorldTime>().deciseconds();
    let until = now + counts * COUNT_DS;
    let later = world.get::<Weakened>(mob).map_or(until, |w| w.0.max(until));
    world.entity_mut(mob).insert(Weakened(later));
}

/// A hulk can't be stunned, knocked down or knocked out (Luna's human `Stun`, `Weaken` and
/// `Paralyse`).
pub fn shrug_off(world: &mut World) {
    let mut q = world.query_filtered::<(Entity, &Mutations), Or<(With<Weakened>, With<Paralyzed>)>>();
    let hulks: Vec<Entity> = q
        .iter(world)
        .filter(|(_, m)| m.0.contains(&Gene::Hulk))
        .map(|(e, _)| e)
        .collect();
    for mob in hulks {
        world.entity_mut(mob).remove::<(Weakened, Paralyzed)>();
    }
}

/// A hulk's punch (Luna `attack_hand`): 5 more, and the one hit knocked down and back.
pub fn hulk_punch(world: &mut World, attacker: Entity, target: Entity, zone: Zone) {
    if !has(world, attacker, Gene::Hulk) {
        return;
    }
    if let Some(mut body) = world.get_mut::<Body>(target) {
        body.take_damage(zone, 5.0, 0.0);
    }
    weaken(world, target, 2.0);
    let (Some(from), Some(to)) = (
        world.get::<GridPos>(attacker).map(|p| p.0),
        world.get::<GridPos>(target).map(|p| p.0),
    ) else {
        return;
    };
    let (dx, dy) = ((to.x - from.x).signum(), (to.y - from.y).signum());
    let dir = match (dx, dy) {
        (0, 1) => Some(rs13_world::Dir::North),
        (0, -1) => Some(rs13_world::Dir::South),
        (1, _) => Some(rs13_world::Dir::East),
        (-1, _) => Some(rs13_world::Dir::West),
        _ => None,
    };
    if let (Some(dir), Some(mut intent)) = (dir, world.get_mut::<crate::movement::MoveIntent>(target)) {
        intent.dir = Some(dir);
    }
}

/// Every 2 s: radiation, and what each power and defect does (Luna
/// `handle_mutations_and_radiation` and `handle_disabilities`).
pub fn run_genes(world: &mut World) {
    if !world.resource::<WorldTime>().tick.0.is_multiple_of(LIFE_TICKS) {
        return;
    }
    let now = world.resource::<WorldTime>().deciseconds();
    let mut q = world.query::<(Entity, &Vitals, Option<&Mutations>, Option<&Radiation>)>();
    let mobs: Vec<(Entity, MobStat, Option<Mutations>, f32)> = q
        .iter(world)
        .filter(|(_, _, m, r)| m.is_some() || r.is_some())
        .map(|(e, v, m, r)| (e, v.stat, m.cloned(), r.map_or(0.0, |r| r.0)))
        .collect();
    for (mob, stat, genes, rads) in mobs {
        if stat == MobStat::Dead {
            continue;
        }
        if rads != 0.0 {
            radiation(world, mob, rads);
        }
        let Some(genes) = genes else { continue };
        let health = world.get::<Body>(mob).map_or(100.0, Body::health);
        if genes.0.contains(&Gene::Hulk) && health <= 25.0 {
            if let Some(mut m) = world.get_mut::<Mutations>(mob) {
                m.0.remove(&Gene::Hulk);
            }
            tell(world, mob, LocMsg::new("gene-hulk-lost"));
            weaken(world, mob, 3.0);
            emote(world, mob, "collapse");
        }
        if genes.0.contains(&Gene::ColdResist)
            && let Some(mut body) = world.get_mut::<Body>(mob)
        {
            let burn: f32 = Zone::ALL.iter().map(|z| body.organ(*z).burn).sum();
            let heal = if burn > 0.0 && burn <= 50.0 {
                1.0
            } else if burn > 50.0 && burn <= 100.0 {
                5.0
            } else {
                0.0
            };
            spread_heal(&mut body, 0.0, heal);
        }
        if genes.0.contains(&Gene::Regenerate)
            && let Some(mut body) = world.get_mut::<Body>(mob)
        {
            body.oxy = (body.oxy - 2.0).max(0.0);
            body.tox = (body.tox - 2.0).max(0.0);
            for zone in Zone::ALL {
                body.heal(zone, 2.0, 2.0);
            }
        }
        if genes.0.contains(&Gene::Deaf) {
            let until = now + LIFE_TICKS as f32 * 10.0 / rs13_core::TICK_RATE_HZ as f32 * 2.0;
            let later = world.get::<Deafened>(mob).map_or(until, |d| d.0.max(until));
            world.entity_mut(mob).insert(Deafened(later));
        }
        if stat != MobStat::Conscious {
            continue;
        }
        let paralyzed = world.get::<Paralyzed>(mob).is_some_and(|p| p.0 > now);
        if genes.0.contains(&Gene::Headache) && !paralyzed && world.resource_mut::<GameRng>().prob(1.0) {
            tell(world, mob, LocMsg::new("gene-seizure"));
            if let (Some(at), Some(name)) = (
                world.get::<GridPos>(mob).map(|p| p.0),
                world.get::<DisplayName>(mob).map(|n| n.0.clone()),
            ) {
                let msg = LocMsg::new("gene-seizure-seen").arg("name", name);
                world.resource_mut::<VisibleEvents>().0.push((at, msg, None));
            }
            if !genes.0.contains(&Gene::Hulk) {
                world.entity_mut(mob).insert(Paralyzed(now + 10.0 * COUNT_DS));
            }
        }
        if genes.0.contains(&Gene::Cough) && !paralyzed && world.resource_mut::<GameRng>().prob(5.0) {
            if let Some(mut pending) = world.get_mut::<PendingActions>(mob) {
                pending.0.push(Action::Drop);
            }
            emote(world, mob, "cough");
        }
        if genes.0.contains(&Gene::Twitch) && world.resource_mut::<GameRng>().prob(5.0) {
            // Luna stuns them, which here, as for every stun, is being knocked down.
            emote(world, mob, "twitch");
            weaken(world, mob, 5.0);
        }
    }
}

fn spread_heal(body: &mut Body, brute: f32, burn: f32) {
    let n = Zone::ALL.len() as f32;
    for zone in Zone::ALL {
        body.heal(zone, brute / n, burn / n);
    }
}

/// Luna's radiation, every 2 s: it wears off, poisoning as it goes; a lot knocks them down, and
/// the worst of it mutates them.
fn radiation(world: &mut World, mob: Entity, rads: f32) {
    let mut rads = rads;
    if rads > 100.0 {
        rads = 100.0;
        weaken(world, mob, 10.0);
        tell(world, mob, LocMsg::new("radiation-weak"));
        emote(world, mob, "collapse");
    }
    let mut tox = 0.0;
    if rads < 1.0 {
        rads = 0.0;
    } else if rads < 50.0 {
        rads -= 1.0;
        if world.resource_mut::<GameRng>().prob(25.0) {
            tox += 1.0;
        }
    } else if rads < 75.0 {
        rads -= 2.0;
        tox += 1.0;
        if world.resource_mut::<GameRng>().prob(5.0) {
            rads -= 15.0;
            weaken(world, mob, 3.0);
            tell(world, mob, LocMsg::new("radiation-weak"));
            emote(world, mob, "collapse");
        }
    } else {
        rads -= 3.0;
        tox += 3.0;
        if world.resource_mut::<GameRng>().prob(1.0) {
            tell(world, mob, LocMsg::new("radiation-mutate"));
            if world.resource_mut::<GameRng>().prob(90.0) {
                random_bad(world, mob);
            } else {
                random_good(world, mob);
            }
            check(world, mob, true);
            emote(world, mob, "gasp");
            rads -= 50.0;
        }
    }
    if let Some(mut body) = world.get_mut::<Body>(mob) {
        body.tox += tox;
    }
    let rads = rads.max(0.0);
    if rads == 0.0 {
        world.entity_mut(mob).remove::<Radiation>();
    } else {
        world.entity_mut(mob).insert(Radiation(rads));
    }
}
