//! Emergency wall lockers: three crowbars, three breath masks and three emergency oxygen
//! tanks behind glass, one of each handed out per click. Spec: `docs/reference/walllockers.md`.

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, GridPos, NetId, ObjectSpec, TilePos};

use crate::actions::ClickOutcome;
use crate::hands::{Hands, HeldBy};

/// The emergency locker prototype (Luna `walllocker/emerglocker`).
pub const EMERGENCY_LOCKER: &str = "emergency-locker";
/// What it holds, by prototype (Luna `item_1` to `item_3`).
pub const KINDS: [&str; 3] = ["crowbar", "breath-mask", "emergency-oxygen"];
/// How many of each it holds (Luna `amount`).
pub const AMOUNT: usize = 3;

/// A wall locker: what it holds of each kind.
#[derive(Component, Debug, Clone, PartialEq, Eq, Default)]
pub struct WallLocker {
    pub stock: [Vec<Entity>; 3],
}

/// On something kept in a wall locker (it is off the map).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InWallLocker(pub Entity);

/// Spawn a locker, full (Luna `New`).
pub fn spawn_locker(world: &mut World, pos: TilePos, id: NetId) -> Result<Entity, String> {
    let locker = world
        .spawn((GridPos(pos), id, Appearance::new(EMERGENCY_LOCKER, "")))
        .id();
    let mut stock: [Vec<Entity>; 3] = Default::default();
    for (kind, held) in KINDS.iter().zip(stock.iter_mut()) {
        for _ in 0..AMOUNT {
            let thing = crate::objects::spawn_object(world, &ObjectSpec::new(*kind), pos)?;
            world
                .entity_mut(thing)
                .remove::<GridPos>()
                .insert(InWallLocker(locker));
            held.push(thing);
        }
    }
    world.entity_mut(locker).insert(WallLocker { stock });
    Ok(locker)
}

/// Everything clicking wall lockers needs.
#[derive(SystemParam)]
pub struct WallLockerWork<'w, 's> {
    lockers: Query<'w, 's, (&'static GridPos, &'static mut WallLocker)>,
}

impl WallLockerWork<'_, '_> {
    pub fn contains(&self, target: Entity) -> bool {
        self.lockers.contains(target)
    }

    /// Luna `attack_hand`: an empty hand gets one of each kind still there, dropped on the
    /// locker's tile. `attackby`: one of its kinds goes back in while there is room. The
    /// caller checks reach.
    pub fn use_on(
        &mut self,
        target: Entity,
        hands: &mut Hands,
        held: Option<&str>,
        commands: &mut Commands,
    ) -> Option<ClickOutcome> {
        let (pos, mut locker) = self.lockers.get_mut(target).ok()?;
        let at = pos.0;
        let Some(item) = hands.active_item() else {
            let mut given = Vec::new();
            for (kind, held) in KINDS.iter().zip(locker.stock.iter_mut()) {
                if let Some(thing) = held.pop() {
                    commands
                        .entity(thing)
                        .remove::<InWallLocker>()
                        .insert(GridPos(at));
                    given.push(*kind);
                }
            }
            return Some(ClickOutcome {
                to_mob: Some(dispensed(&given)),
                seen: None,
            });
        };
        let Some(slot) = KINDS.iter().position(|k| Some(*k) == held) else {
            return Some(ClickOutcome::default());
        };
        if locker.stock[slot].len() >= AMOUNT {
            return Some(ClickOutcome::default());
        }
        *hands.slot_mut(hands.active) = None;
        commands
            .entity(item)
            .remove::<HeldBy>()
            .insert(InWallLocker(target));
        locker.stock[slot].push(item);
        Some(ClickOutcome {
            to_mob: Some(LocMsg::new("walllocker-put-back").arg("item", KINDS[slot])),
            seen: None,
        })
    }
}

/// "Dispensed a crowbar, breath mask and emergency oxygen tank.", or "It's empty...".
fn dispensed(given: &[&str]) -> LocMsg {
    match given {
        [] => LocMsg::new("walllocker-empty"),
        [a] => LocMsg::new("walllocker-dispensed-1").arg("first", *a),
        [a, b] => LocMsg::new("walllocker-dispensed-2")
            .arg("first", *a)
            .arg("second", *b),
        [a, b, c, ..] => LocMsg::new("walllocker-dispensed-3")
            .arg("first", *a)
            .arg("second", *b)
            .arg("third", *c),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_message_lists_what_was_given() {
        assert_eq!(dispensed(&[]).id, "walllocker-empty");
        assert_eq!(dispensed(&["crowbar"]).id, "walllocker-dispensed-1");
        assert_eq!(dispensed(&KINDS).id, "walllocker-dispensed-3");
    }
}
