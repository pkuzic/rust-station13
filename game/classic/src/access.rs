//! Access and ID cards. Spec: `docs/reference/access.md`.

use std::collections::BTreeSet;

use bevy_ecs::prelude::*;
use rs13_world::Slot;
use serde::Deserialize;

use crate::equipment::Equipment;
use crate::hands::Hands;

/// An ID card: the access ids it grants.
#[derive(Component, Debug, Clone, PartialEq, Eq, Default)]
pub struct IdCard {
    pub access: Vec<String>,
}

#[derive(Deserialize)]
struct AccessProto {
    id: String,
    #[allow(dead_code)]
    luna: u32,
}

/// Every known access id, from `content/access.ron`.
#[derive(Resource, Debug)]
pub struct AccessIds(pub BTreeSet<String>);

impl Default for AccessIds {
    fn default() -> Self {
        let protos: Vec<AccessProto> =
            rs13_content::parse_ron("content/access.ron", include_str!("../content/access.ron"))
                .expect("built-in access list is valid");
        Self(protos.into_iter().map(|p| p.id).collect())
    }
}

/// Luna's `check_access`: no requirements lets anyone through; otherwise the card must
/// grant **any one** of the required ids (Luna changed this from "all of them").
pub fn allowed(required: &[String], card: Option<&IdCard>) -> bool {
    if required.is_empty() {
        return true;
    }
    card.is_some_and(|card| required.iter().any(|r| card.access.contains(r)))
}

/// The cards someone shows when they use something: what's in their active hand and what's in
/// their ID slot. Everything that checks a person's access goes through this (Luna `allowed`).
#[derive(Debug, Clone, Copy, Default)]
pub struct Shown<'a> {
    pub held: Option<&'a IdCard>,
    pub worn: Option<&'a IdCard>,
}

impl<'a> Shown<'a> {
    /// Either card will do, so holding a card of no use doesn't undo the one being worn.
    pub fn opens(&self, required: &[String]) -> bool {
        allowed(required, self.held) || allowed(required, self.worn)
    }

    pub fn of(hands: Option<&Hands>, equipment: Option<&Equipment>, cards: &'a Query<&IdCard>) -> Self {
        Self {
            held: hands
                .and_then(Hands::active_item)
                .and_then(|item| cards.get(item).ok()),
            worn: equipment
                .and_then(|e| e.get(Slot::Id))
                .and_then(|item| cards.get(item).ok()),
        }
    }

    pub fn of_mob(world: &'a World, mob: Entity) -> Self {
        Self {
            held: world
                .get::<Hands>(mob)
                .and_then(Hands::active_item)
                .and_then(|item| world.get::<IdCard>(item)),
            worn: world
                .get::<Equipment>(mob)
                .and_then(|e| e.get(Slot::Id))
                .and_then(|item| world.get::<IdCard>(item)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(access: &[&str]) -> IdCard {
        IdCard {
            access: access.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn any_one_matching_access_is_enough() {
        let required = vec!["engine".to_owned(), "engine_equip".to_owned()];
        assert!(allowed(&required, Some(&card(&["engine_equip"]))));
        assert!(!allowed(&required, Some(&card(&["maint_tunnels"]))));
        assert!(!allowed(&required, None));
        assert!(allowed(&[], None));
    }

    #[test]
    fn access_list_loads() {
        let ids = AccessIds::default();
        assert!(ids.0.contains("engine_equip"));
    }
}
