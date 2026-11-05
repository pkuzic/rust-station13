//! The traitor mode: someone among the crew has objectives of their own, and an uplink to buy what
//! they need with (Luna's `gamemodes/traitor`, `newobjective`, `Abi79/uplinks`). Spec:
//! `docs/reference/traitor.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{
    Appearance, DisplayName, GridPos, Hand, MobStat, NetId, ObjectSpec, Slot, TilePos, UplinkPanelView,
};

use crate::actions::Outbox;
use crate::equipment::{Equipment, WornBy};
use crate::escape::in_pod;
use crate::hands::{Hands, HeldBy};
use crate::health::Vitals;
use crate::jobs::HasJob;
use crate::movement::Mob;
use crate::objects::spawn_object;
use crate::rng::GameRng;
use crate::security::Cuffed;
use crate::storage::{InStorage, Storage};
use crate::time::WorldTime;
use crate::wires::PanelViewer;

pub const UPLINK: &str = "uplink-radio";
/// The most traitors a round has (Luna `traitors_possible`).
pub const MOST_TRAITORS: usize = 4;
/// What an uplink starts with (Luna `uplink_uses`).
pub const CRYSTALS: u8 = 10;
/// When a traitor is reminded, ds (Luna's 6000).
pub const REMINDER_DS: f32 = 6000.0;
/// What an uplink sells, and for how many crystals. Luna's other goods wait for what makes them work
/// (`traitor.md`).
pub const GOODS: [(&str, u8); 6] = [
    (crate::weapons::SYNDICATE_REVOLVER, 7),
    ("ammo-357", 3),
    ("toolbox-syndicate", 1),
    ("soap", 1),
    (crate::explosions::C4_SMALL, 4),
    (crate::explosions::C4_LARGE, 7),
];
/// What can be set as a theft (Luna `get_all_thefts`, of what's on this station).
pub const THEFTS: [&str; 8] = [
    "hand-tele",
    "jetpack",
    "magboots",
    "plasma-tank",
    "hypospray",
    "bone-gel",
    "ablative-vest",
    "pinpointer",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Objective {
    /// Carry one of these away.
    Steal(&'static str),
    /// See that this person dies.
    Assassinate {
        target: Entity,
        name: String,
        job: String,
    },
    /// See that this person gets away alive.
    Protect {
        target: Entity,
        name: String,
        job: String,
    },
    Escape,
    /// Be the only one to get away.
    Hijack,
}

impl Objective {
    /// How much of a traitor's workload it is (Luna `weight`).
    fn weight(&self) -> u32 {
        match self {
            Objective::Steal(_) => 20,
            Objective::Assassinate { .. } => 60,
            Objective::Protect { .. } => 30,
            _ => 0,
        }
    }

    /// Its line in a briefing or the report. `done`: `none`, `won` or `failed`.
    fn line(&self, number: usize, done: &str) -> LocMsg {
        let msg = |id: &str| LocMsg::new(id).num("value", number as i64).arg("done", done);
        match self {
            Objective::Steal(item) => msg("objective-steal").arg("item", *item),
            Objective::Assassinate { name, job, .. } => msg("objective-assassinate")
                .arg("name", name.clone())
                .msg("job", format!("job-{job}")),
            Objective::Protect { name, job, .. } => msg("objective-protect")
                .arg("name", name.clone())
                .msg("job", format!("job-{job}")),
            Objective::Escape => msg("objective-escape"),
            Objective::Hijack => msg("objective-hijack"),
        }
    }
}

/// Someone working for the Syndicate.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Traitor {
    pub objectives: Vec<Objective>,
    /// When they're next reminded, ds.
    pub remind_at: Option<f32>,
}

/// Would like to be a traitor (Luna `be_syndicate`): those who would are picked first.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct WantsTraitor;

/// A radio that's really an uplink, and the crystals left in it.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Uplink {
    pub crystals: u8,
}

impl Default for Uplink {
    fn default() -> Self {
        Self { crystals: CRYSTALS }
    }
}

/// What a mob bought this tick: the uplink, and the line of `GOODS`.
#[derive(Component, Debug, Default)]
pub struct PendingUplinkOps(pub Vec<(Entity, usize)>);

/// What a mob's open uplink shows.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct UplinkPanelOut(pub Option<UplinkPanelView>);

