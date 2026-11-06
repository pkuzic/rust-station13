//! The derelict round (Luna's `gamemodes/derelict`): the station is found wrecked and dark, and
//! everyone comes aboard from the arrival shuttle as a scavenger with tools to bring it back.
//! Spec: `docs/reference/derelict.md`.

use std::collections::{HashMap, HashSet};

use bevy_ecs::prelude::*;
use rs13_world::{Dense, Dir, GridPos, Hand, MapLayer, ObjectSpec, Slot, TilePos};

use crate::alarms::AirAlarm;
use crate::equipment::Equipment;
use crate::explosions::{detonate_pending_now, explode, hit_tile};
use crate::hands::{Hands, HeldBy, Item};
use crate::jobs::CardOwner;
use crate::nuke::Landmark;
use crate::objects::{spawn_object, spawn_stack};
use crate::power::Apc;
use crate::rng::GameRng;
use crate::storage::{InStorage, Storage};
use crate::windows::{PaneEdges, WindowPane};

pub const MODE: &str = "derelict";
/// What a scavenger's ID card says they are.
pub const SCAVENGER: &str = "scavenger";
/// Where everyone comes aboard (Luna `area/shuttle/arrival/station`).
pub const ARRIVAL_AREA: &str = "shuttle-arrival-station";

pub const GLASS: &str = "landmark-derelict-glass";
pub const METAL: &str = "landmark-derelict-metal";
pub const NO_BLAST: &str = "landmark-derelict-noblast";
pub const NO_DAMAGE: &str = "landmark-derelict-nodamage";
pub const O2_CANISTER: &str = "landmark-derelict-o2canister";
pub const O2_CRATE: &str = "landmark-derelict-o2crate";
pub const SUPER_PACMAN: &str = "landmark-derelict-superpacman";
pub const SUPPLY_CRATE: &str = "landmark-derelict-supplycrate";

/// Present only in a derelict round.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Derelict;

/// What a supply landmark puts out (Luna `wreakstation`).
const SUPPLIES: [&str; 14] = [
    "emergency-oxygen-engi",
    "emergency-oxygen-engi",
    "emergency-oxygen-engi",
    "emergency-oxygen-engi",
    "utility-belt",
    "utility-belt",
    "toolbox-mechanical",
    "toolbox-electrical",
    "insulated-gloves",
    "insulated-gloves",
    "multitool",
    "multitool",
    "magboots",
    "magboots",
];

fn chance(world: &mut World, percent: f32) -> bool {
    world.resource_mut::<GameRng>().prob(percent)
}

fn marks(world: &mut World) -> HashMap<TilePos, Vec<String>> {
    let mut found: HashMap<TilePos, Vec<String>> = HashMap::new();
    let mut query = world.query::<(&Landmark, &GridPos)>();
    for (mark, at) in query.iter(world) {
        found.entry(at.0).or_default().push(mark.0.clone());
    }
    found
}

fn marked(marks: &HashMap<TilePos, Vec<String>>, at: TilePos, id: &str) -> bool {
    marks.get(&at).is_some_and(|m| m.iter().any(|m| m == id))
}

fn sorted<C: Component>(world: &mut World) -> Vec<(Entity, TilePos)> {
    let mut query = world.query_filtered::<(Entity, &GridPos), With<C>>();
    let mut all: Vec<(Entity, TilePos)> = query.iter(world).map(|(e, p)| (e, p.0)).collect();
    all.sort_by_key(|(e, _)| *e);
    all
}

