//! Spawning map objects, items and player mobs.

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use rs13_world::{
    Appearance, Dense, DisplayName, GridPos, HandsView, MapLayer, MobStat, NetId, ObjectSpec, Opaque, Slot,
    TileMap, TilePos, WornView,
};

use crate::access::{AccessIds, IdCard};
use crate::actions::{Outbox, PendingActions};
use crate::combat::CombatIntent;
use crate::doors::Airlock;
use crate::environment::{Lungs, Surroundings};
use crate::equipment::{Equipment, ItemDef, TankGas, Wearable, WornBy};
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

/// Item prototypes from `content/items.ron`, by id.
#[derive(Resource, Debug)]
pub struct ItemProtos(pub HashMap<String, ItemDef>);

impl Default for ItemProtos {
    fn default() -> Self {
        let protos: Vec<ItemDef> =
            rs13_content::parse_ron("content/items.ron", include_str!("../content/items.ron"))
                .expect("built-in item prototypes are valid");
        Self(protos.into_iter().map(|p| (p.id.clone(), p)).collect())
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
    if matches!(proto, "cable" | "smes" | "apc") {
        return Ok(spawn_power_object(world, proto, pos));
    }
    if let Some(machine) = crate::machines::machine_bundle(proto, spec.dir) {
        let id = world.resource_mut::<NetIds>().alloc();
        let appearance = Appearance::new(machine.sprite, machine.state);
        let mut entity = world.spawn((GridPos(pos), id, appearance));
        machine.apply(&mut entity);
        return Ok(entity.id());
    }
    let def = world.resource::<ItemProtos>().0.get(proto).cloned();
    if proto != "airlock" && def.is_none() {
        return Err(format!("unknown prototype {proto:?} at {pos:?}"));
    }

    let id = world.resource_mut::<NetIds>().alloc();
    if proto == "airlock" {
        let airlock = Airlock {
            req_access: spec.access.clone(),
            ..Airlock::default()
        };
        let appearance = Appearance::new("airlock", airlock.state.name());
        let power = crate::power::PowerUser::new(crate::power::Channel::Equipment, 0.0);
        return Ok(world
            .spawn((GridPos(pos), id, appearance, airlock, Dense, Opaque, power))
            .id());
    }
    let def = def.expect("checked above");
    let item = Item { force: def.force };
    let mut item = world.spawn((GridPos(pos), id, Appearance::new(proto, ""), item));
    if let Some(slot) = def.slot.as_deref().and_then(Slot::parse) {
        item.insert(Wearable {
            slot,
            slowdown: def.slowdown,
            space: def.space,
            internals: def.internals,
        });
    }
    if let Some(tank) = &def.tank {
        item.insert(TankGas::filled(tank));
    }
    if proto == "id-card" {
        item.insert(IdCard {
            access: spec.access.clone(),
        });
    }
    Ok(item.id())
}

/// What every new crew member wears (Luna's assistant: grey jumpsuit, black shoes).
pub const STARTING_OUTFIT: [&str; 2] = ["grey-jumpsuit", "black-shoes"];

/// Spawn a human mob for a player at `pos`, dressed in the starting outfit.
pub fn spawn_player_mob(world: &mut World, pos: TilePos) -> Entity {
    world.init_resource::<NetIds>();
    let id = world.resource_mut::<NetIds>().alloc();
    let mob = spawn_bare_mob(world, pos, id);
    let mut equipment = Equipment::default();
    for proto in STARTING_OUTFIT {
        let Ok(item) = spawn_object(world, &ObjectSpec::new(proto), pos) else {
            continue;
        };
        let slot = world.get::<Wearable>(item).map(|w| w.slot);
        if let Some(slot) = slot {
            world.entity_mut(item).remove::<GridPos>().insert(WornBy(mob));
            equipment.0.insert(slot, item);
        }
    }
    world.entity_mut(mob).insert(equipment);
    mob
}

fn spawn_bare_mob(world: &mut World, pos: TilePos, id: NetId) -> Entity {
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
                WornView::default(),
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
                crate::equipment::PendingEquipment::default(),
                Outbox::default(),
                PlayerControlled,
            ),
        ))
        .id()
}

/// Cables, SMES units and APCs. An APC powers the area its tile is in.
fn spawn_power_object(world: &mut World, proto: &str, pos: TilePos) -> Entity {
    use crate::power::{Apc, Cable, PowerNode, SMES_CAPACITY, SMES_OUTPUT, Smes};
    let id = world.resource_mut::<NetIds>().alloc();
    let area = {
        let mut maps = world.query::<&MapLayer>();
        maps.iter(world)
            .next()
            .and_then(|m| m.0.area(pos))
            .unwrap_or_default()
            .to_owned()
    };
    let mut entity = world.spawn((GridPos(pos), id, PowerNode));
    match proto {
        "cable" => {
            entity.insert((Appearance::new("cable", ""), Cable));
        }
        "smes" => {
            let smes = Smes {
                charge: SMES_CAPACITY,
                capacity: SMES_CAPACITY,
                output: SMES_OUTPUT,
                online: true,
            };
            entity.insert((Appearance::new("smes", "on"), smes, Dense));
        }
        _ => {
            entity.insert((Appearance::new("apc", "on"), Apc::new(area)));
        }
    }
    entity.id()
}