/// How many traitors, of `players` and a roll from 0 to 5 (Luna's `traitor_scaling`).
pub fn how_many(players: usize, roll: usize) -> usize {
    (((players + roll) as f32 / 10.0).round() as usize).clamp(1, MOST_TRAITORS)
}

/// What one traitor is to do (Luna `SelectObjectives`): theft, murder and protection until the
/// work adds up, then how to leave.
fn select_objectives(rng: &mut GameRng, others: &[(Entity, String, String)]) -> Vec<Objective> {
    let mut thefts: Vec<&'static str> = THEFTS.to_vec();
    let mut kills: Vec<&(Entity, String, String)> = others.iter().collect();
    let mut guards = kills.clone();
    let mut chosen: Vec<Objective> = Vec::new();
    let mut work = 0;
    while work < 100 {
        let roll = if kills.is_empty() || guards.is_empty() {
            1
        } else {
            rng.range(1, 100)
        };
        let pick_one = |rng: &mut GameRng, len: usize| rng.range(0, len as i32 - 1) as usize;
        let objective = if roll <= 45 {
            if thefts.is_empty() {
                break;
            }
            let i = pick_one(rng, thefts.len());
            Objective::Steal(thefts.remove(i))
        } else {
            let murder = roll <= 85;
            let list = if murder { &mut kills } else { &mut guards };
            let (target, name, job) = list.remove(pick_one(rng, list.len()));
            // Never to kill and to save one person.
            let clash = chosen.iter().any(|o| match o {
                Objective::Assassinate { target: t, .. } | Objective::Protect { target: t, .. } => {
                    t == target
                }
                _ => false,
            });
            if clash {
                continue;
            }
            let (target, name, job) = (*target, name.clone(), job.clone());
            if murder {
                Objective::Assassinate { target, name, job }
            } else {
                Objective::Protect { target, name, job }
            }
        };
        work += objective.weight();
        chosen.push(objective);
    }
    chosen.push(if rng.prob(90.0) {
        Objective::Escape
    } else {
        Objective::Hijack
    });
    chosen
}

fn tell(world: &mut World, mob: Entity, msg: LocMsg) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(msg);
    }
}

fn brief(world: &mut World, mob: Entity) {
    let lines: Vec<LocMsg> = world.get::<Traitor>(mob).map_or_else(Vec::new, |t| {
        t.objectives
            .iter()
            .enumerate()
            .map(|(i, o)| o.line(i + 1, "none"))
            .collect()
    });
    for line in lines {
        tell(world, mob, line);
    }
}

/// Hide an uplink on them: in their backpack, in a free hand, or at their feet (Luna `equip_traitor`).
fn give_uplink(world: &mut World, mob: Entity, at: TilePos) {
    let Ok(item) = spawn_object(world, &ObjectSpec::new(UPLINK), at) else {
        return;
    };
    let pack = world
        .get::<Equipment>(mob)
        .and_then(|e| e.get(Slot::Back))
        .filter(|p| world.get::<Storage>(*p).is_some());
    if let Some(pack) = pack {
        world.entity_mut(item).remove::<GridPos>().insert(InStorage(pack));
        if let Some(mut storage) = world.get_mut::<Storage>(pack) {
            storage.items.push(item);
        }
        tell(world, mob, LocMsg::new("traitor-uplink-pack"));
    } else if put_in_hand(world, mob, item) {
        tell(world, mob, LocMsg::new("traitor-uplink-hand"));
    } else {
        tell(world, mob, LocMsg::new("traitor-uplink-floor"));
    }
}

/// Into a free hand, if there is one.
fn put_in_hand(world: &mut World, mob: Entity, item: Entity) -> bool {
    let Some(mut hands) = world.get_mut::<Hands>(mob) else {
        return false;
    };
    let other = match hands.active {
        Hand::Left => Hand::Right,
        Hand::Right => Hand::Left,
    };
    let hand = [hands.active, other]
        .into_iter()
        .find(|h| hands.slot_mut(*h).is_none());
    let Some(hand) = hand else {
        return false;
    };
    *hands.slot_mut(hand) = Some(item);
    world.entity_mut(item).remove::<GridPos>().insert(HeldBy(mob));
    true
}

