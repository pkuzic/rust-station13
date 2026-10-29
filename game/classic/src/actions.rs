//! Player and NPC actions other than movement: clicking things, dropping, swapping
//! hands, examining. Spec: `docs/reference/hands.md`.

use std::collections::HashSet;

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, GridPos, TilePos};

use crate::doors::Airlock;
use crate::hands::{Hands, HeldBy, Item};
use crate::movement::Mob;
use crate::time::WorldTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Click on something with the active hand: pick it up, or use it / the held item on it.
    UseOn(Entity),
    /// Put the active-hand item on the floor.
    Drop,
    SwapHands,
    Examine(Entity),
}

/// Actions waiting for the next tick, in the order they were requested.
#[derive(Component, Debug, Default)]
pub struct PendingActions(pub Vec<Action>);

/// Messages for the player controlling this mob, waiting to be sent.
#[derive(Component, Debug, Default)]
pub struct Outbox(pub Vec<LocMsg>);

/// Same tile or one of the eight neighbours, on the same deck.
pub fn in_reach(a: TilePos, b: TilePos) -> bool {
    a.z == b.z && (a.x - b.x).abs() <= 1 && (a.y - b.y).abs() <= 1
}

type ActingMob<'a> = (
    Entity,
    &'a GridPos,
    &'a mut Hands,
    &'a mut PendingActions,
    &'a mut Outbox,
);

pub fn process_actions(
    time: Res<WorldTime>,
    mut mobs: Query<ActingMob, With<Mob>>,
    mut targets: Query<(Option<&GridPos>, Has<Item>, Option<&mut Airlock>), Without<Mob>>,
    appearances: Query<&Appearance>,
    mut commands: Commands,
) {
    let now = time.deciseconds();
    // Items picked up earlier this tick (component changes apply after the system).
    let mut taken: HashSet<Entity> = HashSet::new();

    for (mob, mob_pos, mut hands, mut pending, mut outbox) in &mut mobs {
        for action in pending.0.drain(..) {
            match action {
                Action::SwapHands => hands.active = hands.active.other(),
                Action::Drop => {
                    let active = hands.active;
                    if let Some(item) = hands.slot_mut(active).take() {
                        commands
                            .entity(item)
                            .remove::<HeldBy>()
                            .insert(GridPos(mob_pos.0));
                    }
                }
                Action::Examine(target) => {
                    if let Ok(appearance) = appearances.get(target) {
                        outbox
                            .0
                            .push(LocMsg::new("examine").arg("item", appearance.sprite.clone()));
                    }
                }
                Action::UseOn(target) => {
                    let Ok((Some(target_pos), is_item, airlock)) = targets.get_mut(target) else {
                        // Held by someone, or gone.
                        continue;
                    };
                    if !in_reach(mob_pos.0, target_pos.0) {
                        continue;
                    }
                    if is_item && hands.active_item().is_none() && taken.insert(target) {
                        let active = hands.active;
                        *hands.slot_mut(active) = Some(target);
                        commands.entity(target).remove::<GridPos>().insert(HeldBy(mob));
                    } else if let Some(mut airlock) = airlock {
                        // By hand or with any item: Luna's door `attackby` ignores what it is.
                        airlock.toggle(now);
                    }
                }
            }
        }
    }
}
