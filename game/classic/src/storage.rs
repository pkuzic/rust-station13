//! Containers: toolboxes and belts that hold other items. Spec: `docs/reference/storage.md`.

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, MobStat, NetId, StorageView};
use serde::Deserialize;

use crate::equipment::{EquipAction, PendingEquipment, Wearable, WornBy};
use crate::hands::{Hands, HeldBy, Item};
use crate::health::Vitals;

/// Items a container holds at most (Luna `contents.len >= 7`).
pub const STORAGE_SLOTS: usize = 7;

/// A container in an item prototype.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct StorageSpec {
    /// The only prototypes it takes (Luna `can_hold`), for any size. Empty: anything
    /// smaller than the container itself.
    #[serde(default)]
    pub holds: Vec<String>,
}

/// An item that holds others, in the order they went in.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct Storage {
    pub items: Vec<Entity>,
    pub holds: Vec<String>,
}

/// On an item inside a container. It has no `GridPos` and no `HeldBy` meanwhile.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InStorage(pub Entity);

/// The container whose contents a mob has open (Luna `s_active`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenStorage(pub Entity);

/// What the mob's open container shows, for the server to send when it changes.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct StorageOut(pub Option<StorageView>);

/// Why an item doesn't go in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    Full,
    TooBig,
    /// Not on the container's list (a belt and a sandwich).
    NotAllowed,
}

/// Luna's storage `attackby`: a list of allowed things takes them at any size (and never
/// another container); otherwise only things smaller than the container fit.
pub fn check_insert(
    storage: &Storage,
    container_size: u8,
    item: &str,
    item_size: u8,
    item_is_container: bool,
) -> Result<(), Refusal> {
    if !storage.holds.is_empty() {
        if !storage.holds.iter().any(|h| h == item) || item_is_container {
            return Err(Refusal::NotAllowed);
        }
    } else if item_size >= container_size {
        return Err(Refusal::TooBig);
    }
    if storage.items.len() >= STORAGE_SLOTS {
        return Err(Refusal::Full);
    }
    Ok(())
}

/// What `process_actions` needs for containers.
#[derive(SystemParam)]
pub struct StorageAccess<'w, 's> {
    pub storages: Query<'w, 's, &'static mut Storage>,
    pub inside: Query<'w, 's, &'static InStorage>,
    pub sizes: Query<'w, 's, (&'static Item, &'static Appearance)>,
    pub held: Query<'w, 's, &'static HeldBy>,
    pub worn: Query<'w, 's, (&'static WornBy, &'static Wearable)>,
    pub equipment: Query<'w, 's, &'static mut PendingEquipment>,
}

impl StorageAccess<'_, '_> {
    /// Whether `item` is in one of `mob`'s hands or worn by it (Luna `loc == user`).
    pub fn on(&self, mob: Entity, item: Entity) -> bool {
        self.held.get(item).is_ok_and(|h| h.0 == mob) || self.worn.get(item).is_ok_and(|(w, _)| w.0 == mob)
    }

    /// Clicking `target` with the active hand, when it is a container, something in a
    /// container, or something the mob carries. Returns `None` when it is none of these
    /// and the ordinary click rules apply; otherwise the messages to show the mob and
    /// everyone around.
    pub fn use_on(
        &mut self,
        mob: Entity,
        hands: &mut Hands,
        target: Entity,
        commands: &mut Commands,
    ) -> Option<StorageOutcome> {
        let active = hands.active_item();
        // Something in a container the mob carries: take it out into the empty hand.
        if let Ok(InStorage(container)) = self.inside.get(target).copied() {
            if active.is_none() && self.on(mob, container) {
                if let Ok(mut storage) = self.storages.get_mut(container) {
                    storage.items.retain(|i| *i != target);
                }
                *hands.slot_mut(hands.active) = Some(target);
                commands.entity(target).remove::<InStorage>().insert(HeldBy(mob));
            }
            return Some(StorageOutcome::default());
        }
        let carried = self.on(mob, target);
        if self.storages.contains(target) {
            match active {
                // Open it: only while carrying it (Luna `attack_hand` with `loc == user`);
                // from the floor, an empty hand picks it up as usual.
                None if carried => {
                    commands.entity(mob).insert(OpenStorage(target));
                    return Some(StorageOutcome::default());
                }
                None => return None,
                Some(item) if item != target => return Some(self.insert(hands, item, target, commands)),
                Some(_) => return Some(StorageOutcome::default()),
            }
        }
        if carried && active.is_none() {
            // Luna `attack_hand` on carried things: worn clothes come off into the hand,
            // and an item in the other hand moves to the active one.
            if let Ok((_, wearable)) = self.worn.get(target) {
                if let Ok(mut pending) = self.equipment.get_mut(mob) {
                    pending.0.push(EquipAction::Unequip(wearable.slot));
                }
            } else {
                let other = hands.active.other();
                if hands.get(other) == Some(target) {
                    *hands.slot_mut(other) = None;
                    *hands.slot_mut(hands.active) = Some(target);
                }
            }
            return Some(StorageOutcome::default());
        }
        None
    }

