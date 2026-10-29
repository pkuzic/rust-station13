//! Spawning map objects, items and player mobs.

use std::collections::HashSet;

use bevy_ecs::prelude::*;
use rs13_world::{
    Appearance, Dense, DisplayName, GridPos, HandsView, MapLayer, NetId, ObjectSpec, Opaque, TileMap, TilePos,
};
use serde::Deserialize;

use crate::access::{AccessIds, IdCard};
use crate::actions::{Outbox, PendingActions};
use crate::doors::Airlock;
use crate::hands::{Hands, Item};
use crate::movement::{Mob, MoveIntent};
use crate::names::default_name;
use crate::speech::PendingSpeech;

/// Hands out [`NetId`]s. Every entity clients need to see or name gets one.
#[derive(Resource, Debug, Default)]
pub struct NetIds {
    next: u64,
}

impl NetIds {
    pub fn alloc(&mut self) -> NetId {
        self.next += 1;
        NetId(self.next)
    }
}

#[derive(Deserialize)]
struct ItemProto {
    id: String,
}

/// Ids of all item prototypes, from `content/items.ron`.
#[derive(Resource, Debug)]
pub struct ItemProtos(pub HashSet<String>);

impl Default for ItemProtos {
    fn default() -> Self {
        let protos: Vec<ItemProto> =
            rs13_content::parse_ron("content/items.ron", include_str!("../content/items.ron"))
                .expect("built-in item prototypes are valid");
        Self(protos.into_iter().map(|p| p.id).collect())
    }
}

/// A player-controlled mob: walking into airlocks opens them (Luna requires `M.client`).
#[derive(Component, Debug)]
pub struct PlayerControlled;

/// Spawn the map entity and every object it places. Returns a description of each
/// object that could not be placed, so the caller can report them.
pub fn spawn_map(world: &mut World, map: TileMap) -> Vec<String> {
    world.init_resource::<NetIds>();
    let objects = map.objects.clone();
    let id = world.resource_mut::<NetIds>().alloc();
    world.spawn((MapLayer(map), id));

    objects
        .iter()
        .filter_map(|(pos, spec)| spawn_object(world, spec, *pos).err())
        .collect()
}

/// Spawn one object. Fails on an unknown prototype or access id.
pub fn spawn_object(world: &mut World, spec: &ObjectSpec, pos: TilePos) -> Result<Entity, String> {
    world.init_resource::<NetIds>();
    world.init_resource::<ItemProtos>();
    world.init_resource::<AccessIds>();
    if let Some(bad) = spec
        .access
        .iter()
        .find(|a| !world.resource::<AccessIds>().0.contains(*a))
    {
        return Err(format!("{} at {pos:?}: unknown access id {bad:?}", spec.id));
    }
    let proto = spec.id.as_str();
    let is_item = world.resource::<ItemProtos>().0.contains(proto);
    if proto != "airlock" && !is_item {
        return Err(format!("unknown prototype {proto:?} at {pos:?}"));
    }

    let id = world.resource_mut::<NetIds>().alloc();
    if proto == "airlock" {
        let airlock = Airlock {
            req_access: spec.access.clone(),
            ..Airlock::default()
        };
        let appearance = Appearance::new("airlock", airlock.state.name());
        return Ok(world
            .spawn((GridPos(pos), id, appearance, airlock, Dense, Opaque))
            .id());
    }
    let mut item = world.spawn((GridPos(pos), id, Appearance::new(proto, ""), Item));
    if proto == "id-card" {
        item.insert(IdCard {
            access: spec.access.clone(),
        });
    }
    Ok(item.id())
}

/// Spawn a human mob for a player at `pos`.
pub fn spawn_player_mob(world: &mut World, pos: TilePos) -> Entity {
    world.init_resource::<NetIds>();
    let id = world.resource_mut::<NetIds>().alloc();
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new("human", ""),
            DisplayName(default_name(id.0)),
            Dense,
            Mob::default(),
            MoveIntent::default(),
            Hands::default(),
            HandsView::default(),
            PendingActions::default(),
            PendingSpeech::default(),
            Outbox::default(),
            PlayerControlled,
        ))
        .id()
}
