//! AutoTraitor (Luna's `Extend-A-Traitormongous`): a traitor round that keeps adding traitors as
//! people come and go. Spec: `docs/reference/autotraitor.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::MobStat;

use crate::escape::{EscapePods, PodState};
use crate::health::Vitals;
use crate::objects::PlayerControlled;
use crate::rng::GameRng;
use crate::time::WorldTime;
use crate::traitor::{Traitor, WantsTraitor, choose_some, make_traitor};

pub const MODE: &str = "autotraitor";
/// How often it looks to add a traitor, ds (Luna's 9000).
pub const CHECK_DS: f32 = 9000.0;

/// Present only in an AutoTraitor round: when it next looks.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq)]
pub struct AutoTraitor {
    pub check_at: f32,
}

/// The most traitors for so many players, and the chance in percent of one more
/// (Luna: a tenth of them and one, and the rest of the tens tenfold).
pub fn start_odds(players: usize) -> (usize, f32) {
    let most = players / 10 + 1;
    (most, (players - (most - 1) * 10) as f32 * 10.0)
}

/// The same, later on, with this many traitors alive (Luna: fivefold, and 50 more while two short).
pub fn later_odds(players: usize, traitors: usize) -> (usize, f32) {
    let most = players / 10 + 1;
    let mut chance = (players - (most - 1) * 10) as f32 * 5.0;
    if traitors + 1 < most {
        chance += 50.0;
    }
    (most, chance)
}

/// Words of welcome after the traitor round's (Luna's `announce`).
pub fn announcement() -> LocMsg {
    LocMsg::new("autotraitor-announce")
}

pub fn setup_round(world: &mut World) {
    choose_some(world, |players, rng| {
        let (most, chance) = start_odds(players);
        most - 1 + usize::from(rng.prob(chance))
    });
    let now = world.resource::<WorldTime>().deciseconds();
    world.insert_resource(AutoTraitor {
        check_at: now + CHECK_DS,
    });
}

/// Living players, living traitors, and those who'd like to be one and aren't.
fn count(world: &mut World) -> (usize, usize, Vec<Entity>) {
    let mut people = world
        .query_filtered::<(Entity, Option<&Vitals>, Has<Traitor>, Has<WantsTraitor>), With<PlayerControlled>>(
        );
    let living: Vec<(Entity, bool, bool)> = people
        .iter(world)
        .filter(|(_, v, ..)| v.is_some_and(|v| v.stat != MobStat::Dead))
        .map(|(e, _, t, w)| (e, t, w))
        .collect();
    let traitors = living.iter().filter(|(_, t, _)| *t).count();
    let mut wanting: Vec<Entity> = living
        .iter()
        .filter(|(_, t, w)| !*t && *w)
        .map(|(e, ..)| *e)
        .collect();
    wanting.sort();
    (living.len(), traitors, wanting)
}

fn departed(world: &World) -> bool {
    world
        .get_resource::<EscapePods>()
        .is_some_and(|pods| matches!(pods.state, PodState::Transit { .. } | PodState::Arrived))
}

pub fn run_autotraitor(world: &mut World) {
    let Some(state) = world.get_resource::<AutoTraitor>().copied() else {
        return;
    };
    let now = world.resource::<WorldTime>().deciseconds();
    if now < state.check_at || departed(world) {
        return;
    }
    world.resource_mut::<AutoTraitor>().check_at = now + CHECK_DS;
    let (players, traitors, wanting) = count(world);
    let (most, chance) = later_odds(players, traitors);
    if traitors >= most || !world.resource_mut::<GameRng>().prob(chance) || wanting.is_empty() {
        return;
    }
    let i = world.resource_mut::<GameRng>().range(0, wanting.len() as i32 - 1) as usize;
    make_traitor(world, wanting[i], &["traitor-debt", "traitor-now"]);
}

/// Someone joining late who'd like to be a traitor may be made one (Luna's `latespawn`).
pub fn late_join(world: &mut World, mob: Entity) {
    if world.get_resource::<AutoTraitor>().is_none()
        || departed(world)
        || world.get::<WantsTraitor>(mob).is_none()
    {
        return;
    }
    let (players, traitors, _) = count(world);
    let (most, chance) = later_odds(players, traitors);
    if traitors < most && world.resource_mut::<GameRng>().prob(chance) {
        make_traitor(world, mob, &["traitor-you"]);
    }
}
