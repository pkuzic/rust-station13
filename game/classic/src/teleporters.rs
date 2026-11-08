//! Teleporters (Luna's `machinery/teleporter.dm`, `hand_tele`, `portals.dm`, `radio/beacon`): a
//! console locks onto a tracking beacon, a station two tiles along turns the hub between them on,
//! and walking into the hub throws you to the beacon. A hand teleporter opens a portal to a locked
//! console's beacon, or somewhere near at random. Spec: `docs/reference/teleporters.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, Dense, GridPos, MapLayer, MobStat, NetId, PickListView, TilePos};

use crate::actions::{Outbox, in_reach};
use crate::combat::VisibleEvents;
use crate::filing::{PendingPicks, PickListOut};
use crate::hands::{Hands, HeldBy};
use crate::health::Vitals;
use crate::movement::Bumps;
use crate::objects::NetIds;
use crate::power::{Channel, PowerUser};
use crate::rng::GameRng;
use crate::sparks::SparkShowers;
use crate::time::WorldTime;
use crate::wires::PanelViewer;

pub const CONSOLE: &str = "teleporter-console";
pub const HUB: &str = "teleporter-hub";
pub const STATION: &str = "teleporter-station";
pub const BEACON: &str = "tracking-beacon";
pub const HAND_TELE: &str = "hand-tele";
pub const PORTAL: &str = "portal";

/// A portal lasts 30 s (Luna's `spawn(300)`); a hand teleporter keeps at most four open.
pub const PORTAL_DS: f32 = 300.0;
pub const MAX_PORTALS: usize = 4;
/// A portal goes wrong 4% of the time, the hub 5%: out into space.
pub const PORTAL_FAIL: f32 = 4.0;
pub const HUB_FAIL: f32 = 5.0;
/// How far the hand teleporter's "None (Dangerous)" can throw you, tiles (Luna `orange(10)`).
pub const DANGER_RANGE: i32 = 10;

pub fn is_teleporter_part(proto: &str) -> bool {
    matches!(proto, CONSOLE | HUB | STATION)
}

/// The teleporter computer: its id, and the beacon it's locked onto.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct TeleConsole {
    pub id: String,
    pub locked: Option<Entity>,
}

/// The hub: on while the station beside it is engaged and powered.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TeleHub {
    pub on: bool,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TeleStation {
    pub engaged: bool,
}

/// A tracking beacon (Luna `radio/beacon`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Beacon;

/// Where a portal leads: a beacon wherever it's taken, or a place.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Destination {
    Beacon(Entity),
    Tile(TilePos),
}

/// A portal from a hand teleporter.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Portal {
    pub to: Destination,
    pub until: f32,
    pub creator: Entity,
    /// Went wrong once: it throws no one any more (Luna's `portal1`).
    pub broken: bool,
}

pub fn spawn_part(world: &mut World, proto: &str, pos: TilePos, id: NetId) -> Entity {
    let code = world.resource_mut::<GameRng>().range(1000, 9999);
    let mut e = world.spawn((GridPos(pos), id, Dense));
    match proto {
        CONSOLE => {
            e.insert((
                Appearance::new(CONSOLE, "teleport"),
                TeleConsole {
                    id: code.to_string(),
                    locked: None,
                },
                PowerUser::new(Channel::Equipment, 100.0),
            ));
        }
        HUB => {
            e.insert((Appearance::new(HUB, "tele0"), TeleHub::default()));
        }
        _ => {
            e.insert((
                Appearance::new(STATION, "controller"),
                TeleStation::default(),
                PowerUser::new(Channel::Equipment, 100.0),
            ));
        }
    }
    e.id()
}

fn at(world: &World, e: Entity) -> Option<TilePos> {
    world.get::<GridPos>(e).map(|p| p.0)
}

fn tell(world: &mut World, mob: Entity, msg: LocMsg) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(msg);
    }
}

fn around(world: &mut World, where_: TilePos, id: &str) {
    world
        .resource_mut::<VisibleEvents>()
        .0
        .push((where_, LocMsg::new(id), None));
}

fn powered(world: &World, e: Entity) -> bool {
    world.get::<PowerUser>(e).is_none_or(|p| p.powered)
}

/// The first of `T` on a tile.
fn on_tile<T: Component>(world: &mut World, tile: TilePos) -> Option<Entity> {
    let mut q = world.query_filtered::<(Entity, &GridPos), With<T>>();
    q.iter(world).find(|(_, p)| p.0 == tile).map(|(e, _)| e)
}

fn offset(tile: TilePos, dx: i32) -> TilePos {
    TilePos::new(tile.x + dx, tile.y, tile.z)
}

