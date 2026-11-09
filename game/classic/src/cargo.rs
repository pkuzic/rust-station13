//! Cargo: ordering supplies from Centcom, the supply shuttle that brings them and the points
//! they cost (Luna `supplyshuttle.dm`, `supplypacks.dm`). Spec: docs/reference/cargo.md.
//!
//! - **Points** start at 50 and grow by one every 30 s; every crate sent back on the
//!   shuttle earns 4, and every plasma sheet in one 1 more.
//! - **The ordering console** (anyone) requests packs; **the supply shuttle console** (cargo
//!   access) orders them outright or approves requests, paying in points, and sends the
//!   shuttle. Orders can only be placed while the shuttle waits at Centcom's dock.
//! - **The shuttle** takes 2 minutes to come with the crates; it is laid into the supply
//!   hangar's empty berth on arrival (the berth is open space until then), and sent back it
//!   goes at once, selling the crates aboard. It never carries anyone alive or dead.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_simulation::atmos::gas::TILE_VOLUME;
use rs13_simulation::atmos::{Cell, GasMix};
use rs13_world::{
    CARGO_CATEGORIES, CargoOp, CargoPage, CargoPanelView, Dense, DisplayName, GridPos, MapLayer, MobStat,
    NetId, ObjectSpec, ShuttleAt, TileDef, TileMap, TilePos,
};
use serde::Deserialize;

use crate::actions::{Outbox, in_reach};
use crate::atmos::StationAtmos;
use crate::closets::Closet;
use crate::filing::PendingPicks;
use crate::hands::Stack;
use crate::health::Vitals;
use crate::movement::Mob;
use crate::power::{Channel, PowerUser};
use crate::rng::GameRng;
use crate::time::WorldTime;
use crate::wires::PanelViewer;

pub const SUPPLY_CONSOLE: &str = "supply-console";
pub const ORDER_CONSOLE: &str = "order-console";
/// Where crates are put down on the shuttle (Luna `supplymarker`): only in the shuttle map.
pub const MARKER: &str = "supply-marker";
/// The supply hangar's berth the shuttle docks into (Luna `/area/supply/station`).
pub const BERTH_AREA: &str = "supply-station";
/// The hangar's blast doors, shut whenever the shuttle comes or goes (Luna `qmshuttle`).
pub const HANGAR_DOORS: &str = "qmshuttle";
/// What a supply shuttle console asks for (Luna `access_cargo`).
pub const ACCESS: &str = "cargo";

/// Points at round start, one more every `POINT_EVERY_DS`, and what crates and plasma sheets
/// sent back earn (Luna `SUPPLY_*`).
pub const START_POINTS: u32 = 50;
pub const POINT_EVERY_DS: f32 = 300.0;
pub const PER_CRATE: u32 = 4;
pub const PER_PLASMA_SHEET: u32 = 1;
/// How long the shuttle takes to come from the dock, ds (Luna `SUPPLY_MOVETIME`).
pub const MOVE_DS: f32 = 1200.0;

const PLASMA_SHEET: &str = "plasma-sheet";
const PAPER: &str = "paper";

/// The supply shuttle as it waits at Centcom's dock, laid into the berth when it arrives.
const SHUTTLE_MAP: &str = include_str!("../maps/luna_supply_shuttle.ron");

/// A pack Centcom sells, from `content/supply_packs.ron`.
#[derive(Debug, Clone, Deserialize)]
pub struct PackDef {
    pub id: String,
    pub categories: Vec<String>,
    pub cost: u32,
    pub contains: Vec<String>,
    #[serde(default)]
    pub amount: Option<u32>,
    #[serde(default)]
    pub container: Option<String>,
    #[serde(default)]
    pub access: Option<String>,
}

/// Every pack, in Luna's order.
#[derive(Resource, Debug)]
pub struct SupplyPacks(pub Vec<PackDef>);

impl Default for SupplyPacks {
    fn default() -> Self {
        Self(
            rs13_content::parse_ron(
                "content/supply_packs.ron",
                include_str!("../content/supply_packs.ron"),
            )
            .expect("built-in supply packs are valid"),
        )
    }
}

