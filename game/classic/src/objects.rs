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
    spawn_station(world, vec![map])
}

/// Spawn a station of several decks (each with its own `z`): a map entity per deck, every
/// object they place, and one air simulation across them all.
pub fn spawn_station(world: &mut World, decks: Vec<TileMap>) -> Vec<String> {
    world.init_resource::<NetIds>();
    let layouts = decks.clone();
    let mut objects = Vec::new();
    for map in decks {
        objects.extend(map.objects.clone());
        let id = world.resource_mut::<NetIds>().alloc();
        world.spawn((MapLayer(map), id, rs13_world::AtmosView::default()));
    }

    let problems = objects
        .iter()
        .filter_map(|(pos, spec)| spawn_object(world, spec, *pos).err())
        .collect();

    crate::closets::take_in_loose_items(world);

    // Canisters and other portables placed on a connector port start connected to its pipes.
    let mut ports = world.query_filtered::<&GridPos, With<crate::machines::Connector>>();
    let port_tiles: Vec<TilePos> = ports.iter(world).map(|p| p.0).collect();
    let mut canisters = world
        .query_filtered::<(&GridPos, &mut crate::machines::PipeMember), With<crate::machines::Portable>>();
    for (pos, mut member) in canisters.iter_mut(world) {
        member.open = port_tiles.contains(&pos.0);
    }
    world.init_resource::<crate::machines::PipeNets>();
    world.resource_mut::<crate::machines::PipeNets>().dirty = true;

    // The shield field, and the edges it seals while up.
    let field_edges = crate::shields::lay_field(world, &layouts);

    // Air: every door tile (airlocks, blast doors) is a portal between zones, and every
    // firedoor and window pane seals an edge.
    let mut doors =
        world.query_filtered::<&GridPos, Or<(With<Airlock>, With<crate::blastdoors::BlastDoor>)>>();
    let portals: Vec<TilePos> = doors.iter(world).map(|p| p.0).collect();
    let mut firedoors = world.query::<(&GridPos, &crate::firedoors::Firedoor)>();
    let edges: Vec<(TilePos, rs13_world::Dir)> =
        firedoors.iter(world).filter_map(|(p, d)| d.edge(p.0)).collect();
    let layout_refs: Vec<&TileMap> = layouts.iter().collect();
    let mut atmos = crate::atmos::build(&layout_refs, &portals, &edges);
    for (at, dir) in field_edges {
        atmos.0.add_edge(at, dir);
    }
    // Window panes seal theirs for good.
    let mut panes = world.query::<(&GridPos, &crate::windows::WindowPane)>();
    for (at, dir) in panes.iter(world).map(|(p, w)| w.edge(p.0)) {
        atmos.0.add_edge(at, dir);
        atmos.0.set_edge_open(at, dir, false);
    }
    // Window doors seal theirs while shut.
    let mut windoors = world.query::<(&GridPos, &crate::windoors::Windoor)>();
    for (at, dir) in windoors.iter(world).map(|(p, w)| w.edge(p.0)) {
        atmos.0.add_edge(at, dir);
        atmos.0.set_edge_open(at, dir, false);
    }
    world.insert_resource(atmos);
    problems
}

/// How far into the wall it faces an APC is drawn, pixels (Luna `apc/New`).
const APC_WALL_PX: i32 = 24;