/// Where a thing is: its tile, or that of whoever holds it (Luna `find_loc`).
fn where_is(world: &World, thing: Entity) -> Option<TilePos> {
    at(world, thing).or_else(|| world.get::<HeldBy>(thing).and_then(|h| at(world, h.0)))
}

fn area_of(world: &mut World, tile: TilePos) -> String {
    let mut layers = world.query::<&MapLayer>();
    layers
        .iter(world)
        .find(|l| l.0.z == tile.z)
        .and_then(|l| l.0.area(tile).map(str::to_owned))
        .unwrap_or_default()
}

/// Every beacon somewhere on the station, named after its area (the second of a name gets
/// " (2)", and so on), in a steady order.
fn beacons(world: &mut World) -> Vec<(String, Entity)> {
    let mut q = world.query_filtered::<Entity, With<Beacon>>();
    let mut all: Vec<Entity> = q.iter(world).collect();
    all.sort();
    let mut out: Vec<(String, Entity)> = Vec::new();
    for beacon in all {
        let Some(tile) = where_is(world, beacon) else {
            continue;
        };
        let name = area_of(world, tile);
        let seen = out
            .iter()
            .filter(|(n, _)| *n == name || n.starts_with(&format!("{name} (")))
            .count();
        let shown = if seen == 0 {
            name
        } else {
            format!("{name} ({})", seen + 1)
        };
        out.push((shown, beacon));
    }
    out
}

/// Each hub with a console two tiles before it locked onto a beacon: its console's id, whether
/// it's on, and the beacon.
fn hubs(world: &mut World) -> Vec<(String, bool, Entity)> {
    let mut q = world.query::<(Entity, &GridPos, &TeleHub)>();
    let mut all: Vec<(Entity, TilePos, bool)> = q.iter(world).map(|(e, p, h)| (e, p.0, h.on)).collect();
    all.sort_by_key(|(e, ..)| *e);
    let mut out = Vec::new();
    for (_, tile, on) in all {
        let Some(console) = on_tile::<TeleConsole>(world, offset(tile, -2)) else {
            continue;
        };
        if let Some(c) = world.get::<TeleConsole>(console)
            && let Some(beacon) = c.locked
        {
            out.push((c.id.clone(), on, beacon));
        }
    }
    out
}

/// A click on the console, the station or a portal, in reach.
pub fn click(world: &mut World, mob: Entity, target: Entity) {
    let (Some(here), Some(there)) = (at(world, mob), at(world, target)) else {
        return;
    };
    if !in_reach(here, there) {
        return;
    }
    if world.get::<TeleConsole>(target).is_some() {
        // Luna's `attack_hand`, from any click: the list of places to lock onto.
        if powered(world, target) {
            world
                .entity_mut(mob)
                .insert((PanelViewer(target), PickListOut::default()));
        }
    } else if world.get::<TeleStation>(target).is_some() {
        if !powered(world, target) {
            return;
        }
        let engaged = {
            let Some(mut s) = world.get_mut::<TeleStation>(target) else {
                return;
            };
            s.engaged = !s.engaged;
            s.engaged
        };
        if on_tile::<TeleHub>(world, offset(there, 1)).is_some() {
            around(
                world,
                there,
                if engaged {
                    "tele-engaged"
                } else {
                    "tele-disengaged"
                },
            );
        }
    } else if world.get::<Portal>(target).is_some() {
        through_portal(world, target, mob);
    }
}

/// The hand teleporter used in the hand: its list.
pub fn use_hand_tele(world: &mut World, mob: Entity, tele: Entity) {
    world
        .entity_mut(mob)
        .insert((PanelViewer(tele), PickListOut::default()));
}

/// Whether the filing cabinets should leave this viewer's list alone.
pub fn keeps_own_list(sprite: &str) -> bool {
    matches!(sprite, CONSOLE | HAND_TELE)
}

fn sprite(world: &World, e: Entity) -> Option<String> {
    world.get::<Appearance>(e).map(|a| a.sprite.clone())
}

fn holding(world: &World, mob: Entity, item: Entity) -> bool {
    world.get::<Hands>(mob).and_then(Hands::active_item) == Some(item)
}

fn awake(world: &World, mob: Entity) -> bool {
    world
        .get::<Vitals>(mob)
        .is_none_or(|v| v.stat == MobStat::Conscious)
}

/// Picks in the console's and the hand teleporter's lists (before the DNA consoles take the rest).
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
            match sprite(world, target).as_deref() {
                Some(CONSOLE) => console_pick(world, mob, target, line),
                Some(HAND_TELE) => hand_pick(world, mob, target, line),
                _ => others.push((target, line)),
            }
        }
        if !others.is_empty()
            && let Some(mut pending) = world.get_mut::<PendingPicks>(mob)
        {
            pending.0.extend(others);
        }
    }
}