impl SupplyPacks {
    pub fn get(&self, id: &str) -> Option<&PackDef> {
        self.0.iter().find(|p| p.id == id)
    }

    /// The packs listed under a category, by its index in [`CARGO_CATEGORIES`].
    pub fn in_category(&self, category: u8) -> Vec<&PackDef> {
        let Some(name) = CARGO_CATEGORIES.get(usize::from(category)) else {
            return Vec::new();
        };
        self.0
            .iter()
            .filter(|p| p.categories.iter().any(|c| c == name))
            .collect()
    }
}

/// A pack asked for: by whom (a requester, or the one who approved or ordered it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order {
    pub pack: String,
    pub by: String,
}

/// Where cargo stands this round.
#[derive(Resource, Debug)]
pub struct Cargo {
    pub points: u32,
    /// Docked at the station.
    pub at_station: bool,
    /// On its way to the station: when it arrives, ds.
    pub arrives_at: Option<f32>,
    pub next_point_at: f32,
    pub requests: Vec<Order>,
    /// Paid for: delivered when the shuttle next comes, cleared when it is sent back.
    pub orders: Vec<Order>,
    /// What the shuttle brought with it (its doors, belts and panes), gone when it leaves.
    pub parts: Vec<Entity>,
    /// Things left aboard when it left, with where they were: back when it returns.
    pub away: Vec<(Entity, TilePos)>,
    /// Its floor needs air, next tick (the air's zones are rebuilt in between).
    pub air_due: Vec<TilePos>,
}

impl Default for Cargo {
    fn default() -> Self {
        Self {
            points: START_POINTS,
            at_station: false,
            arrives_at: None,
            next_point_at: POINT_EVERY_DS,
            requests: Vec::new(),
            orders: Vec::new(),
            parts: Vec::new(),
            away: Vec::new(),
            air_due: Vec::new(),
        }
    }
}

impl Cargo {
    pub fn shuttle_at(&self, now: f32) -> ShuttleAt {
        match (self.at_station, self.arrives_at) {
            (_, Some(at)) => ShuttleAt::Moving(((at - now).max(0.0) / 10.0).ceil() as u32),
            (true, None) => ShuttleAt::Station,
            (false, None) => ShuttleAt::Dock,
        }
    }

    fn at_dock(&self) -> bool {
        !self.at_station && self.arrives_at.is_none()
    }
}

/// What a cargo console shows besides the shuttle and points (Luna's `temp`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum Page {
    #[default]
    Main,
    Categories,
    Category(u8),
    Requests,
    Orders,
    Note(&'static str),
}

/// A cargo console: the supply shuttle console (`supply`) or an ordering console.
#[derive(Component, Debug, Clone, Default)]
pub struct CargoConsole {
    pub supply: bool,
    page: Page,
}

/// What a mob's open cargo console window shows.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct CargoPanelOut(pub Option<CargoPanelView>);

pub fn is_cargo_console(proto: &str) -> bool {
    matches!(proto, SUPPLY_CONSOLE | ORDER_CONSOLE)
}

pub fn spawn_console(world: &mut World, proto: &str, pos: TilePos, id: NetId) -> Entity {
    world
        .spawn((
            GridPos(pos),
            id,
            Dense,
            rs13_world::Appearance::new(proto, ""),
            CargoConsole {
                supply: proto == SUPPLY_CONSOLE,
                page: Page::Main,
            },
            PowerUser::new(Channel::Equipment, 100.0),
        ))
        .id()
}

fn tell(world: &mut World, mob: Entity, msg: LocMsg) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(msg);
    }
}

fn powered(world: &World, console: Entity) -> bool {
    world.get::<PowerUser>(console).is_some_and(|p| p.powered)
}

fn awake(world: &World, mob: Entity) -> bool {
    world
        .get::<Vitals>(mob)
        .is_none_or(|v| v.stat == MobStat::Conscious)
}

fn now(world: &World) -> f32 {
    world.get_resource::<WorldTime>().map_or(0.0, |t| t.deciseconds())
}