    fn insert(
        &mut self,
        hands: &mut Hands,
        item: Entity,
        container: Entity,
        commands: &mut Commands,
    ) -> StorageOutcome {
        let (Ok((item_info, item_look)), Ok((container_item, container_look))) =
            (self.sizes.get(item), self.sizes.get(container))
        else {
            return StorageOutcome::default();
        };
        let item_size = item_info.size;
        let item_sprite = item_look.sprite.clone();
        let container_sprite = container_look.sprite.clone();
        let container_size = container_item.size;
        let is_container = self.storages.contains(item);
        let Ok(mut storage) = self.storages.get_mut(container) else {
            return StorageOutcome::default();
        };
        match check_insert(&storage, container_size, &item_sprite, item_size, is_container) {
            Ok(()) => {
                storage.items.push(item);
                *hands.slot_mut(hands.active) = None;
                commands
                    .entity(item)
                    .remove::<HeldBy>()
                    .insert(InStorage(container));
                StorageOutcome {
                    to_mob: None,
                    seen: Some(
                        LocMsg::new("storage-added")
                            .arg("item", item_sprite)
                            .arg("target", container_sprite),
                    ),
                }
            }
            Err(Refusal::NotAllowed) => StorageOutcome {
                to_mob: Some(
                    LocMsg::new("storage-cannot-hold")
                        .arg("item", item_sprite)
                        .arg("target", container_sprite),
                ),
                seen: None,
            },
            // Luna says nothing when it is full or the thing is too big.
            Err(Refusal::Full | Refusal::TooBig) => StorageOutcome::default(),
        }
    }
}

/// Messages from a container click: one for the mob, one for everyone who can see it
/// (needs the mob's `$name` added).
#[derive(Debug, Default)]
pub struct StorageOutcome {
    pub to_mob: Option<LocMsg>,
    pub seen: Option<LocMsg>,
}

/// Keep each mob's open container view current; close it once the mob no longer carries
/// the container or is knocked out.
pub fn update_storage_views(
    mut mobs: Query<(Entity, Option<&OpenStorage>, &mut StorageOut, Option<&Vitals>)>,
    storages: Query<&Storage>,
    held: Query<&HeldBy>,
    worn: Query<&WornBy>,
    ids: Query<&NetId>,
    mut commands: Commands,
) {
    for (mob, open, mut out, vitals) in &mut mobs {
        let view = open.and_then(|OpenStorage(container)| {
            let carried = held.get(*container).is_ok_and(|h| h.0 == mob)
                || worn.get(*container).is_ok_and(|w| w.0 == mob);
            let awake = vitals.is_none_or(|v| v.stat == MobStat::Conscious);
            let storage = storages.get(*container).ok()?;
            (carried && awake).then(|| StorageView {
                target: ids.get(*container).copied().unwrap_or(NetId(0)),
                items: storage
                    .items
                    .iter()
                    .filter_map(|i| ids.get(*i).ok().copied())
                    .collect(),
                slots: STORAGE_SLOTS as u8,
            })
        });
        if view.is_none() && open.is_some() {
            commands.entity(mob).remove::<OpenStorage>();
        }
        if out.0 != view {
            out.0 = view;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toolbox() -> Storage {
        Storage::default()
    }

    #[test]
    fn a_toolbox_takes_smaller_things_up_to_seven() {
        let mut storage = toolbox();
        assert_eq!(check_insert(&storage, 4, "crowbar", 2, false), Ok(()));
        assert_eq!(
            check_insert(&storage, 4, "toolbox", 4, true),
            Err(Refusal::TooBig)
        );
        storage.items = vec![Entity::PLACEHOLDER; STORAGE_SLOTS];
        assert_eq!(check_insert(&storage, 4, "crowbar", 2, false), Err(Refusal::Full));
    }

    #[test]
    fn a_belt_takes_only_its_tools_at_any_size() {
        let belt = Storage {
            items: Vec::new(),
            holds: vec!["crowbar".into(), "multitool".into()],
        };
        assert_eq!(check_insert(&belt, 3, "crowbar", 2, false), Ok(()));
        assert_eq!(
            check_insert(&belt, 3, "id-card", 1, false),
            Err(Refusal::NotAllowed)
        );
    }
}