fn console_pick(world: &mut World, mob: Entity, console: Entity, line: usize) {
    let (Some(here), Some(there)) = (at(world, mob), at(world, console)) else {
        return;
    };
    if !in_reach(here, there) || !awake(world, mob) || !powered(world, console) {
        return;
    }
    let Some((_, beacon)) = beacons(world).get(line).cloned() else {
        return;
    };
    if let Some(mut c) = world.get_mut::<TeleConsole>(console) {
        c.locked = Some(beacon);
    }
    world.entity_mut(mob).remove::<PanelViewer>();
    around(world, there, "tele-locked-in");
}

/// Luna `hand_tele/attack_self`, once a line is picked.
fn hand_pick(world: &mut World, mob: Entity, tele: Entity, line: usize) {
    world.entity_mut(mob).remove::<PanelViewer>();
    if !holding(world, mob, tele) || !awake(world, mob) {
        return;
    }
    let Some(here) = at(world, mob) else {
        return;
    };
    let hubs = hubs(world);
    let to = match hubs.get(line) {
        Some((_, _, beacon)) => Destination::Beacon(*beacon),
        None if line == hubs.len() => match near_floor(world, here) {
            Some(tile) => Destination::Tile(tile),
            None => return,
        },
        None => return,
    };
    let open = {
        let mut q = world.query::<&Portal>();
        q.iter(world).filter(|p| p.creator == tele).count()
    };
    if open >= MAX_PORTALS {
        tell(world, mob, LocMsg::new("tele-recharging"));
        return;
    }
    around(world, here, "tele-locked-in");
    let now = world.resource::<WorldTime>().deciseconds();
    let id = world.resource_mut::<NetIds>().alloc();
    world.spawn((
        GridPos(here),
        id,
        Appearance::new(PORTAL, ""),
        Dense,
        Portal {
            to,
            until: now + PORTAL_DS,
            creator: tele,
            broken: false,
        },
    ));
}

/// A random floor within ten tiles, not within four of the map's edge.
fn near_floor(world: &mut World, here: TilePos) -> Option<TilePos> {
    let mut layers = world.query::<&MapLayer>();
    let map = layers.iter(world).find(|l| l.0.z == here.z)?.0.clone();
    let (w, h) = (map.width as i32, map.height as i32);
    let mut tiles = Vec::new();
    for y in here.y - DANGER_RANGE..=here.y + DANGER_RANGE {
        for x in here.x - DANGER_RANGE..=here.x + DANGER_RANGE {
            let t = TilePos::new(x, y, here.z);
            if t == here || x < 4 || y < 4 || x >= w - 4 || y >= h - 4 {
                continue;
            }
            if map.tile(t).is_some() {
                tiles.push(t);
            }
        }
    }
    if tiles.is_empty() {
        return None;
    }
    let i = world.resource_mut::<GameRng>().range(0, tiles.len() as i32 - 1) as usize;
    Some(tiles[i])
}

/// Somewhere out in space: a random space tile of any deck (Luna sends them to its space level).
fn deep_space(world: &mut World) -> Option<TilePos> {
    let mut layers = world.query::<&MapLayer>();
    let mut tiles = Vec::new();
    for layer in layers.iter(world) {
        let m = &layer.0;
        for y in 0..m.height as i32 {
            for x in 0..m.width as i32 {
                let t = TilePos::new(x, y, m.z);
                if m.tile(t).is_some_and(|t| t.id == "space") {
                    tiles.push(t);
                }
            }
        }
    }
    if tiles.is_empty() {
        return None;
    }
    let i = world.resource_mut::<GameRng>().range(0, tiles.len() as i32 - 1) as usize;
    Some(tiles[i])
}

/// Luna `do_teleport`: there, give or take `precision` tiles, with a shower of sparks.
fn teleport(world: &mut World, thing: Entity, to: TilePos, precision: i32) {
    let (dx, dy) = {
        let mut rng = world.resource_mut::<GameRng>();
        (rng.range(-precision, precision), rng.range(-precision, precision))
    };
    let dest = TilePos::new(to.x + dx, to.y + dy, to.z);
    world.entity_mut(thing).insert(GridPos(dest));
    world.resource_mut::<SparkShowers>().0.push((dest, 5));
}

fn through_portal(world: &mut World, portal: Entity, thing: Entity) {
    let Some(p) = world.get::<Portal>(portal).copied() else {
        return;
    };
    if p.broken {
        return;
    }
    let dest = match p.to {
        Destination::Beacon(b) => where_is(world, b),
        Destination::Tile(t) => Some(t),
    };
    let Some(dest) = dest else {
        // Its beacon is gone: so is the portal.
        world.entity_mut(portal).despawn();
        return;
    };
    if world.resource_mut::<GameRng>().prob(PORTAL_FAIL) {
        if let Some(mut p) = world.get_mut::<Portal>(portal) {
            p.broken = true;
        }
        world.entity_mut(portal).insert(Appearance::new(PORTAL, "broken"));
        if let Some(space) = deep_space(world) {
            teleport(world, thing, space, 0);
        }
        return;
    }
    // "You will appear adjacent to the beacon."
    teleport(world, thing, dest, 1);
}