/// A click on a cargo console opens its window, at its main page. The supply shuttle console
/// wants cargo access (Luna "Access Denied.").
pub fn click(world: &mut World, mob: Entity, console: Entity) {
    let Some(supply) = world.get::<CargoConsole>(console).map(|c| c.supply) else {
        return;
    };
    if !powered(world, console) {
        return;
    }
    if supply && !crate::access::Shown::of_mob(world, mob).opens(&[ACCESS.to_owned()]) {
        tell(world, mob, LocMsg::new("cargo-access-denied"));
        return;
    }
    if let Some(mut c) = world.get_mut::<CargoConsole>(console) {
        c.page = Page::Main;
    }
    world.entity_mut(mob).insert(PanelViewer(console));
}

/// Buttons pressed on cargo consoles (sent as picks); others are left for the next module.
pub fn run_picks(world: &mut World) {
    let mut q = world.query::<(Entity, &mut PendingPicks)>();
    let picks: Vec<(Entity, Vec<(Entity, usize)>)> = q
        .iter_mut(world)
        .filter(|(_, p)| !p.0.is_empty())
        .map(|(mob, mut p)| (mob, std::mem::take(&mut p.0)))
        .collect();
    for (mob, list) in picks {
        let mut others = Vec::new();
        for (target, line) in list {
            if world.get::<CargoConsole>(target).is_some() {
                press(world, mob, target, line);
            } else {
                others.push((target, line));
            }
        }
        if !others.is_empty()
            && let Some(mut pending) = world.get_mut::<PendingPicks>(mob)
        {
            pending.0.extend(others);
        }
    }
}

fn set_page(world: &mut World, console: Entity, page: Page) {
    if let Some(mut c) = world.get_mut::<CargoConsole>(console) {
        c.page = page;
    }
}

fn name_of(world: &World, mob: Entity) -> String {
    world
        .get::<DisplayName>(mob)
        .map_or_else(|| "Unknown".to_owned(), |n| n.0.clone())
}

/// One button of a cargo console (Luna `ordercomp/Topic`, `supplycomp/Topic`).
fn press(world: &mut World, mob: Entity, console: Entity, line: usize) {
    let (Some(here), Some(there)) = (
        world.get::<GridPos>(mob).map(|p| p.0),
        world.get::<GridPos>(console).map(|p| p.0),
    ) else {
        return;
    };
    if !in_reach(here, there) || !awake(world, mob) || !powered(world, console) {
        return;
    }
    let Some(op) = u16::try_from(line).ok().and_then(CargoOp::from_line) else {
        return;
    };
    let Some(c) = world.get::<CargoConsole>(console).cloned() else {
        return;
    };
    world.init_resource::<Cargo>();
    world.init_resource::<SupplyPacks>();
    let t = now(world);
    match op {
        CargoOp::Main => set_page(world, console, Page::Main),
        CargoOp::Order => set_page(world, console, Page::Categories),
        CargoOp::Category(cat) if usize::from(cat) < CARGO_CATEGORIES.len() => {
            set_page(world, console, Page::Category(cat));
        }
        CargoOp::Pack(i) => {
            let Page::Category(cat) = c.page else { return };
            let Some(pack) = world
                .resource::<SupplyPacks>()
                .in_category(cat)
                .get(usize::from(i))
                .map(|p| (p.id.clone(), p.cost))
            else {
                return;
            };
            let by = name_of(world, mob);
            let note = if c.supply {
                order(world, pack, by)
            } else {
                world
                    .resource_mut::<Cargo>()
                    .requests
                    .push(Order { pack: pack.0, by });
                "cargo-requested"
            };
            set_page(world, console, Page::Note(note));
        }
        CargoOp::Requests => set_page(world, console, Page::Requests),
        CargoOp::Orders => set_page(world, console, Page::Orders),
        CargoOp::Approve(i) if c.supply && world.resource::<Cargo>().at_dock() => {
            let mut cargo = world.resource_mut::<Cargo>();
            if usize::from(i) >= cargo.requests.len() {
                return;
            }
            let request = cargo.requests.remove(usize::from(i));
            let cost = world
                .resource::<SupplyPacks>()
                .get(&request.pack)
                .map_or(0, |p| p.cost);
            let by = name_of(world, mob);
            let note = order(world, (request.pack, cost), by);
            set_page(world, console, Page::Note(note));
        }
        CargoOp::Remove(i) if c.supply => {
            let mut cargo = world.resource_mut::<Cargo>();
            if usize::from(i) < cargo.requests.len() {
                cargo.requests.remove(usize::from(i));
            }
            set_page(world, console, Page::Requests);
        }
        CargoOp::ClearRequests if c.supply => {
            world.resource_mut::<Cargo>().requests.clear();
            set_page(world, console, Page::Note("cargo-requests-cleared"));
        }
        CargoOp::Send if c.supply => {
            let cargo = world.resource::<Cargo>();
            if cargo.arrives_at.is_some() {
                return;
            }
            if cargo.at_station {
                if anyone_aboard(world) {
                    tell(world, mob, LocMsg::new("cargo-no-people"));
                    return;
                }
                send_to_dock(world);
            } else {
                world.resource_mut::<Cargo>().arrives_at = Some(t + MOVE_DS);
                shut_hangar(world);
                tell(
                    world,
                    mob,
                    LocMsg::new("cargo-called").num("value", (MOVE_DS / 600.0).round() as i64),
                );
            }
            set_page(world, console, Page::Note("cargo-shuttle-sent"));
        }
        _ => {}
    }
}