/// Wreck the station before anyone's aboard (Luna `wreakstation`).
pub fn wreck(world: &mut World) {
    world.insert_resource(Derelict);
    let marks = marks(world);

    // Every APC flat, a quarter without a cell, and some of the rest broken.
    let mut apcs = world.query::<(Entity, &Apc)>();
    let mut apc_list: Vec<Entity> = apcs.iter(world).map(|(e, _)| e).collect();
    apc_list.sort();
    for apc in apc_list {
        let lose_cell = chance(world, 25.0);
        let broken = !lose_cell && chance(world, 40.0);
        if let Some(mut apc) = world.get_mut::<Apc>(apc) {
            apc.cell = if lose_cell {
                None
            } else {
                apc.cell.map(|(_, capacity)| (0.0, capacity))
            };
            if broken {
                apc.broken = true;
                apc.operating = false;
            }
        }
    }

    // The floors, then the walls, as a light blast would; now and then a big one.
    let mut layers = world.query::<&MapLayer>();
    let mut floors = Vec::new();
    let mut walls = Vec::new();
    for layer in layers.iter(world) {
        for y in 0..layer.0.height as i32 {
            for x in 0..layer.0.width as i32 {
                let at = TilePos::new(x, y, layer.0.z);
                let Some(tile) = layer.0.tile(at) else {
                    continue;
                };
                if tile.id == "space" {
                    continue;
                }
                if !tile.solid {
                    floors.push(at);
                } else if tile.id.contains("wall") {
                    walls.push(at);
                }
            }
        }
    }
    for at in floors {
        if chance(world, 70.0) && !marked(&marks, at, NO_DAMAGE) {
            hit_tile(world, at, 3);
        }
        if chance(world, 1.0) && chance(world, 5.0) && !marked(&marks, at, NO_BLAST) {
            explode(world, at, 3, 5, 7, 9);
        }
    }
    detonate_pending_now(world);
    for at in walls {
        if !marked(&marks, at, NO_DAMAGE) && chance(world, 40.0) {
            hit_tile(world, at, 3);
        }
    }

    // Window panes, twice over (Luna's "hacking airlocks" smashes windows again), each blow
    // breaking one half the time.
    for odds in [40.0, 65.0] {
        for (pane, at) in sorted::<WindowPane>(world) {
            if world.get_entity(pane).is_err() || marked(&marks, at, NO_DAMAGE) {
                continue;
            }
            if chance(world, odds) && chance(world, 50.0) {
                world.entity_mut(pane).despawn();
            }
        }
    }
    let edges = {
        let mut panes = world.query::<(&GridPos, &WindowPane)>();
        crate::windows::pane_edges(panes.iter(world))
    };
    world.resource_mut::<PaneEdges>().0 = edges;

    // Most air alarms broken.
    for (alarm, at) in sorted::<AirAlarm>(world) {
        if !marked(&marks, at, NO_DAMAGE)
            && chance(world, 80.0)
            && let Some(mut alarm) = world.get_mut::<AirAlarm>(alarm)
        {
            alarm.broken = true;
        }
    }

    scatter(world);
    supply(world, &marks);
}

/// Loose things shoved about, and one in ten gone.
fn scatter(world: &mut World) {
    let mut dense = world.query_filtered::<&GridPos, With<Dense>>();
    let occupied: HashSet<TilePos> = dense.iter(world).map(|p| p.0).collect();
    let sealed = world.resource::<PaneEdges>().0.clone();
    let mut layers = world.query::<&MapLayer>();
    let maps: HashMap<i32, rs13_world::TileMap> = layers.iter(world).map(|l| (l.0.z, l.0.clone())).collect();
    for (item, mut at) in sorted::<Item>(world) {
        let steps = world.resource_mut::<GameRng>().range(1, 5) + 1;
        for _ in 0..steps {
            let dir = [Dir::North, Dir::South, Dir::East, Dir::West]
                [world.resource_mut::<GameRng>().range(0, 3) as usize];
            let Some(map) = maps.get(&at.z) else {
                break;
            };
            if let Some(to) = crate::movement::resolve_step(map, &occupied, &sealed, at, dir) {
                at = to;
            }
        }
        world.entity_mut(item).insert(GridPos(at));
        if chance(world, 10.0) {
            gone(world, item);
        }
    }
}

/// Gone, with whatever's in it.
fn gone(world: &mut World, item: Entity) {
    let inside = world
        .get::<Storage>(item)
        .map(|s| s.items.clone())
        .unwrap_or_default();
    for thing in inside {
        if world.get_entity(thing).is_ok() {
            world.entity_mut(thing).despawn();
        }
    }
    world.entity_mut(item).despawn();
}

fn put(world: &mut World, proto: &str, at: TilePos) -> Option<Entity> {
    spawn_object(world, &ObjectSpec::new(proto), at).ok()
}

/// What the supply landmarks put out.
fn supply(world: &mut World, marks: &HashMap<TilePos, Vec<String>>) {
    let mut places: Vec<(TilePos, String)> = marks
        .iter()
        .flat_map(|(at, ids)| ids.iter().map(move |id| (*at, id.clone())))
        .collect();
    places.sort_by_key(|(at, id)| (at.z, at.y, at.x, id.clone()));
    for (at, id) in places {
        match id.as_str() {
            GLASS | METAL => {
                let amount = world.resource_mut::<GameRng>().range(5, 50) as u32;
                let sheet = if id == GLASS { "glass-sheet" } else { "metal-sheet" };
                let _ = spawn_stack(world, sheet, at, amount);
            }
            O2_CANISTER => {
                let proto = if chance(world, 40.0) {
                    "canister"
                } else {
                    "oxygen-canister"
                };
                put(world, proto, at);
            }
            O2_CRATE => {
                put(world, "crate-internals-oxy", at);
            }
            SUPER_PACMAN => {
                put(world, crate::generators::SUPER_PACMAN, at);
                for _ in 0..2 {
                    let _ = spawn_stack(world, crate::generators::URANIUM_SHEET, at, 50);
                }
            }
            SUPPLY_CRATE => {
                for proto in SUPPLIES {
                    put(world, proto, at);
                }
                put(world, "crate-engineering", at);
            }
            _ => {}
        }
    }
}

/// Everyone aboard is kitted out and brought in on the arrival shuttle (Luna `post_setup`).
pub fn setup_round(world: &mut World) {
    let mut people =
        world.query_filtered::<Entity, (With<crate::objects::PlayerControlled>, With<Equipment>)>();
    let mut all: Vec<Entity> = people.iter(world).collect();
    all.sort();
    for mob in all {
        arrive(world, mob);
    }
}

