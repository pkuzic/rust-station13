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

    crate::closets::take_in_loose_items(world);

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

    // Air: every door tile is a portal between zones, and every firedoor seals an edge.
    let mut doors = world.query_filtered::<&GridPos, With<Airlock>>();
    let portals: Vec<TilePos> = doors.iter(world).map(|p| p.0).collect();
    let mut firedoors = world.query::<(&GridPos, &crate::firedoors::Firedoor)>();
    let edges: Vec<(TilePos, rs13_world::Dir)> =
        firedoors.iter(world).filter_map(|(p, d)| d.edge(p.0)).collect();
    let atmos = crate::atmos::build(&layout, &portals, &edges);
    world.insert_resource(atmos);
    problems
}

/// Spawn one object. Fails on an unknown prototype or access id.
pub fn spawn_object(world: &mut World, spec: &ObjectSpec, pos: TilePos) -> Result<Entity, String> {
    let entity = spawn_thing(world, spec, pos)?;
    if crate::floors::lies_under_floor(spec) {
        world.entity_mut(entity).insert(crate::floors::UnderFloor);
    }
    Ok(entity)
}

fn spawn_thing(world: &mut World, spec: &ObjectSpec, pos: TilePos) -> Result<Entity, String> {
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
        return spawn_power_object(world, spec, pos);
    }
    world.init_resource::<crate::closets::ClosetProtos>();
    if world
        .resource::<crate::closets::ClosetProtos>()
        .0
        .contains_key(proto)
    {
        let id = world.resource_mut::<NetIds>().alloc();
        if let Some(closet) = crate::closets::spawn_closet(world, spec, pos, id) {
            return closet;
        }
    }
    if proto == crate::firealarms::FIRE_ALARM {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::firealarms::spawn_fire_alarm(world, spec, pos, id));
    }
    if proto == crate::cells::CELL_CHARGER {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::cells::spawn_cell_charger(world, spec, pos, id));
    }
    if proto == crate::dispensers::TANK_DISPENSER {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::dispensers::spawn_tank_dispenser(world, spec, pos, id));
    }
    if proto == crate::generators::PACMAN {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::generators::spawn_pacman(world, spec, pos, id));
    }
    if proto == "firedoor" {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::firedoors::spawn_firedoor(world, spec, pos, id));
    }
    if crate::lights::is_light_proto(proto) {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::lights::spawn_light(world, spec, pos, id));
    }
    if crate::furniture::is_furniture(proto) {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::furniture::spawn_furniture(world, spec, pos, id));
    }
    if proto == "meter" {
        let id = world.resource_mut::<NetIds>().alloc();
        let power =
            crate::power::PowerUser::new(crate::power::Channel::Equipment, crate::meters::METER_POWER);
        return Ok(world
            .spawn((
                GridPos(pos),
                id,
                Appearance::new(proto, "meterX"),
                crate::meters::Meter::default(),
                power,
            ))
            .id());
    }
    if proto == "fuel-tank" {
        let id = world.resource_mut::<NetIds>().alloc();
        let tank = crate::welding::FuelTank {
            fuel: crate::welding::TANK_FUEL,
        };
        return Ok(world
            .spawn((GridPos(pos), id, Appearance::new(proto, ""), tank, Dense))
            .id());
    }
    if let Some(machine) = crate::machines::machine_bundle(proto, spec.dir) {
        let id = world.resource_mut::<NetIds>().alloc();
        let appearance = Appearance::new(machine.sprite, machine.state.clone());
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
        let wiring = crate::wires::Wiring::default();
        return Ok(world
            .spawn((
                GridPos(pos),
                id,
                appearance,
                airlock,
                wiring,
                Dense,
                Opaque,
                power,
            ))
            .id());
    }
    let def = def.expect("checked above");
    let item = Item {
        force: def.force,
        size: def.size,
    };
    let look = def.base.clone().unwrap_or_else(|| proto.to_owned());
    let mut item = world.spawn((GridPos(pos), id, Appearance::new(look, ""), item));
    if let Some(slot) = def.slot.as_deref().and_then(Slot::parse) {
        item.insert(Wearable {
            slot,
            slowdown: def.slowdown,
            space: def.space,
            internals: def.internals,
            siemens: def.siemens,
            magnetic: def.magnetic,
        });
    }
    if let Some(tank) = &def.tank {
        item.insert(TankGas::filled(tank));
    }
    if def.conducts {
        item.insert(crate::hands::Conducts);
    }
    if let Some(max) = def.stack {
        item.insert(crate::hands::Stack { amount: max, max });
    }
    if proto == crate::welding::WELDER {
        item.insert(crate::welding::Welder::default());
    }
    if let Some(light) = crate::flashlights::kind(proto) {
        item.insert(light);
    }
    if let Some(cell) = crate::cells::kind(proto) {
        item.insert(cell);
    }
    if crate::jetpacks::is_jetpack(proto) {
        item.insert(crate::jetpacks::Jetpack::default());
    }
    if proto == "id-card" {
        item.insert(IdCard {
            access: spec.access.clone(),
        });
    }
    let container = item.id();
    if let Some(storage) = &def.storage {
        // Fill it as Luna's `New()` does.
        let mut items = Vec::new();
        for inner in &def.contents {
            let thing = spawn_object(world, &ObjectSpec::new(inner.as_str()), pos)?;
            world
                .entity_mut(thing)
                .remove::<GridPos>()
                .insert(crate::storage::InStorage(container));
            items.push(thing);
        }
        world.entity_mut(container).insert(crate::storage::Storage {
            items,
            holds: storage.holds.clone(),
        });
    }
    Ok(container)
}

/// A pile of `amount` of a stacking item (floor tiles, cable) on the floor at `pos`.
pub fn spawn_stack(world: &mut World, proto: &str, pos: TilePos, amount: u32) -> Result<Entity, String> {
    let pile = spawn_object(world, &ObjectSpec::new(proto), pos)?;
    if let Some(mut stack) = world.get_mut::<crate::hands::Stack>(pile) {
        stack.amount = amount.clamp(1, stack.max);
    }
    Ok(pile)
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
                Outbox::default(),
                crate::sound::Earshot::default(),
                PlayerControlled,
            ),
            // Machine windows and requests.
            (
                crate::equipment::PendingEquipment::default(),
                crate::wires::PendingWires::default(),
                crate::wires::WirePanelOut::default(),
                crate::alarms::PendingAlarmOps::default(),
                crate::alarms::AlarmPanelOut::default(),
                crate::storage::StorageOut::default(),
            ),
        ))
        .id()
}

/// Cables, SMES units and APCs. An APC powers the area its tile is in.
fn spawn_power_object(world: &mut World, spec: &ObjectSpec, pos: TilePos) -> Result<Entity, String> {
    use crate::power::{Apc, Cable, PowerNode, SMES_CAPACITY, SMES_OUTPUT, Smes};
    let cable = Cable::from_ends(&spec.ends);
    if spec.id == "cable" && cable.is_none() {
        return Err(format!("cable at {pos:?}: odd ends {:?}", spec.ends));
    }
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
    match spec.id.as_str() {
        "cable" => {
            let cable = cable.expect("checked above");
            entity.insert((Appearance::new("cable", cable.state()), cable));
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
    Ok(entity.id())
}