/// Pay for a pack and add it to the orders, if the shuttle is at the dock and there are the
/// points. The note to show.
fn order(world: &mut World, (pack, cost): (String, u32), by: String) -> &'static str {
    let mut cargo = world.resource_mut::<Cargo>();
    if !cargo.at_dock() {
        return "cargo-must-be-docked";
    }
    if cargo.points < cost {
        return "cargo-not-enough";
    }
    cargo.points -= cost;
    cargo.orders.push(Order { pack, by });
    "cargo-ordered"
}

/// The tiles of the berth on the station, and its lowest corner.
fn berth(world: &mut World) -> Option<(Vec<TilePos>, TilePos)> {
    let mut maps = world.query::<&MapLayer>();
    for map in maps.iter(world) {
        let map = &map.0;
        let tiles: Vec<TilePos> = (0..map.height as i32)
            .flat_map(|y| (0..map.width as i32).map(move |x| TilePos::new(x, y, map.z)))
            .filter(|p| map.area(*p) == Some(BERTH_AREA))
            .collect();
        if let Some(corner) = tiles
            .iter()
            .copied()
            .reduce(|a, b| TilePos::new(a.x.min(b.x), a.y.min(b.y), a.z))
        {
            return Some((tiles, corner));
        }
    }
    None
}

/// Anyone, alive or dead, on the berth while the shuttle is in it (Luna `supply_can_move`).
fn anyone_aboard(world: &mut World) -> bool {
    let Some((tiles, _)) = berth(world) else {
        return false;
    };
    let mut mobs = world.query_filtered::<&GridPos, With<Mob>>();
    mobs.iter(world).any(|p| tiles.contains(&p.0))
}

fn shut_hangar(world: &mut World) {
    let t = now(world);
    let mut doors = world.query::<&mut crate::blastdoors::BlastDoor>();
    for mut door in doors.iter_mut(world) {
        if door.id == HANGAR_DOORS && !door.blocks() {
            door.shutter = crate::blastdoors::Shutter::Closing(t + crate::blastdoors::CLOSING_DS);
        }
    }
}

fn set_tile(world: &mut World, at: TilePos, tile: TileDef, cell: Cell) {
    let mut maps = world.query::<&mut MapLayer>();
    if let Some(mut map) = maps.iter_mut(world).find(|m| m.0.z == at.z) {
        map.0.set_tile(at, tile);
    }
    if let Some(mut atmos) = world.get_resource_mut::<StationAtmos>() {
        atmos.0.set_cell(at, cell);
    }
}

