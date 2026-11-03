//! Slipping on soap and banana peels (Luna's `HasEntered` on them). Spec: `docs/reference/combat.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, GridPos, Slot, TilePos};

use crate::actions::Outbox;
use crate::combat::COUNT_DS;
use crate::equipment::{Equipment, Wearable};
use crate::flashes::Weakened;
use crate::movement::Mob;
use crate::pulling::{Pulling, Stepped};
use crate::time::WorldTime;

/// What can be slipped on.
pub const SLIPPERY: [&str; 2] = ["soap", "banana-peel"];
/// How long a slip takes you off your feet, in Luna's counts (its `Stun(6)` and `Weaken(4)`).
const SLIP_COUNTS: f32 = 6.0;

/// Those who just stepped onto something slippery, without the shoes for it.
pub fn slip(world: &mut World) {
    let movers: Vec<Entity> = world
        .resource::<Stepped>()
        .0
        .iter()
        .map(|(mob, _)| *mob)
        .collect();
    if movers.is_empty() {
        return;
    }
    let mut things = world.query_filtered::<(&GridPos, &Appearance), Without<Mob>>();
    let slippery: Vec<(TilePos, String)> = things
        .iter(world)
        .filter(|(_, look)| SLIPPERY.contains(&look.sprite.as_str()))
        .map(|(at, look)| (at.0, look.sprite.clone()))
        .collect();
    if slippery.is_empty() {
        return;
    }
    let now = world.resource::<WorldTime>().deciseconds();
    for mob in movers {
        let Some(at) = world.get::<GridPos>(mob).map(|p| p.0) else {
            continue;
        };
        let Some((_, thing)) = slippery.iter().find(|(there, _)| *there == at) else {
            continue;
        };
        let safe = world
            .get::<Equipment>(mob)
            .and_then(|e| e.get(Slot::Shoes))
            .and_then(|shoes| world.get::<Wearable>(shoes))
            .is_some_and(|shoes| shoes.no_slip);
        if safe {
            continue;
        }
        world
            .entity_mut(mob)
            .remove::<Pulling>()
            .insert(Weakened(now + SLIP_COUNTS * COUNT_DS));
        if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
            outbox.0.push(LocMsg::new("slipped").arg("item", thing.clone()));
        }
    }
}
