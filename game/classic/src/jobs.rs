//! Jobs (Luna's `jobs.dm`, `Equip_Rank`): each start point on the map is for one job, and whoever
//! starts there gets its ID card, its clothes and its tools. Spec: `docs/reference/jobs.md`.

use std::sync::OnceLock;

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{GridPos, Hidden, JobPanelView, ObjectSpec, Slot, TilePos};
use serde::Deserialize;

use crate::actions::Outbox;
use crate::equipment::{Equipment, Wearable, WornBy};
use crate::hands::{Hands, HeldBy};
use crate::movement::Mob;
use crate::objects::spawn_object;
use crate::storage::{InStorage, Storage};

/// A job's kit. Its order in `content/jobs.ron` is the order jobs fill up in.
#[derive(Debug, Deserialize)]
pub struct Job {
    pub id: String,
    /// What its ID card opens (Luna `get_access`).
    pub access: Vec<String>,
    /// Put on when they arrive.
    pub wears: Vec<String>,
    /// In their hands.
    pub holds: Vec<String>,
    /// In their backpack.
    pub packed: Vec<String>,
}

pub fn jobs() -> &'static [Job] {
    static JOBS: OnceLock<Vec<Job>> = OnceLock::new();
    JOBS.get_or_init(|| {
        rs13_content::parse_ron("content/jobs.ron", include_str!("../content/jobs.ron"))
            .expect("built-in job list is valid")
    })
}

/// On an ID card waiting at a start point: whose it is. Nobody can see or take it until someone
/// starts there.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct JobCard(pub String);

/// The job someone has.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct HasJob(pub String);

/// Whose a card is and the job on it (Luna `registered`, `assignment`).
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct CardOwner {
    pub owner: Option<Entity>,
    pub job: String,
}

/// Where someone took up their job, for when they swap it for another.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Post(pub TilePos);

/// Someone just arrived, who may still swap their job for another.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChoosingJob;

/// What a mob answered: this line of the list, or `None` to stay as they are.
#[derive(Component, Debug, Default)]
pub struct PendingJobOps(pub Vec<Option<usize>>);

/// What a mob's job window shows.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct JobPanelOut(pub Option<JobPanelView>);

/// Mark a card placed for a job.
pub fn set_aside(world: &mut World, card: Entity, job: &str) {
    world.entity_mut(card).insert((JobCard(job.to_owned()), Hidden));
}

/// Where the next player should start: a free start point of the job fewest people have, the
/// earlier in the list the better, or any free one on a map without jobs (Luna's
/// `DivideOccupations`, with no preferences to go by yet).
pub fn pick_start(world: &mut World, starts: &[TilePos], taken: &[TilePos]) -> Option<TilePos> {
    let free = |at: &&TilePos| !taken.contains(at);
    let mut cards = world.query::<(&JobCard, &GridPos)>();
    let waiting: Vec<(String, TilePos)> = cards.iter(world).map(|(j, p)| (j.0.clone(), p.0)).collect();
    let mut holders = world.query::<&HasJob>();
    let held: Vec<String> = holders.iter(world).map(|j| j.0.clone()).collect();
    let open = |job: &Job| {
        starts
            .iter()
            .filter(free)
            .find(|at| waiting.iter().any(|(j, p)| *j == job.id && p == *at))
            .copied()
    };
    jobs()
        .iter()
        .filter_map(|job| open(job).map(|at| (held.iter().filter(|h| **h == job.id).count(), at)))
        // `min_by_key` keeps the first of equals, so the list breaks ties.
        .min_by_key(|(count, _)| *count)
        .map(|(_, at)| at)
        .or_else(|| {
            starts
                .iter()
                .filter(free)
                .find(|at| !waiting.iter().any(|(_, p)| p == *at))
                .copied()
        })
        .or_else(|| starts.iter().find(free).copied())
}

fn wear(world: &mut World, mob: Entity, item: Entity) {
    let Some(slot) = world.get::<Wearable>(item).map(|w| w.slot) else {
        return;
    };
    let old = world.get::<Equipment>(mob).and_then(|e| e.get(slot));
    if let Some(old) = old {
        world.entity_mut(old).despawn();
    }
    world.entity_mut(item).remove::<GridPos>().insert(WornBy(mob));
    if let Some(mut equipment) = world.get_mut::<Equipment>(mob) {
        equipment.0.insert(slot, item);
    }
}