/// The shuttle arrives: its floor, walls and fittings are laid into the berth, what was left
/// aboard comes back, and the orders are put down on its markers (Luna `send_supply_shuttle`
/// and `process_supply_order`).
fn arrive(world: &mut World) {
    let Some((tiles, corner)) = berth(world) else {
        return;
    };
    let Ok(shuttle) = TileMap::from_ron(SHUTTLE_MAP, corner.z) else {
        return;
    };
    let place = |x: i32, y: i32| TilePos::new(corner.x + x, corner.y + y, corner.z);
    let mut floor = Vec::new();
    let doors: Vec<(i32, i32)> = shuttle
        .objects
        .iter()
        .filter(|(_, s)| s.id == crate::doors::SHUTTLE_DOOR)
        .map(|(p, _)| (p.x, p.y))
        .collect();
    for y in 0..shuttle.height as i32 {
        for x in 0..shuttle.width as i32 {
            let at = place(x, y);
            let Some(tile) = shuttle.tile(TilePos::new(x, y, corner.z)).cloned() else {
                continue;
            };
            if !tiles.contains(&at) {
                continue;
            }
            let cell = if doors.contains(&(x, y)) {
                Cell::Portal
            } else if tile.solid {
                Cell::Solid
            } else {
                floor.push(at);
                Cell::Open
            };
            set_tile(world, at, tile, cell);
        }
    }
    let mut parts = Vec::new();
    let mut markers = Vec::new();
    for (pos, spec) in &shuttle.objects {
        let at = place(pos.x, pos.y);
        if spec.id == MARKER {
            markers.push(at);
            continue;
        }
        let Ok(part) = crate::objects::spawn_object(world, spec, at) else {
            continue;
        };
        // A pane seals its edge for good, as those the map starts with do.
        if let Some(edge) = world.get::<crate::windows::WindowPane>(part).map(|w| w.edge(at))
            && let Some(mut atmos) = world.get_resource_mut::<StationAtmos>()
        {
            atmos.0.add_edge(edge.0, edge.1);
            atmos.0.set_edge_open(edge.0, edge.1, false);
        }
        parts.push(part);
    }
    let away = std::mem::take(&mut world.resource_mut::<Cargo>().away);
    for (thing, at) in away {
        if world.get_entity(thing).is_ok() {
            world.entity_mut(thing).insert(GridPos(at));
        }
    }
    let orders = world.resource::<Cargo>().orders.clone();
    for order in &orders {
        deliver(world, order, &markers);
    }
    let mut cargo = world.resource_mut::<Cargo>();
    cargo.parts = parts;
    cargo.at_station = true;
    cargo.arrives_at = None;
    cargo.air_due = floor;
}

/// Put a pack down on a free marker (else any), in its crate if it comes in one, with its
/// manifest (Luna `process_supply_order`).
fn deliver(world: &mut World, order: &Order, markers: &[TilePos]) {
    world.init_resource::<SupplyPacks>();
    let Some(pack) = world.resource::<SupplyPacks>().get(&order.pack).cloned() else {
        return;
    };
    let mut taken = world.query::<&GridPos>();
    let used: Vec<TilePos> = taken.iter(world).map(|p| p.0).collect();
    // Luna takes the last free marker it finds.
    let at = match markers.iter().rev().find(|m| !used.contains(m)) {
        Some(at) => *at,
        None => {
            if markers.is_empty() {
                return;
            }
            let i = world.resource_mut::<GameRng>().range(0, markers.len() as i32 - 1) as usize;
            markers[i]
        }
    };
    let crate_entity = pack
        .container
        .as_deref()
        .and_then(|id| crate::objects::spawn_object(world, &ObjectSpec::new(id), at).ok());
    if let (Some(c), Some(access)) = (crate_entity, pack.access.as_ref())
        && let Some(mut lock) = world.get_mut::<crate::closets::ClosetLock>(c)
    {
        lock.access = vec![access.clone()];
    }
    let mut items = Vec::new();
    for id in &pack.contains {
        let made = match pack.amount {
            Some(n) => crate::objects::spawn_stack(world, id, at, n),
            None => crate::objects::spawn_object(world, &ObjectSpec::new(id.as_str()), at),
        };
        if let Ok(item) = made {
            items.push(item);
        }
    }
    let mut sheet = ObjectSpec::new(PAPER);
    sheet
        .vars
        .insert("name".to_owned(), "[msg:cargo-manifest-paper]".to_owned());
    sheet.vars.insert("info".to_owned(), manifest(&pack));
    if let Ok(paper) = crate::objects::spawn_object(world, &sheet, at) {
        items.push(paper);
    }
    if let Some(c) = crate_entity {
        for item in items {
            crate::closets::put_inside(world, c, item);
        }
    }
}

