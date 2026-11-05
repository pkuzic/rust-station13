//! Jobs (Luna's `jobs.dm`, `Equip_Rank`): each start point on the map is for one job, and whoever
//! starts there gets its ID card, its clothes and its tools. Spec: `docs/reference/jobs.md`.

use std::sync::OnceLock;

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{GridPos, Hand, Hidden, JobPanelView, ObjectSpec, Slot, TilePos};
use serde::Deserialize;

use crate::access::IdCard;
use crate::actions::Outbox;
use crate::equipment::{Equipment, Wearable, WornBy};
use crate::hands::{Hands, HeldBy};
use crate::movement::Mob;
use crate::objects::spawn_object;
use crate::storage::{InStorage, Storage};
use crate::time::WorldTime;

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

/// Someone just arrived and choosing a job, out of the world until they do, or until this
/// time, ds.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ChoosingJob(pub f32);

/// How long they have to choose, ds.
pub const CHOOSING_DS: f32 = 600.0;

/// What a mob answered: this line of the list, or `None` to stay as they are.
#[derive(Component, Debug, Default)]
pub struct PendingJobOps(pub Vec<Option<usize>>);

/// What a mob's job window shows.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct JobPanelOut(pub Option<JobPanelView>);

/// Mark a card placed for a job, which gives it the job's access (`content/jobs.ron`). A test map's
/// job that isn't on the list keeps whatever access it was placed with.
pub fn set_aside(world: &mut World, card: Entity, job: &str) {
    if let Some(job) = jobs().iter().find(|j| j.id == job) {
        world.entity_mut(card).insert(IdCard {
            access: job.access.clone(),
        });
    }
    world.entity_mut(card).insert((JobCard(job.to_owned()), Hidden));
}

/// `pick_start`, with every mob's tile taken.
pub fn next_start(world: &mut World, starts: &[TilePos]) -> Option<TilePos> {
    let mut mobs = world.query_filtered::<&GridPos, With<Mob>>();
    let taken: Vec<TilePos> = mobs.iter(world).map(|p| p.0).collect();
    pick_start(world, starts, &taken)
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

/// Put an item on someone, in place of whatever they wore there.
pub fn wear(world: &mut World, mob: Entity, item: Entity) {
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

/// Offer someone arriving at `at` the job waiting there, if there is one: they stay out of the
/// world, holding that post, until they've chosen. True if there was one.
pub fn offer(world: &mut World, mob: Entity, at: TilePos) -> bool {
    let mut cards = world.query::<(&JobCard, &GridPos)>();
    let Some(job) = cards
        .iter(world)
        .find(|(_, p)| p.0 == at)
        .map(|(j, _)| j.0.clone())
    else {
        return false;
    };
    let until = world.resource::<WorldTime>().deciseconds() + CHOOSING_DS;
    world
        .entity_mut(mob)
        .insert((HasJob(job), Post(at), ChoosingJob(until), Hidden));
    true
}

/// Whoever has settled at `at` gets the job waiting there, if there is one.
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
        // Luna's kits go in the left hand and people start with the right active: the hand that does
        // things stays free, as machines want an empty one.
        let free = match hands.active {
            Hand::Left => Hand::Right,
            Hand::Right => Hand::Left,
        };
        if hands.slot_mut(free).is_none() {
            *hands.slot_mut(free) = Some(item);
            world.entity_mut(item).remove::<GridPos>().insert(HeldBy(mob));
        }
    }
    world.entity_mut(mob).insert((HasJob(job.clone()), Post(at)));
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(LocMsg::new("job-assigned").arg("job", job.clone()));
    }
    Some(job)
}

/// Every job, and how many of its posts have nobody at them.
pub fn open_posts(world: &mut World) -> Vec<(String, u16)> {
    let mut mobs = world.query_filtered::<&GridPos, With<Mob>>();
    let taken: Vec<TilePos> = mobs.iter(world).map(|p| p.0).collect();
    let mut cards = world.query::<(&JobCard, &GridPos)>();
    let waiting: Vec<(String, TilePos)> = cards.iter(world).map(|(j, p)| (j.0.clone(), p.0)).collect();
    jobs()
        .iter()
        .map(|job| {
            let open = waiting
                .iter()
                .filter(|(j, p)| *j == job.id && !taken.contains(p))
                .count();
            (job.id.clone(), open as u16)
        })
        .collect()
}

/// The first free post of a job.
pub fn free_post(world: &mut World, job: &str) -> Option<TilePos> {
    let mut mobs = world.query_filtered::<&GridPos, With<Mob>>();
    let taken: Vec<TilePos> = mobs.iter(world).map(|p| p.0).collect();
    let mut cards = world.query::<(&JobCard, &GridPos)>();
    cards
        .iter(world)
        .find(|(j, p)| j.0 == job && !taken.contains(&p.0))
        .map(|(_, p)| p.0)
}

/// Someone has decided: out they come, at their post, with its card and kit.
fn settle(world: &mut World, mob: Entity) {
    let Some(post) = world.get::<Post>(mob).map(|p| p.0) else {
        return;
    };
    world.entity_mut(mob).remove::<(ChoosingJob, Hidden)>();
    take_up(world, mob, post);
}

/// Answers to the job window, and whoever has taken too long stays with the job they were offered
/// (Luna's job choice, offered as they arrive rather than before the round).
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
            let Some(line) = answer else {
                settle(world, mob);
                continue;
            };
            let Some(job) = jobs().get(line) else {
                continue;
            };
            // A job with no free post leaves the window open, to choose again.
            let Some(at) = free_post(world, &job.id) else {
                continue;
            };
            world.entity_mut(mob).insert((GridPos(at), Post(at)));
            settle(world, mob);
        }
    }
    let now = world.resource::<WorldTime>().deciseconds();
    let mut waiting = world.query::<(Entity, &ChoosingJob)>();
    let late: Vec<Entity> = waiting
        .iter(world)
        .filter(|(_, c)| now >= c.0)
        .map(|(e, _)| e)
        .collect();
    for mob in late {
        settle(world, mob);
    }
}

/// Show each new arrival the jobs still open.
pub fn update_job_views(
    mut mobs: Query<(Option<&HasJob>, Option<&ChoosingJob>, &mut JobPanelOut)>,
    cards: Query<(&JobCard, &GridPos)>,
    people: Query<&GridPos, With<Mob>>,
) {
    let taken: Vec<TilePos> = people.iter().map(|p| p.0).collect();
    for (job, choosing, mut out) in &mut mobs {
        let view = choosing.and(job).map(|job| JobPanelView {
            current: job.0.clone(),
            rows: jobs()
                .iter()
                .map(|j| {
                    let open = cards
                        .iter()
                        .filter(|(c, p)| c.0 == j.id && !taken.contains(&p.0))
                        .count();
                    (j.id.clone(), open as u16)
                })
                .collect(),
        });
        if out.0 != view {
            out.0 = view;
        }
    }
}
