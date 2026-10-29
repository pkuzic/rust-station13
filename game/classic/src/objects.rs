//! Spawning map objects, items and player mobs.

use std::collections::HashSet;

use bevy_ecs::prelude::*;
use rs13_world::{Appearance, Dense, GridPos, HandsView, MapLayer, NetId, TileMap, TilePos};
use serde::Deserialize;

use crate::actions::{Outbox, PendingActions};
use crate::doors::Airlock;
use crate::hands::{Hands, Item};
use crate::movement::{Mob, MoveIntent};

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

/// Spawn the map entity and every object it places. Returns the prototype ids that
/// are not known, so the caller can report them.
pub fn spawn_map(world: &mut World, map: TileMap) -> Vec<String> {
    world.init_resource::<NetIds>();
    world.init_resource::<ItemProtos>();
    let objects = map.objects.clone();
    let id = world.resource_mut::<NetIds>().alloc();
    world.spawn((MapLayer(map), id));

    let mut unknown = Vec::new();
    for (pos, proto) in objects {
        if spawn_object(world, &proto, pos).is_none() {
            unknown.push(proto);
        }
    }
    unknown
}

/// Spawn one object by prototype id, or `None` if the id is unknown.
pub fn spawn_object(world: &mut World, proto: &str, pos: TilePos) -> Option<Entity> {
    world.init_resource::<NetIds>();
    world.init_resource::<ItemProtos>();
    let id = world.resource_mut::<NetIds>().alloc();
    if proto == "airlock" {
        let airlock = Airlock::default();
        let appearance = Appearance::new("airlock", airlock.state.name());
        return Some(world.spawn((GridPos(pos), id, appearance, airlock, Dense)).id());
    }
    if world.resource::<ItemProtos>().0.contains(proto) {
        return Some(
            world
                .spawn((GridPos(pos), id, Appearance::new(proto, ""), Item))
                .id(),
        );
    }
    None
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
            Dense,
            Mob::default(),
            MoveIntent::default(),
            Hands::default(),
            HandsView::default(),
            PendingActions::default(),
            Outbox::default(),
            PlayerControlled,
        ))
        .id()
}