/// Luna `teleport/hub/teleport`, when something walks into a hub that's on.
fn through_hub(world: &mut World, hub: Entity, thing: Entity) {
    let Some(tile) = at(world, hub) else {
        return;
    };
    let Some(console) = on_tile::<TeleConsole>(world, offset(tile, -2)) else {
        return;
    };
    let target = world
        .get::<TeleConsole>(console)
        .and_then(|c| c.locked)
        .and_then(|b| where_is(world, b));
    let Some(target) = target else {
        around(world, tile, "tele-no-lock");
        return;
    };
    if world.resource_mut::<GameRng>().prob(HUB_FAIL) {
        if let Some(space) = deep_space(world) {
            teleport(world, thing, space, 2);
        }
    } else {
        teleport(world, thing, target, 0);
    }
}

/// Every tick: hubs follow their stations, walking into a hub or a portal goes through it, old
/// portals close, and the lists stay up to date.
pub fn run_teleporters(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    // The hub beside each station.
    let mut q = world.query::<(Entity, &GridPos, &TeleStation, Option<&PowerUser>)>();
    let stations: Vec<(Entity, TilePos, bool, bool)> = q
        .iter(world)
        .map(|(e, p, s, power)| (e, p.0, s.engaged, power.is_none_or(|p| p.powered)))
        .collect();
    for (station, tile, engaged, power) in stations {
        let look = if power { "controller" } else { "controller-p" };
        if world.get::<Appearance>(station).is_some_and(|a| a.state != look) {
            world.entity_mut(station).insert(Appearance::new(STATION, look));
        }
        if let Some(hub) = on_tile::<TeleHub>(world, offset(tile, 1)) {
            let on = engaged && power;
            if world.get::<TeleHub>(hub).is_some_and(|h| h.on != on) {
                world.entity_mut(hub).insert((
                    TeleHub { on },
                    Appearance::new(HUB, if on { "tele1" } else { "tele0" }),
                ));
            }
        }
    }
    // Walking into them.
    let bumps = std::mem::take(&mut world.resource_mut::<Bumps>().0);
    for (mob, tile) in bumps {
        if let Some(hub) = on_tile::<TeleHub>(world, tile) {
            if world.get::<TeleHub>(hub).is_some_and(|h| h.on) {
                through_hub(world, hub, mob);
            }
        } else if let Some(portal) = on_tile::<Portal>(world, tile) {
            through_portal(world, portal, mob);
        }
    }
    // Old portals close.
    let mut q = world.query::<(Entity, &Portal)>();
    let old: Vec<Entity> = q
        .iter(world)
        .filter(|(_, p)| now >= p.until)
        .map(|(e, _)| e)
        .collect();
    for portal in old {
        world.entity_mut(portal).despawn();
    }
}

/// Keep each open list in step, and close it out of reach.
pub fn update_views(world: &mut World) {
    let mut viewers = world.query::<(Entity, &PanelViewer)>();
    let all: Vec<(Entity, Entity)> = viewers.iter(world).map(|(m, v)| (m, v.0)).collect();
    for (mob, target) in all {
        let shown = match sprite(world, target).as_deref() {
            Some(CONSOLE) => {
                let near = at(world, mob)
                    .zip(at(world, target))
                    .is_some_and(|(a, b)| in_reach(a, b));
                if near && powered(world, target) {
                    Some(beacons(world).into_iter().map(|(n, _)| n).collect::<Vec<_>>())
                } else {
                    None
                }
            }
            Some(HAND_TELE) => {
                if holding(world, mob, target) {
                    let mut rows: Vec<String> = hubs(world)
                        .into_iter()
                        .map(|(id, on, _)| {
                            if on {
                                format!("msg:tele-row-active:{id}")
                            } else {
                                format!("msg:tele-row-inactive:{id}")
                            }
                        })
                        .collect();
                    rows.push("msg:tele-row-none".to_owned());
                    Some(rows)
                } else {
                    None
                }
            }
            _ => continue,
        };
        let view = match (shown, world.get::<NetId>(target).copied(), sprite(world, target)) {
            (Some(rows), Some(id), Some(title)) => Some(PickListView {
                target: id,
                title,
                rows,
            }),
            _ => {
                world.entity_mut(mob).remove::<PanelViewer>();
                None
            }
        };
        if let Some(mut out) = world.get_mut::<PickListOut>(mob)
            && out.0 != view
        {
            out.0 = view;
        }
    }
}
