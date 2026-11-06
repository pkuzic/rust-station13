//! Corporate restructuring (Luna's `restructuring` mode): head office names one of the crew to be
//! let go, and everyone is to see to it. Spec: `docs/reference/monkey-restructuring.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{DisplayName, HumanLook, MobStat};

use crate::actions::Outbox;
use crate::health::Vitals;
use crate::jobs::HasJob;
use crate::objects::PlayerControlled;
use crate::rng::GameRng;
use crate::time::WorldTime;

pub const MODE: &str = "restructuring";
/// How long after the start head office names someone (Luna's 120 ds), and how long it waits again
/// while there's nobody to name (30 ds).
pub const PICK_DS: f32 = 120.0;
pub const RETRY_DS: f32 = 30.0;

/// Present only in a restructuring round.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq)]
pub struct Restructuring {
    pub pick_at: Option<f32>,
    pub target: Option<Entity>,
}

/// Head office's word as the round starts (Luna's `announce`).
pub fn announcement() -> [LocMsg; 2] {
    [
        LocMsg::new("restructuring-announce"),
        LocMsg::new("restructuring-announce-care"),
    ]
}

pub fn setup_round(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    world.insert_resource(Restructuring {
        pick_at: Some(now + PICK_DS),
        target: None,
    });
}

pub fn run_restructuring(world: &mut World) {
    let Some(at) = world.get_resource::<Restructuring>().and_then(|r| r.pick_at) else {
        return;
    };
    let now = world.resource::<WorldTime>().deciseconds();
    if now < at {
        return;
    }
    let mut people =
        world.query_filtered::<(Entity, Option<&Vitals>), (With<PlayerControlled>, With<HumanLook>)>();
    let mut alive: Vec<Entity> = people
        .iter(world)
        .filter(|(_, v)| v.is_none_or(|v| v.stat != MobStat::Dead))
        .map(|(e, _)| e)
        .collect();
    alive.sort();
    if alive.is_empty() {
        world.resource_mut::<Restructuring>().pick_at = Some(now + RETRY_DS);
        return;
    }
    let i = world.resource_mut::<GameRng>().range(0, alive.len() as i32 - 1) as usize;
    let target = alive[i];
    *world.resource_mut::<Restructuring>() = Restructuring {
        pick_at: None,
        target: Some(target),
    };
    let name = world
        .get::<DisplayName>(target)
        .map_or_else(String::new, |n| n.0.clone());
    let accused = match world.get::<HasJob>(target) {
        Some(job) => LocMsg::new("restructuring-target-job")
            .arg("name", name)
            .msg("job", format!("job-{}", job.0)),
        None => LocMsg::new("restructuring-target").arg("name", name),
    };
    let mut everyone = world.query::<&mut Outbox>();
    for mut outbox in everyone.iter_mut(world) {
        outbox.0.push(accused.clone());
    }
    if let Some(mut outbox) = world.get_mut::<Outbox>(target) {
        outbox.0.push(LocMsg::new("restructuring-you"));
    }
}