/// Whoever has just started at `at` gets the job waiting there, if there is one.
pub fn take_up(world: &mut World, mob: Entity, at: TilePos) -> Option<String> {
    let mut cards = world.query::<(Entity, &JobCard, &GridPos)>();
    let (card, job) = cards
        .iter(world)
        .find(|(_, _, p)| p.0 == at)
        .map(|(e, j, _)| (e, j.0.clone()))?;
    world
        .entity_mut(card)
        .remove::<(JobCard, Hidden)>()
        .insert(CardOwner {
            owner: Some(mob),
            job: job.clone(),
        });
    wear(world, mob, card);
    let kit = jobs().iter().find(|j| j.id == job)?;
    for proto in &kit.wears {
        if let Ok(item) = spawn_object(world, &ObjectSpec::new(proto.as_str()), at) {
            wear(world, mob, item);
        }
    }
    let pack = world.get::<Equipment>(mob).and_then(|e| e.get(Slot::Back));
    for proto in &kit.packed {
        let (Some(pack), Ok(item)) = (pack, spawn_object(world, &ObjectSpec::new(proto.as_str()), at)) else {
            continue;
        };
        world.entity_mut(item).remove::<GridPos>().insert(InStorage(pack));
        if let Some(mut storage) = world.get_mut::<Storage>(pack) {
            storage.items.push(item);
        }
    }
    for proto in &kit.holds {
        let Ok(item) = spawn_object(world, &ObjectSpec::new(proto.as_str()), at) else {
            continue;
        };
        let Some(mut hands) = world.get_mut::<Hands>(mob) else {
            continue;
        };
        let active = hands.active;
        if hands.slot_mut(active).is_none() {
            *hands.slot_mut(active) = Some(item);
            world.entity_mut(item).remove::<GridPos>().insert(HeldBy(mob));
        }
    }
    world
        .entity_mut(mob)
        .insert((HasJob(job.clone()), Post(at), ChoosingJob));
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(LocMsg::new("job-assigned").arg("job", job.clone()));
    }
    Some(job)
}

fn despawn_with_contents(world: &mut World, item: Entity) {
    let inside = world
        .get::<Storage>(item)
        .map(|s| s.items.clone())
        .unwrap_or_default();
    for thing in inside {
        despawn_with_contents(world, thing);
    }
    world.entity_mut(item).despawn();
}

/// Swap someone's job for the one on `line` of the list: their kit goes, their card goes back to
/// its post, and they start again at a free post of the new job.
fn change_job(world: &mut World, mob: Entity, line: usize) -> bool {
    let Some(new) = jobs().get(line) else {
        return false;
    };
    let mut mobs = world.query_filtered::<&GridPos, With<Mob>>();
    let taken: Vec<TilePos> = mobs.iter(world).map(|p| p.0).collect();
    let mut cards = world.query::<(&JobCard, &GridPos)>();
    let Some(at) = cards
        .iter(world)
        .find(|(j, p)| j.0 == new.id && !taken.contains(&p.0))
        .map(|(_, p)| p.0)
    else {
        return false;
    };
    let (Some(old), Some(post)) = (
        world.get::<HasJob>(mob).map(|j| j.0.clone()),
        world.get::<Post>(mob).map(|p| p.0),
    ) else {
        return false;
    };
    let card = world.get::<Equipment>(mob).and_then(|e| e.get(Slot::Id));
    let worn: Vec<Entity> = world
        .get::<Equipment>(mob)
        .map(|e| e.0.values().copied().filter(|i| Some(*i) != card).collect())
        .unwrap_or_default();
    for item in worn {
        despawn_with_contents(world, item);
    }
    if let Some(mut equipment) = world.get_mut::<Equipment>(mob) {
        equipment.0.clear();
    }
    let held: Vec<Entity> = world
        .get::<Hands>(mob)
        .map(|h| h.left.into_iter().chain(h.right).collect())
        .unwrap_or_default();
    for item in held {
        despawn_with_contents(world, item);
    }
    if let Some(mut hands) = world.get_mut::<Hands>(mob) {
        hands.left = None;
        hands.right = None;
    }
    if let Some(card) = card {
        world
            .entity_mut(card)
            .remove::<WornBy>()
            .remove::<CardOwner>()
            .insert((GridPos(post), JobCard(old), Hidden));
    }
    world.entity_mut(mob).insert(GridPos(at));
    take_up(world, mob, at);
    true
}

/// Answers to the job window (Luna's job choice, offered on arrival rather than before the round).
pub fn run_job_choices(world: &mut World) {
    let mut askers = world.query::<(Entity, &mut PendingJobOps)>();
    let asked: Vec<(Entity, Vec<Option<usize>>)> = askers
        .iter_mut(world)
        .filter(|(_, ops)| !ops.0.is_empty())
        .map(|(e, mut ops)| (e, std::mem::take(&mut ops.0)))
        .collect();
    for (mob, answers) in asked {
        for answer in answers {
            if world.get::<ChoosingJob>(mob).is_none() {
                continue;
            }
            if let Some(line) = answer {
                change_job(world, mob, line);
            }
            world.entity_mut(mob).remove::<ChoosingJob>();
        }
    }
}

/// Show each new arrival the jobs still open.
pub fn update_job_views(
    mut mobs: Query<(Option<&HasJob>, Option<&ChoosingJob>, &mut JobPanelOut)>,
    cards: Query<&JobCard>,
) {
    for (job, choosing, mut out) in &mut mobs {
        let view = choosing.and(job).map(|job| JobPanelView {
            current: job.0.clone(),
            rows: jobs()
                .iter()
                .map(|j| {
                    let open = cards.iter().filter(|c| c.0 == j.id).count();
                    (j.id.clone(), open as u16)
                })
                .collect(),
        });
        if out.0 != view {
            out.0 = view;
        }
    }
}