/// Choose the round's traitors from those who've started, and tell them.
pub fn choose_traitors(world: &mut World) {
    let mut crew = world.query_filtered::<(Entity, &DisplayName, &HasJob), With<Mob>>();
    let mut all: Vec<(Entity, String, String)> = crew
        .iter(world)
        .map(|(e, n, j)| (e, n.0.clone(), j.0.clone()))
        .collect();
    all.sort_by_key(|(e, ..)| *e);
    if all.is_empty() {
        return;
    }
    let now = world.resource::<WorldTime>().deciseconds();
    let mut wanting = world.query_filtered::<Entity, With<WantsTraitor>>();
    let wants: Vec<Entity> = wanting.iter(world).collect();
    let mut rng = world.resource_mut::<GameRng>();
    let roll = rng.range(0, 5) as usize;
    let count = how_many(all.len(), roll).min(all.len());
    // If anyone would like to be one, it's from among them (Luna `get_possible_traitors`).
    let wanting: Vec<_> = all.iter().filter(|(e, ..)| wants.contains(e)).cloned().collect();
    let mut candidates = if wanting.is_empty() { all.clone() } else { wanting };
    let count = count.min(candidates.len());
    let mut chosen = Vec::new();
    for _ in 0..count {
        let i = rng.range(0, candidates.len() as i32 - 1) as usize;
        chosen.push(candidates.remove(i).0);
    }
    let plans: Vec<(Entity, Vec<Objective>)> = chosen
        .iter()
        .map(|mob| {
            let others: Vec<_> = all.iter().filter(|(e, ..)| e != mob).cloned().collect();
            (*mob, select_objectives(&mut rng, &others))
        })
        .collect();
    for (mob, objectives) in plans {
        world.entity_mut(mob).insert(Traitor {
            objectives,
            remind_at: Some(now + REMINDER_DS),
        });
        tell(world, mob, LocMsg::new("traitor-you"));
        tell(world, mob, LocMsg::new("traitor-repeat"));
        tell(world, mob, LocMsg::new("traitor-you"));
        brief(world, mob);
        if let Some(at) = world.get::<GridPos>(mob).map(|p| p.0) {
            give_uplink(world, mob, at);
        }
    }
}

/// A click on an uplink in the hand opens it, and for a traitor it also goes over their objectives,
/// standing in for Luna's Notes.
pub fn use_uplink(world: &mut World, mob: Entity, uplink: Entity) {
    if world.get::<HeldBy>(uplink).is_some_and(|h| h.0 == mob) {
        world.entity_mut(mob).insert(PanelViewer(uplink));
        brief(world, mob);
    }
}

fn buy(world: &mut World, mob: Entity, uplink: Entity, line: usize) {
    let Some((proto, cost)) = GOODS.get(line).copied() else {
        return;
    };
    if !world.get::<HeldBy>(uplink).is_some_and(|h| h.0 == mob) {
        return;
    }
    let Some(mut left) = world.get_mut::<Uplink>(uplink) else {
        return;
    };
    if left.crystals < cost {
        return;
    }
    left.crystals -= cost;
    let Some(at) = world.get::<GridPos>(mob).map(|p| p.0) else {
        return;
    };
    if proto == crate::explosions::C4_SMALL || proto == crate::explosions::C4_LARGE {
        if let Some((charge, detonator)) = crate::explosions::spawn_charge(world, proto, at) {
            put_in_hand(world, mob, charge);
            put_in_hand(world, mob, detonator);
        }
        return;
    }
    if let Ok(item) = spawn_object(world, &ObjectSpec::new(proto), at) {
        put_in_hand(world, mob, item);
    }
}

/// Purchases and reminders.
pub fn run_traitors(world: &mut World) {
    let mut buyers = world.query::<(Entity, &mut PendingUplinkOps)>();
    let asked: Vec<(Entity, Vec<(Entity, usize)>)> = buyers
        .iter_mut(world)
        .filter(|(_, ops)| !ops.0.is_empty())
        .map(|(e, mut ops)| (e, std::mem::take(&mut ops.0)))
        .collect();
    for (mob, ops) in asked {
        for (uplink, line) in ops {
            buy(world, mob, uplink, line);
        }
    }
    let now = world.resource::<WorldTime>().deciseconds();
    let mut traitors = world.query::<(Entity, &Traitor)>();
    let due: Vec<Entity> = traitors
        .iter(world)
        .filter(|(_, t)| t.remind_at.is_some_and(|at| now >= at))
        .map(|(e, _)| e)
        .collect();
    for mob in due {
        if let Some(mut traitor) = world.get_mut::<Traitor>(mob) {
            traitor.remind_at = None;
        }
        tell(world, mob, LocMsg::new("traitor-reminder"));
    }
}

