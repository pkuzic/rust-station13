//! Spawning map objects, items and player mobs.

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use rs13_world::{
    Appearance, Dense, DisplayName, GridPos, HandsView, MapLayer, MobStat, NetId, ObjectSpec, Opaque,
    TileMap, TilePos,
};
use serde::Deserialize;

use crate::access::{AccessIds, IdCard};
use crate::actions::{Outbox, PendingActions};
use crate::combat::CombatIntent;
use crate::doors::Airlock;
use crate::environment::{Lungs, Surroundings};
use crate::hands::{Hands, Item};
use crate::health::{Body, HEALTH_FULL, Vitals};
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
    force: f32,
}

/// Item prototypes from `content/items.ron`: id to force.
#[derive(Resource, Debug)]
pub struct ItemProtos(pub HashMap<String, f32>);

impl Default for ItemProtos {
    fn default() -> Self {
        let protos: Vec<ItemProto> =
            rs13_content::parse_ron("content/items.ron", include_str!("../content/items.ron"))
                .expect("built-in item prototypes are valid");
        Self(protos.into_iter().map(|p| (p.id, p.force)).collect())
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
    let layout = map.clone();
    let id = world.resource_mut::<NetIds>().alloc();
    world.spawn((MapLayer(map), id, rs13_world::AtmosView::default()));

    let problems = objects
        .iter()
        .filter_map(|(pos, spec)| spawn_object(world, spec, *pos).err())
        .collect();

    // Canisters placed on a connector port start connected to its pipes.
    let mut ports = world.query_filtered::<&GridPos, With<crate::machines::Connector>>();
    let port_tiles: Vec<TilePos> = ports.iter(world).map(|p| p.0).collect();
    let mut canisters = world
        .query_filtered::<(&GridPos, &mut crate::machines::PipeMember), With<crate::machines::Canister>>();
    for (pos, mut member) in canisters.iter_mut(world) {
        member.open = port_tiles.contains(&pos.0);
    }
    world.init_resource::<crate::machines::PipeNets>();
    world.resource_mut::<crate::machines::PipeNets>().dirty = true;

    // Air: every door tile is a portal between zones.
    let mut doors = world.query_filtered::<&GridPos, With<Airlock>>();
    let portals: Vec<TilePos> = doors.iter(world).map(|p| p.0).collect();
    let atmos = crate::atmos::build(&layout, &portals);
    world.insert_resource(atmos);
    problems
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
    if let Some(machine) = crate::machines::machine_bundle(proto, spec.dir) {
        let id = world.resource_mut::<NetIds>().alloc();
        let appearance = Appearance::new(machine.sprite, machine.state);
        let mut entity = world.spawn((GridPos(pos), id, appearance));
        machine.apply(&mut entity);
        return Ok(entity.id());
    }
    let force = world.resource::<ItemProtos>().0.get(proto).copied();
    if proto != "airlock" && force.is_none() {
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
    let item = Item {
        force: force.unwrap_or(0.0),
    };
    let mut item = world.spawn((GridPos(pos), id, Appearance::new(proto, ""), item));
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
            // What others see.
            (
                GridPos(pos),
                id,
                Appearance::new("human", ""),
                DisplayName(default_name(id.0)),
                Dense,
                HandsView::default(),
            ),
            // What the mob is and does.
            (
                Mob::default(),
                MoveIntent::default(),
                Hands::default(),
                PendingActions::default(),
                PendingSpeech::default(),
                Body::default(),
                Vitals {
                    health: HEALTH_FULL,
                    stat: MobStat::Conscious,
                },
                CombatIntent::default(),
                Lungs::default(),
                Surroundings::default(),
                Outbox::default(),
                PlayerControlled,
            ),
        ))
        .id()
}