/// Someone joining late arrives the same way (Luna `latespawn`).
pub fn late_join(world: &mut World, mob: Entity) {
    if world.get_resource::<Derelict>().is_some() {
        arrive(world, mob);
    }
}

fn arrive(world: &mut World, mob: Entity) {
    equip_scavenger(world, mob);
    if let Some(at) = landing(world) {
        world.entity_mut(mob).insert(GridPos(at));
    }
}

/// A clear tile of the arrival shuttle.
fn landing(world: &mut World) -> Option<TilePos> {
    let mut dense = world.query_filtered::<&GridPos, With<Dense>>();
    let taken: HashSet<TilePos> = dense.iter(world).map(|p| p.0).collect();
    let mut layers = world.query::<&MapLayer>();
    let mut clear = Vec::new();
    for layer in layers.iter(world) {
        for y in 0..layer.0.height as i32 {
            for x in 0..layer.0.width as i32 {
                let at = TilePos::new(x, y, layer.0.z);
                if layer.0.area(at) == Some(ARRIVAL_AREA)
                    && layer.0.tile(at).is_some_and(|t| !t.solid)
                    && !taken.contains(&at)
                {
                    clear.push(at);
                }
            }
        }
    }
    if clear.is_empty() {
        return None;
    }
    let i = world.resource_mut::<GameRng>().range(0, clear.len() as i32 - 1) as usize;
    Some(clear[i])
}

/// The slots Luna empties before kitting someone out: all but the back and the hands.
const STRIPPED: [Slot; 10] = [
    Slot::Id,
    Slot::Shoes,
    Slot::Belt,
    Slot::Gloves,
    Slot::Glasses,
    Slot::Head,
    Slot::Mask,
    Slot::Ears,
    Slot::Suit,
    Slot::Uniform,
];

fn wear(world: &mut World, mob: Entity, proto: &str) {
    if let Some(item) = put_near(world, mob, proto) {
        crate::jobs::wear(world, mob, item);
    }
}

fn put_near(world: &mut World, mob: Entity, proto: &str) -> Option<Entity> {
    let at = world.get::<GridPos>(mob).map(|p| p.0)?;
    put(world, proto, at)
}

fn pack(world: &mut World, mob: Entity, proto: &str) {
    let Some(bag) = world.get::<Equipment>(mob).and_then(|e| e.get(Slot::Back)) else {
        return;
    };
    let Some(item) = put_near(world, mob, proto) else {
        return;
    };
    world.entity_mut(item).remove::<GridPos>().insert(InStorage(bag));
    if let Some(mut storage) = world.get_mut::<Storage>(bag) {
        storage.items.push(item);
    }
}

/// Luna `equip_scavenger`. Pockets aren't built, so what Luna puts in them goes in the backpack.
pub fn equip_scavenger(world: &mut World, mob: Entity) {
    for slot in STRIPPED {
        let Some(old) = world
            .get_mut::<Equipment>(mob)
            .and_then(|mut e| e.0.remove(&slot))
        else {
            continue;
        };
        gone(world, old);
    }
    wear(world, mob, "headset");
    wear(world, mob, "firesuit");
    wear(world, mob, "black-jumpsuit");
    wear(world, mob, "gas-mask");
    wear(world, mob, "utility-belt");
    wear(world, mob, "hardhat-red");
    if world
        .get::<Equipment>(mob)
        .and_then(|e| e.get(Slot::Back))
        .is_none()
    {
        wear(world, mob, "backpack");
    }
    pack(world, mob, "emergency-oxygen-engi");
    pack(world, mob, "circuit-box");
    pack(world, mob, "high-power-cell");
    pack(world, mob, "high-power-cell");

    if let Some(card) = put_near(world, mob, "id-card") {
        world.entity_mut(card).insert(CardOwner {
            owner: Some(mob),
            job: SCAVENGER.to_owned(),
        });
        crate::jobs::wear(world, mob, card);
    }

    let toolbox = if chance(world, 55.0) {
        "toolbox-mechanical"
    } else {
        "toolbox-electrical"
    };
    let left_free = world.get::<Hands>(mob).is_some_and(|h| h.left.is_none());
    if left_free && let Some(item) = put_near(world, mob, toolbox) {
        world.entity_mut(item).remove::<GridPos>().insert(HeldBy(mob));
        if let Some(mut hands) = world.get_mut::<Hands>(mob) {
            *hands.slot_mut(Hand::Left) = Some(item);
        }
    }
    let shoes = if chance(world, 50.0) {
        "magboots"
    } else {
        "black-shoes"
    };
    wear(world, mob, shoes);
    if chance(world, 45.0) {
        pack(world, mob, "multitool");
    }
    if chance(world, 40.0) {
        wear(world, mob, "insulated-gloves");
    }
}
