//! Jobs (Luna's `jobs.dm`, `Equip_Rank`): each start point on the map is for one job, and whoever
//! starts there gets its ID card, its clothes and its tools. Spec: `docs/reference/jobs.md`.

use std::sync::OnceLock;

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{GridPos, Hidden, ObjectSpec, Slot, TilePos};
use serde::Deserialize;

use crate::actions::Outbox;
use crate::equipment::{Equipment, Wearable, WornBy};
use crate::hands::{Hands, HeldBy};
use crate::objects::spawn_object;
use crate::storage::{InStorage, Storage};

/// A job's kit. Its order in `content/jobs.ron` is the order jobs fill up in.
#[derive(Debug, Deserialize)]
pub struct Job {
    pub id: String,
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
    world.entity_mut(card).remove::<(JobCard, Hidden)>();
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
    world.entity_mut(mob).insert(HasJob(job.clone()));
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(LocMsg::new("job-assigned").arg("job", job.clone()));
    }
    Some(job)
}