/// Spawn one object. Fails on an unknown prototype or access id.
pub fn spawn_object(world: &mut World, spec: &ObjectSpec, pos: TilePos) -> Result<Entity, String> {
    let entity = spawn_thing(world, spec, pos)?;
    // Tagged for an airlock console (`airlock-control.md`).
    let tag = |name: &str| {
        spec.vars
            .get(name)
            .map(|v| v.trim().trim_matches('"').to_owned())
            .filter(|v| !v.is_empty())
    };
    if spec.id == "id-card"
        && let Some(job) = tag("job")
    {
        crate::jobs::set_aside(world, entity, &job);
    }
    if spec.id == "airlock"
        && let Some(tag) = tag("id_tag")
    {
        world
            .entity_mut(entity)
            .insert(crate::airlock_control::DoorTag(tag));
    }
    if spec.id == crate::windoors::BRIG_WINDOOR
        && let Some(tag) = tag("id")
    {
        world.entity_mut(entity).insert(crate::brig::CellDoor(tag));
    }
    if spec.id == "vent"
        && let Some(tag) = tag("id")
    {
        world
            .entity_mut(entity)
            .insert(crate::airlock_control::PumpTag(tag));
    }
    if crate::floors::lies_under_floor(spec) {
        world.entity_mut(entity).insert(crate::floors::UnderFloor);
    }
    // Hung on a wall by the map (BYOND `pixel_x`, `pixel_y`): drawn there, kept on its tile.
    let pixel = |name: &str| {
        spec.vars
            .get(name)
            .and_then(|v| v.trim().parse::<i32>().ok())
            .map_or(0, |v| v.clamp(-127, 127) as i8)
    };
    let mut offset = [pixel("pixel_x"), pixel("pixel_y")];
    // An APC hangs on the wall it faces, 24 pixels over (Luna `apc/New`).
    if offset == [0, 0]
        && spec.id == "apc"
        && let Some(dir) = spec.dir
    {
        let (dx, dy) = dir.offset();
        offset = [(dx * APC_WALL_PX) as i8, (dy * APC_WALL_PX) as i8];
    }
    if offset != [0, 0] {
        world.entity_mut(entity).insert(rs13_world::PixelOffset(offset));
    }
    // Placed facing a way: drawn that way.
    if let Some(dir) = spec.dir {
        world.entity_mut(entity).insert(rs13_world::Facing::of(dir));
    }
    Ok(entity)
}

