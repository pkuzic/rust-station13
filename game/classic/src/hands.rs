//! Hands and held items. Spec: `docs/reference/hands.md`.

use bevy_ecs::prelude::*;
use rs13_world::{Hand, HandsView, NetId};

/// Something that can be picked up.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Item {
    /// Brute damage when hitting someone with it (Luna `force`).
    pub force: f32,
    /// Luna `w_class`: what it fits into.
    pub size: u8,
}

/// Electricity goes through it: it can shock whoever holds it into something live (Luna
/// `CONDUCT`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Conducts;

/// Several of the same item piled into one, such as lengths of cable in a coil.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stack {
    pub amount: u32,
    pub max: u32,
}

/// On a held item: the mob holding it. A held item has no `GridPos`.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeldBy(pub Entity);

/// A mob's two hands.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Hands {
    pub left: Option<Entity>,
    pub right: Option<Entity>,
    pub active: Hand,
}

impl Hands {
    pub fn get(&self, hand: Hand) -> Option<Entity> {
        match hand {
            Hand::Left => self.left,
            Hand::Right => self.right,
        }
    }

    pub fn slot_mut(&mut self, hand: Hand) -> &mut Option<Entity> {
        match hand {
            Hand::Left => &mut self.left,
            Hand::Right => &mut self.right,
        }
    }

    pub fn active_item(&self) -> Option<Entity> {
        self.get(self.active)
    }
}

/// Mirror `Hands` into the replicated `HandsView`, naming items by `NetId`.
pub fn sync_hands_view(mut mobs: Query<(&Hands, &mut HandsView)>, ids: Query<&NetId>) {
    for (hands, mut view) in &mut mobs {
        let id = |e: Option<Entity>| e.and_then(|e| ids.get(e).ok().copied());
        let new = HandsView {
            left: id(hands.left),
            right: id(hands.right),
            active: hands.active,
        };
        // Only write on change so replication sends nothing for idle mobs.
        view.set_if_neq(new);
    }
}