/// A pack's manifest: what's in it, in the reader's language (Luna's "Centcomm Shipping
/// Manifest").
pub fn manifest(pack: &PackDef) -> String {
    let mut text = String::from("[b][msg:cargo-manifest-title][/b]\n[msg:cargo-manifest-destination]\n");
    text.push_str("[msg:cargo-manifest-contents]\n");
    for id in &pack.contains {
        text.push_str(&format!("- [noun:{id}]\n"));
    }
    text.push_str("[msg:cargo-manifest-check]\n");
    text
}

/// Take something out of the world, with whatever is inside it.
fn destroy(world: &mut World, thing: Entity) {
    let inside: Vec<Entity> = world
        .get::<Closet>(thing)
        .map(|c| c.contents.clone())
        .into_iter()
        .flatten()
        .chain(
            world
                .get::<crate::storage::Storage>(thing)
                .map(|s| s.items.clone())
                .into_iter()
                .flatten(),
        )
        .collect();
    for item in inside {
        destroy(world, item);
    }
    if world.get_entity(thing).is_ok() {
        world.entity_mut(thing).despawn();
    }
}

/// What a crate earns sent back: 4, and 1 more for every plasma sheet in it (Luna
/// `sell_crates`).
fn worth(world: &World, closet: Entity) -> u32 {
    let sheets: u32 = world
        .get::<Closet>(closet)
        .map(|c| c.contents.clone())
        .unwrap_or_default()
        .into_iter()
        .filter(|item| {
            world
                .get::<rs13_world::Appearance>(*item)
                .is_some_and(|a| a.sprite == PLASMA_SHEET)
        })
        .map(|item| world.get::<Stack>(item).map_or(1, |s| s.amount))
        .sum();
    PER_CRATE + sheets * PER_PLASMA_SHEET
}

/// Send the shuttle back to the dock: the crates aboard are sold, its fittings go with it, the
/// rest left aboard waits at the dock, and the berth is open space again.
fn send_to_dock(world: &mut World) {
    shut_hangar(world);
    let Some((tiles, _)) = berth(world) else {
        return;
    };
    let parts = std::mem::take(&mut world.resource_mut::<Cargo>().parts);
    for part in parts {
        if world.get_entity(part).is_ok() {
            world.entity_mut(part).despawn();
        }
    }
    let mut things = world.query_filtered::<(Entity, &GridPos), Without<Mob>>();
    let aboard: Vec<(Entity, TilePos)> = things
        .iter(world)
        .filter(|(_, p)| tiles.contains(&p.0))
        .map(|(e, p)| (e, p.0))
        .collect();
    let mut earned = 0;
    let mut away = Vec::new();
    for (thing, at) in aboard {
        if world.get::<Closet>(thing).is_some_and(|c| c.is_crate) {
            earned += worth(world, thing);
            destroy(world, thing);
        } else {
            world.entity_mut(thing).remove::<GridPos>();
            away.push((thing, at));
        }
    }
    for at in &tiles {
        set_tile(
            world,
            *at,
            TileDef {
                id: rs13_simulation::lighting::SPACE.to_owned(),
                solid: false,
                opaque: None,
                airtight: None,
            },
            Cell::Space,
        );
    }
    let mut cargo = world.resource_mut::<Cargo>();
    cargo.points += earned;
    cargo.away = away;
    cargo.orders.clear();
    cargo.at_station = false;
    cargo.air_due.clear();
}