fn spawn_thing(world: &mut World, spec: &ObjectSpec, pos: TilePos) -> Result<Entity, String> {
    world.init_resource::<NetIds>();
    world.init_resource::<crate::rng::GameRng>();
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
    if matches!(proto, "cable" | "smes" | "apc" | "terminal") {
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
    if crate::medbay::is_medbay_part(proto) {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::medbay::spawn_part(world, proto, pos, id));
    }
    if proto == crate::autolathe::AUTOLATHE {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::autolathe::spawn_autolathe(world, pos, id));
    }
    if crate::scenery::is_scenery(proto) {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::scenery::spawn_scenery(world, proto, pos, id));
    }
    if crate::reagent_tanks::is_tank(proto) {
        let id = world.resource_mut::<NetIds>().alloc();
        if let Some(tank) = crate::reagent_tanks::spawn_tank(world, proto, pos, id) {
            return Ok(tank);
        }
    }
    if proto == crate::chem_master::CHEM_MASTER {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::chem_master::spawn_chem_master(world, pos, id));
    }
    if proto == crate::chem_dispenser::CHEM_DISPENSER {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::chem_dispenser::spawn_chem_dispenser(world, pos, id));
    }
    if crate::security::is_turret_part(proto) {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::security::spawn_turret_part(world, spec, pos, id));
    }
    if proto == crate::weapons::RECHARGER {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::weapons::spawn_recharger(world, pos, id));
    }
    if proto == crate::brig::DOOR_TIMER {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::brig::spawn_door_timer(world, spec, pos, id));
    }
    if crate::cloning::is_cloning_part(proto) {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::cloning::spawn_part(world, proto, pos, id));
    }
    if crate::airlock_control::is_airlock_control(proto) {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::airlock_control::spawn_part(world, spec, pos, id));
    }
    if proto == crate::arcade::ARCADE {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::arcade::spawn_arcade(world, pos, id));
    }
    if proto == crate::microwaves::MICROWAVE {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::microwaves::spawn_microwave(world, spec, pos, id));
    }
    if proto == crate::monkeys::MONKEY {
        let id = world.resource_mut::<NetIds>().alloc();
        let monkey = spawn_bare_mob(world, pos, id);
        world.entity_mut(monkey).remove::<PlayerControlled>().insert((
            crate::monkeys::Monkey,
            Appearance::new(crate::monkeys::MONKEY, ""),
            DisplayName(crate::monkeys::monkey_name(id.0)),
        ));
        return Ok(monkey);
    }
    if crate::flashes::is_flasher(proto) {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::flashes::spawn_flasher(world, spec, pos, id));
    }
    if proto == crate::navbeacons::NAVBEACON {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::navbeacons::spawn_navbeacon(world, spec, pos, id));
    }
    if crate::shields::is_shield_part(proto) {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::shields::spawn_shield_part(world, proto, pos, id));
    }
    if proto == crate::morgue::CREMATORIUM {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::morgue::spawn_crematorium(world, spec, pos, id));
    }
    if proto == crate::morgue::IGNITER {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::morgue::spawn_igniter(world, spec, pos, id));
    }
    if proto == crate::morgue::MORGUE {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::morgue::spawn_morgue(world, pos, id));
    }
    if proto == crate::heaters::SPACE_HEATER {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::heaters::spawn_heater(world, spec, pos, id));
    }
    if proto == crate::conveyors::CONVEYOR {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::conveyors::spawn_conveyor(world, spec, pos, id));
    }
    if proto == crate::conveyors::CONVEYOR_SWITCH {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::conveyors::spawn_switch(world, spec, pos, id));
    }
    if crate::filing::is_cabinet(proto) {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::filing::spawn_cabinet(world, proto, pos, id));
    }
    if proto == crate::bedsheets::LINEN_BIN {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::bedsheets::spawn_linen_bin(world, pos, id));
    }
    if proto == crate::firealarms::FIRE_ALARM {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::firealarms::spawn_fire_alarm(world, spec, pos, id));
    }
    if proto == crate::status_displays::STATUS_DISPLAY {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::status_displays::spawn_display(world, spec, pos, id));
    }
    if proto == crate::cameras::CAMERA {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::cameras::spawn_camera(world, spec, pos, id));
    }
    if proto == crate::cameras::SECURITY_CONSOLE {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::cameras::spawn_console(world, pos, id));
    }
    if let Some(kind) = crate::vending::kind(proto) {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::vending::spawn_vending(world, kind, pos, id));
    }
    if crate::portables::is_portable(proto) {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::portables::spawn_portable(world, proto, pos, id));
    }
    if proto == crate::walllockers::EMERGENCY_LOCKER {
        let id = world.resource_mut::<NetIds>().alloc();
        return crate::walllockers::spawn_locker(world, pos, id);
    }
    if crate::chairs::seat(proto).is_some() {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::chairs::spawn_chair(world, spec, pos, id));
    }
    if proto == crate::supermatter::SUPERMATTER {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::supermatter::spawn_supermatter(world, spec, pos, id));
    }
    if proto == crate::supermatter::LASER {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::supermatter::spawn_laser(world, spec, pos, id));
    }
    if proto == crate::supermatter::LASER_COMPUTER {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::supermatter::spawn_laser_computer(world, spec, pos, id));
    }
    if proto == crate::teg::TEG {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::teg::spawn_teg(world, spec, pos, id));
    }
    if proto == crate::power_monitor::POWER_MONITOR {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::power_monitor::spawn_power_monitor(world, spec, pos, id));
    }
    if proto == crate::radio::INTERCOM {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::radio::spawn_intercom(world, spec, pos, id));
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
    if proto == crate::igniters::IGNITER {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::igniters::spawn_igniter(world, spec, pos, id));
    }
    if matches!(
        proto,
        crate::disposals::DISPOSAL_BIN | crate::disposals::DISPOSAL_PIPE | crate::disposals::DISPOSAL_OUTLET
    ) {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::disposals::spawn_disposal(world, spec, pos, id));
    }
    if crate::multiz::is_multiz(proto) {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::multiz::spawn_multiz(world, spec, pos, id));
    }
    if proto == crate::windows::WINDOW_PANE {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::windows::spawn_window_pane(world, spec, pos, id));
    }
    if proto == crate::blastdoors::BLAST_DOOR {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::blastdoors::spawn_blast_door(world, spec, pos, id));
    }
    if proto == crate::blastdoors::DOOR_BUTTON {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::blastdoors::spawn_door_button(world, spec, pos, id));
    }
    if proto == crate::floodlights::FLOODLIGHT {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::floodlights::spawn_floodlight(world, pos, id));
    }
    if proto == crate::igniters::SPARKER {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::igniters::spawn_sparker(world, spec, pos, id));
    }
    if proto == crate::igniters::IGNITION_SWITCH {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::igniters::spawn_ignition_switch(world, spec, pos, id));
    }
    if proto == crate::windoors::WINDOOR || proto == crate::windoors::BRIG_WINDOOR {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::windoors::spawn_windoor(world, spec, pos, id));
    }
    if proto == "firedoor" {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::firedoors::spawn_firedoor(world, spec, pos, id));
    }
    if crate::lights::is_light_proto(proto) {
        let id = world.resource_mut::<NetIds>().alloc();
        return Ok(crate::lights::spawn_light(world, spec, pos, id));
    }
    if matches!(
        proto,
        crate::extinguishers::CABINET | crate::extinguishers::WATER_TANK
    ) {
        let id = world.resource_mut::<NetIds>().alloc();
        return crate::extinguishers::spawn_fixture(world, spec, pos, id);
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
    if let Some(machine) = crate::machines::machine_bundle(proto, spec.dir, &spec.vars) {
        let id = world.resource_mut::<NetIds>().alloc();
        let appearance = Appearance::new(machine.sprite, machine.state.clone());
        let mut entity = world.spawn((GridPos(pos), id, appearance));
        machine.apply(&mut entity);
        return Ok(entity.id());
    }
    let def = world.resource::<ItemProtos>().0.get(proto).cloned();
    if proto != "airlock" && proto != crate::doors::SHUTTLE_DOOR && def.is_none() {
        return Err(format!("unknown prototype {proto:?} at {pos:?}"));
    }

    let id = world.resource_mut::<NetIds>().alloc();
    // Luna's `door/unpowered`: no power, no panel, no wires (`doors.md`).
    if proto == crate::doors::SHUTTLE_DOOR {
        let airlock = Airlock::default();
        let appearance = Appearance::new(proto, airlock.state.name());
        return Ok(world
            .spawn((GridPos(pos), id, appearance, airlock, Dense, Opaque))
            .id());
    }
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
    let roll = world.resource_mut::<crate::rng::GameRng>().range(0, 3);
    let mut item = world.spawn((GridPos(pos), id, Appearance::new(look, ""), item));
    crate::food::spawn_food(&mut item, proto);
    crate::weapons::fit_weapon(&mut item, proto);
    crate::chemistry::fit(&mut item, proto);
    crate::smoking::spawn_smokable(&mut item, proto, roll);
    crate::paperwork::spawn_paperwork(&mut item, spec);
    crate::bedsheets::spawn_bedsheet(&mut item, spec);
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
    if let Some(ext) = crate::extinguishers::kind(proto) {
        item.insert(ext);
    }
    if let Some(light) = crate::flashlights::kind(proto) {
        item.insert(light);
    }
    if let Some(cell) = crate::cells::kind(proto) {
        item.insert(cell);
    }
    if proto == crate::radio::RADIO || proto == crate::radio::HEADSET {
        item.insert(crate::radio::Radio::handheld());
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

pub(crate) fn spawn_bare_mob(world: &mut World, pos: TilePos, id: NetId) -> Entity {
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
                rs13_world::Facing::of(rs13_world::Dir::South),
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
                crate::food::Nutrition::default(),
                Surroundings::default(),
                Outbox::default(),
                crate::sound::Earshot::default(),
                PlayerControlled,
            ),
            (
                crate::chemistry::Reagents::new(crate::chemistry::BLOODSTREAM),
                (
                    crate::jobs::PendingJobOps::default(),
                    crate::jobs::JobPanelOut::default(),
                    crate::autolathe::PendingLatheOps::default(),
                    crate::autolathe::LathePanelOut::default(),
                    crate::chem_master::PendingMasterOps::default(),
                    crate::chem_master::MasterPanelOut::default(),
                ),
                crate::medbay::PendingSleeperOps::default(),
                (
                    crate::medbay::SleeperPanelOut::default(),
                    crate::medbay::ScanPanelOut::default(),
                ),
                crate::chem_dispenser::PendingChemOps::default(),
                crate::chem_dispenser::ChemPanelOut::default(),
                crate::arcade::PendingArcadeOps::default(),
                crate::arcade::ArcadePanelOut::default(),
                crate::brig::PendingBrigOps::default(),
                crate::brig::BrigPanelOut::default(),
            ),
            // Machine windows and requests.
            (
                crate::equipment::PendingEquipment::default(),
                crate::wires::PendingWires::default(),
                crate::wires::WirePanelOut::default(),
                crate::alarms::PendingAlarmOps::default(),
                crate::apc::PendingApcOps::default(),
                crate::apc::ApcPanelOut::default(),
                crate::power_monitor::PowerMonitorOut::default(),
                crate::supermatter::PendingLaserOps::default(),
                crate::supermatter::LaserPanelOut::default(),
                crate::freezers::PendingFreezerOps::default(),
                crate::freezers::FreezerPanelOut::default(),
                (
                    crate::gas_panel::PendingGasOps::default(),
                    crate::gas_panel::GasPanelOut::default(),
                    crate::vending::PendingVendOps::default(),
                    crate::vending::VendingPanelOut::default(),
                    crate::cameras::PendingCameraOps::default(),
                    crate::cameras::CameraPanelOut::default(),
                    (
                        crate::paperwork::PendingPaperOps::default(),
                        crate::paperwork::PaperOut::default(),
                        crate::paperwork::ClipboardOut::default(),
                        crate::filing::PendingPicks::default(),
                        crate::filing::PickListOut::default(),
                        crate::heaters::PendingHeaterOps::default(),
                        crate::heaters::HeaterPanelOut::default(),
                    ),
                    crate::smes_panel::PendingSmesOps::default(),
                    crate::smes_panel::SmesPanelOut::default(),
                    crate::alarms::AlarmPanelOut::default(),
                    crate::storage::StorageOut::default(),
                ),
            ),
        ))
        .id()
}