/// Everything someone has on them: in their hands, worn, and in what those hold.
fn carried(world: &mut World, mob: Entity) -> Vec<Entity> {
    let mut stored = world.query::<(Entity, &InStorage)>();
    let inside: Vec<(Entity, Entity)> = stored.iter(world).map(|(e, s)| (e, s.0)).collect();
    let mut held = world.query::<(Entity, &HeldBy)>();
    let mut all: Vec<Entity> = held
        .iter(world)
        .filter(|(_, h)| h.0 == mob)
        .map(|(e, _)| e)
        .collect();
    let mut worn = world.query::<(Entity, &WornBy)>();
    all.extend(worn.iter(world).filter(|(_, w)| w.0 == mob).map(|(e, _)| e));
    let mut i = 0;
    while i < all.len() {
        let holder = all[i];
        all.extend(inside.iter().filter(|(_, c)| *c == holder).map(|(e, _)| *e));
        i += 1;
    }
    all
}

fn alive(world: &World, mob: Entity) -> bool {
    world.get::<Vitals>(mob).is_some_and(|v| v.stat != MobStat::Dead)
}

fn got_away(world: &mut World, mob: Entity) -> bool {
    let at = world.get::<GridPos>(mob).map(|p| p.0);
    alive(world, mob) && at.is_some_and(|at| in_pod(world, at))
}

fn achieved(world: &mut World, traitor: Entity, objective: &Objective) -> bool {
    match objective {
        Objective::Steal(item) => {
            let on_them = carried(world, traitor);
            on_them
                .into_iter()
                .any(|e| world.get::<Appearance>(e).is_some_and(|a| a.sprite == *item))
        }
        Objective::Assassinate { target, .. } => !alive(world, *target),
        Objective::Protect { target, .. } => got_away(world, *target),
        Objective::Escape => got_away(world, traitor) && world.get::<Cuffed>(traitor).is_none(),
        Objective::Hijack => {
            let mut mobs = world.query_filtered::<Entity, With<Mob>>();
            let everyone: Vec<Entity> = mobs.iter(world).collect();
            got_away(world, traitor) && !everyone.into_iter().any(|m| m != traitor && got_away(world, m))
        }
    }
}

/// Who the traitors were and how they did, for everyone (Luna `declare_completion`).
pub fn report(world: &mut World) -> Vec<LocMsg> {
    let mut traitors = world.query::<(Entity, &Traitor, Option<&DisplayName>)>();
    let list: Vec<(Entity, Vec<Objective>, String)> = traitors
        .iter(world)
        .map(|(e, t, n)| {
            (
                e,
                t.objectives.clone(),
                n.map_or_else(String::new, |n| n.0.clone()),
            )
        })
        .collect();
    let mut said = Vec::new();
    for (mob, objectives, name) in list {
        said.push(LocMsg::new("round-traitor").arg("name", name));
        let mut won = true;
        for (i, objective) in objectives.iter().enumerate() {
            let done = achieved(world, mob, objective);
            won &= done;
            said.push(objective.line(i + 1, if done { "won" } else { "failed" }));
        }
        said.push(LocMsg::new(if won {
            "round-traitor-won"
        } else {
            "round-traitor-lost"
        }));
    }
    said
}

/// Show each player the uplink they have open.
#[allow(clippy::type_complexity)]
pub fn update_uplink_views(
    mut viewers: Query<(Entity, Option<&PanelViewer>, &mut UplinkPanelOut)>,
    uplinks: Query<(&Uplink, &NetId, Option<&HeldBy>)>,
    mut commands: Commands,
) {
    for (mob, viewer, mut out) in &mut viewers {
        let target = viewer.and_then(|v| uplinks.get(v.0).ok());
        let view = target.and_then(|(uplink, id, held)| {
            held.is_some_and(|h| h.0 == mob).then(|| UplinkPanelView {
                target: *id,
                crystals: uplink.crystals,
                goods: GOODS
                    .iter()
                    .map(|(proto, cost)| ((*proto).to_owned(), *cost))
                    .collect(),
            })
        });
        if view.is_none() && target.is_some() {
            commands.entity(mob).remove::<PanelViewer>();
        }
        if out.0 != view {
            out.0 = view;
        }
    }
}