/// Points come in, the shuttle arrives, and its floor gets air.
pub fn run_cargo(world: &mut World) {
    world.init_resource::<Cargo>();
    let t = now(world);
    let air_due = std::mem::take(&mut world.resource_mut::<Cargo>().air_due);
    if !air_due.is_empty()
        && let Some(mut atmos) = world.get_resource_mut::<StationAtmos>()
    {
        for at in air_due {
            atmos.0.add_at(at, &GasMix::standard_air(TILE_VOLUME));
        }
    }
    let mut cargo = world.resource_mut::<Cargo>();
    while t >= cargo.next_point_at {
        cargo.points += 1;
        cargo.next_point_at += POINT_EVERY_DS;
    }
    if cargo.arrives_at.is_some_and(|at| t >= at) {
        arrive(world);
    }
}

/// Show each player the cargo console they have open; close it when out of reach.
pub fn update_views(world: &mut World) {
    world.init_resource::<Cargo>();
    world.init_resource::<SupplyPacks>();
    let t = now(world);
    let mut q = world.query::<(Entity, &GridPos, &PanelViewer)>();
    let viewers: Vec<(Entity, TilePos, Entity)> = q.iter(world).map(|(m, p, v)| (m, p.0, v.0)).collect();
    for (mob, at, console) in viewers {
        let Some(c) = world.get::<CargoConsole>(console).cloned() else {
            continue;
        };
        let (Some(place), Some(id)) = (
            world.get::<GridPos>(console).map(|p| p.0),
            world.get::<NetId>(console).copied(),
        ) else {
            continue;
        };
        if !in_reach(at, place) || !awake(world, mob) || !powered(world, console) {
            world.entity_mut(mob).remove::<PanelViewer>();
            if let Some(mut out) = world.get_mut::<CargoPanelOut>(mob) {
                out.0 = None;
            }
            continue;
        }
        let cargo = world.resource::<Cargo>();
        let packs = world.resource::<SupplyPacks>();
        let page = match &c.page {
            Page::Main => CargoPage::Main,
            Page::Categories => CargoPage::Categories,
            Page::Category(cat) => CargoPage::Packs(
                *cat,
                packs
                    .in_category(*cat)
                    .iter()
                    .map(|p| (p.id.clone(), p.cost))
                    .collect(),
            ),
            Page::Requests => CargoPage::Requests(
                cargo
                    .requests
                    .iter()
                    .map(|o| (o.pack.clone(), o.by.clone()))
                    .collect(),
            ),
            Page::Orders => CargoPage::Orders(
                cargo
                    .orders
                    .iter()
                    .map(|o| (o.pack.clone(), o.by.clone()))
                    .collect(),
            ),
            Page::Note(id) => CargoPage::Note((*id).to_owned()),
        };
        let view = CargoPanelView {
            target: id,
            supply: c.supply,
            points: cargo.points,
            shuttle: cargo.shuttle_at(t),
            page,
        };
        match world.get_mut::<CargoPanelOut>(mob) {
            Some(mut out) => {
                if out.0.as_ref() != Some(&view) {
                    out.0 = Some(view);
                }
            }
            None => {
                world.entity_mut(mob).insert(CargoPanelOut(Some(view)));
            }
        }
    }
}

/// Close the window of anyone no longer looking at a cargo console.
pub fn clear_closed_views(
    mut out: Query<(&mut CargoPanelOut, Option<&PanelViewer>)>,
    consoles: Query<(), With<CargoConsole>>,
) {
    for (mut out, viewer) in &mut out {
        if out.0.is_some() && !viewer.is_some_and(|v| consoles.contains(v.0)) {
            out.0 = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pack_is_listed_somewhere_and_costs_something() {
        let packs = SupplyPacks::default();
        assert!(packs.0.len() >= 25, "{}", packs.0.len());
        for p in &packs.0 {
            assert!(p.cost > 0, "{}", p.id);
            assert!(!p.contains.is_empty(), "{}", p.id);
            assert!(
                p.categories
                    .iter()
                    .all(|c| CARGO_CATEGORIES.contains(&c.as_str())),
                "{}: {:?}",
                p.id,
                p.categories
            );
        }
    }

    #[test]
    fn the_shuttle_fits_the_berth() {
        let shuttle = TileMap::from_ron(SHUTTLE_MAP, 0).unwrap();
        assert_eq!((shuttle.width, shuttle.height), (13, 7));
        assert!(shuttle.objects.iter().filter(|(_, s)| s.id == MARKER).count() >= 30);
    }
}