/// Cables, SMES units and APCs. An APC powers the area its tile is in.
fn spawn_power_object(world: &mut World, spec: &ObjectSpec, pos: TilePos) -> Result<Entity, String> {
    use crate::power::{Apc, Cable, PowerNode, SMES_OUTPUT, SMES_UNIT, Smes, Terminal};
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
            // Luna's settings from the map; charge and capacity are in its units.
            let number = |name: &str, default: f64| {
                spec.vars
                    .get(name)
                    .and_then(|v| v.trim().parse::<f64>().ok())
                    .unwrap_or(default)
            };
            let capacity = number("capacity", 5.0e6) * SMES_UNIT;
            let smes = Smes {
                charge: (number("charge", 5.0e6) * SMES_UNIT).min(capacity),
                capacity,
                output: number("output", SMES_OUTPUT),
                online: number("online", 1.0) != 0.0,
                input: number("chargelevel", 0.0),
                charge_auto: number("chargemode", 1.0) != 0.0,
                charging: number("charging", 0.0) != 0.0,
                last_output: 0.0,
            };
            entity.insert((Appearance::new("smes", ""), smes, Dense));
        }
        "terminal" => {
            let terminal = Terminal {
                dir: spec.dir.unwrap_or(rs13_world::Dir::South),
            };
            entity.insert((Appearance::new("terminal", ""), terminal));
        }
        _ => {
            entity.insert((Appearance::new("apc", "on"), Apc::new(area)));
        }
    }
    Ok(entity.id())
}
