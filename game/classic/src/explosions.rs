//! Explosions (Luna's `explosion`, and the `ex_act` of what they hit): a blast that spreads from a
//! point, held back by walls and doors, breaking what it reaches according to how close it was. Spec:
//! `docs/reference/explosions.md`.

use std::collections::{HashMap, VecDeque};

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_simulation::atmos::Cell;
use rs13_world::{Appearance, Ghost, GridPos, MapLayer, ObjectSpec, Slot, TileDef, TilePos};

use crate::atmos::StationAtmos;
use crate::cells::PowerCell;
use crate::closets::{ClosetLock, InCloset};
use crate::combat::{COUNT_DS, Paralyzed, VisibleEvents};
use crate::construction::dismantle_wall;
use crate::doors::{Airlock, DoorState};
use crate::equipment::{Equipment, TankGas, Wearable, WornBy};
use crate::hands::{Hands, HeldBy, Item};
use crate::health::{Body, Zone};
use crate::movement::Mob;
use crate::objects::spawn_object;
use crate::power::PowerUser;
use crate::rng::GameRng;
use crate::sound::{EMP, EXPLOSION, EXPLOSION_FAR, SoundEvent, SoundEvents};
use crate::storage::{InStorage, Storage};
use crate::time::WorldTime;
use crate::weapons::{Baton, Fires, Gun};

/// How long between the bang being set off and the blast, ds (Luna's `sleep(5)`).
const FUSE_DS: f32 = 5.0;
/// What holds a blast back, in tiles of distance (Luna's `explosionstrength`, `floorstrength`).
const OPEN_STRENGTH: i32 = 1;
const WALL_STRENGTH: i32 = 4;
const REINFORCED_STRENGTH: i32 = 5;
const FLOOR_STRENGTH: i32 = 6;
const DOOR_STRENGTH: i32 = 3;
const WINDOW_STRENGTH: i32 = 1;
/// A burning gas tank's limits, kPa (Luna's `TANK_LEAK_PRESSURE` and so on, at 101.325 an atmosphere).
const ATMOSPHERE: f64 = 101.325;
const TANK_LEAK: f64 = 30.0 * ATMOSPHERE;
const TANK_RUPTURE: f64 = 40.0 * ATMOSPHERE;
const TANK_FRAGMENT: f64 = 50.0 * ATMOSPHERE;
const TANK_SCALE: f64 = 10.0 * ATMOSPHERE;
/// How often a tank is looked at, in ticks: once a second.
const TANK_TICKS: u64 = 20;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Blast {
    pub at: TilePos,
    pub devastation: i32,
    pub heavy: i32,
    pub light: i32,
    pub flash: i32,
    pub goes_off_at: f32,
}

/// Blasts on their way.
#[derive(Resource, Debug, Default)]
pub struct PendingBlasts(pub Vec<Blast>);

/// Set off an explosion here, by Luna's four ranges.
pub fn explode(world: &mut World, at: TilePos, devastation: i32, heavy: i32, light: i32, flash: i32) {
    let goes_off_at = world.resource::<WorldTime>().deciseconds() + FUSE_DS;
    world.resource_mut::<PendingBlasts>().0.push(Blast {
        at,
        devastation,
        heavy,
        light,
        flash,
        goes_off_at,
    });
}

/// Every blast waiting, set off at once (for wreckage made before anyone's aboard).
pub fn detonate_pending_now(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    for blast in &mut world.resource_mut::<PendingBlasts>().0 {
        blast.goes_off_at = now;
    }
    run_blasts(world);
}

/// Blasts whose time has come.
pub fn run_blasts(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    let mut pending = std::mem::take(&mut world.resource_mut::<PendingBlasts>().0);
    let due: Vec<Blast> = pending.iter().filter(|b| now >= b.goes_off_at).copied().collect();
    pending.retain(|b| now < b.goes_off_at);
    world.resource_mut::<PendingBlasts>().0.extend(pending);
    for blast in due {
        detonate(world, blast);
    }
}

fn tile_at(world: &mut World, pos: TilePos) -> Option<TileDef> {
    let mut maps = world.query::<&MapLayer>();
    maps.iter(world)
        .find(|m| m.0.z == pos.z)
        .and_then(|m| m.0.tile(pos).cloned())
}

/// How far each tile is from the blast, in what it takes to get there.
fn flood(world: &mut World, blast: Blast) -> HashMap<TilePos, i32> {
    let mut maps = world.query::<&MapLayer>();
    let decks: HashMap<i32, rs13_world::TileMap> = maps.iter(world).map(|m| (m.0.z, m.0.clone())).collect();
    let mut doors = world.query::<(&GridPos, &Airlock)>();
    let mut blocking: HashMap<TilePos, i32> = doors
        .iter(world)
        .filter(|(_, d)| d.state == DoorState::Closed)
        .map(|(p, _)| (p.0, DOOR_STRENGTH))
        .collect();
    let mut looks = world.query::<(&GridPos, &Appearance)>();
    for (at, look) in looks.iter(world) {
        if look.sprite.contains("window") {
            blocking.entry(at.0).or_insert(WINDOW_STRENGTH);
        }
    }
    let strength = |pos: TilePos| {
        let tile = decks.get(&pos.z).and_then(|m| m.tile(pos));
        let own = match tile.map(|t| t.id.as_str()) {
            Some("r-wall") => REINFORCED_STRENGTH,
            Some(_) if tile.is_some_and(|t| t.solid) => WALL_STRENGTH,
            _ => OPEN_STRENGTH,
        };
        own + blocking.get(&pos).copied().unwrap_or(0)
    };
    let mut distance: HashMap<TilePos, i32> = HashMap::new();
    distance.insert(blast.at, 0);
    let mut queue = VecDeque::from([blast.at]);
    let reach = blast
        .flash
        .max(blast.light)
        .max(blast.heavy)
        .max(blast.devastation);
    while let Some(here) = queue.pop_front() {
        let so_far = distance[&here];
        if so_far > reach {
            continue;
        }
        let steps = [
            (TilePos::new(here.x + 1, here.y, here.z), None),
            (TilePos::new(here.x - 1, here.y, here.z), None),
            (TilePos::new(here.x, here.y + 1, here.z), None),
            (TilePos::new(here.x, here.y - 1, here.z), None),
            (TilePos::new(here.x, here.y, here.z + 1), Some(FLOOR_STRENGTH)),
            (TilePos::new(here.x, here.y, here.z - 1), Some(FLOOR_STRENGTH)),
        ];
        for (next, through_floor) in steps {
            if decks.get(&next.z).and_then(|m| m.tile(next)).is_none() {
                continue;
            }
            let further = so_far + through_floor.unwrap_or_else(|| strength(next));
            if distance.get(&next).is_none_or(|d| further < *d) {
                distance.insert(next, further);
                queue.push_back(next);
            }
        }
    }
    distance
}

/// The tiles a small blast reaches, walls or not (Luna's `tg_explosion`, for what its flood handles
/// badly).
fn small_blast(blast: Blast) -> HashMap<TilePos, i32> {
    let reach = blast
        .flash
        .max(blast.light)
        .max(blast.heavy)
        .max(blast.devastation)
        .max(0);
    let mut distance = HashMap::new();
    for x in -reach..=reach {
        for y in -reach..=reach {
            let (dx, dy) = (x.abs() as f32, y.abs() as f32);
            let far = dx.max(dy) + 0.5 * dx.min(dy);
            distance.insert(
                TilePos::new(blast.at.x + x, blast.at.y + y, blast.at.z),
                far.ceil() as i32,
            );
        }
    }
    distance
}

fn detonate(world: &mut World, blast: Blast) {
    let distance = if blast.devastation <= 1 {
        small_blast(blast)
    } else {
        flood(world, blast)
    };
    let mut order: Vec<(TilePos, i32)> = distance.into_iter().collect();
    order.sort_by_key(|(at, d)| (*d, at.x, at.y, at.z));
    for (at, far) in order {
        let (things, tile) = if far < blast.devastation {
            let tile = if world.resource_mut::<GameRng>().prob(5.0) {
                2
            } else {
                1
            };
            (Some(1), Some(tile))
        } else if far <= blast.heavy {
            (Some(2), Some(2))
        } else if far <= blast.light {
            (Some(3), Some(3))
        } else {
            (None, None)
        };
        if far <= blast.flash {
            flash_at(world, at);
        }
        if let Some(severity) = things {
            hit_things(world, at, severity, false);
        }
        if let Some(severity) = tile {
            hit_tile(world, at, severity);
        }
    }
    // Everyone hears it from afar, and those near it hear it properly (Luna's two sounds).
    let mut sounds = world.resource_mut::<SoundEvents>();
    sounds
        .0
        .push(SoundEvent::new(EXPLOSION, blast.at, 100.0).extra_range(blast.devastation.max(1)));
    let mut people = world.query_filtered::<&GridPos, With<Mob>>();
    let everyone: Vec<TilePos> = people.iter(world).map(|p| p.0).collect();
    let mut sounds = world.resource_mut::<SoundEvents>();
    for at in everyone {
        sounds.0.push(SoundEvent::new(EXPLOSION_FAR, at, 60.0));
    }
    let msg = LocMsg::new("explosion-heard");
    world
        .resource_mut::<VisibleEvents>()
        .0
        .push((blast.at, msg, None));
}

/// How much power an EMP takes out of what it hits, by severity (Luna's `1000 / severity`).
const EMP_DRAIN: i32 = 1000;

/// An electromagnetic pulse (Luna's `empulse`): cells and energy weapons lose charge, and locks can fail,
/// out to `light` tiles. At `heavy` it's a coin toss which it is, and closer is always the worse.
pub fn empulse(world: &mut World, at: TilePos, heavy: i32, light: i32) {
    let light = light.max(heavy);
    world
        .resource_mut::<SoundEvents>()
        .0
        .push(SoundEvent::new(EMP, at, 100.0));
    let mut guns = world.query_filtered::<Entity, Or<(With<Gun>, With<Baton>, With<PowerCell>)>>();
    let charged: Vec<Entity> = guns.iter(world).collect();
    for thing in charged {
        let Some(here) = tank_at(world, thing) else {
            continue;
        };
        let Some(severity) = emp_severity(world, at, here, heavy, light) else {
            continue;
        };
        let drain = EMP_DRAIN / severity;
        if let Some(mut gun) = world.get_mut::<Gun>(thing)
            && !matches!(gun.fires, Fires::Bullet(_))
        {
            gun.charge = (gun.charge - drain).max(0);
        }
        if let Some(mut baton) = world.get_mut::<Baton>(thing) {
            baton.charge = (baton.charge - drain).max(0);
        }
        if let Some(mut cell) = world.get_mut::<PowerCell>(thing) {
            cell.charge = (cell.charge - f64::from(drain)).max(0.0);
        }
    }
    let mut locks = world.query::<(Entity, &GridPos, &ClosetLock)>();
    let closets: Vec<(Entity, TilePos, ClosetLock)> =
        locks.iter(world).map(|(e, p, l)| (e, p.0, l.clone())).collect();
    for (closet, here, lock) in closets {
        let Some(severity) = emp_severity(world, at, here, heavy, light) else {
            continue;
        };
        if world.resource_mut::<GameRng>().prob(50.0 / severity as f32) {
            world.entity_mut(closet).insert(ClosetLock {
                locked: !lock.locked,
                ..lock
            });
        }
    }
}

fn emp_severity(world: &mut World, at: TilePos, here: TilePos, heavy: i32, light: i32) -> Option<i32> {
    if here.z != at.z {
        return None;
    }
    let distance = (here.x - at.x).abs().max((here.y - at.y).abs());
    if distance < heavy {
        Some(1)
    } else if distance == heavy {
        Some(if world.resource_mut::<GameRng>().prob(50.0) {
            1
        } else {
            2
        })
    } else if distance <= light {
        Some(2)
    } else {
        None
    }
}

fn flash_at(world: &mut World, at: TilePos) {
    let mut mobs = world.query_filtered::<(Entity, &GridPos), With<Mob>>();
    let here: Vec<Entity> = mobs
        .iter(world)
        .filter(|(_, p)| p.0 == at)
        .map(|(e, _)| e)
        .collect();
    for mob in here {
        crate::flashes::dazzle(world, mob);
    }
}

fn replace_tile(world: &mut World, at: TilePos, id: &str, solid: bool) {
    let mut maps = world.query::<&mut MapLayer>();
    if let Some(mut map) = maps.iter_mut(world).find(|m| m.0.z == at.z) {
        map.0.set_tile(
            at,
            TileDef {
                id: id.to_owned(),
                solid,
                opaque: None,
                airtight: None,
            },
        );
    }
}

fn set_cell(world: &mut World, at: TilePos, cell: Cell) {
    if let Some(mut atmos) = world.get_resource_mut::<StationAtmos>() {
        atmos.0.set_cell(at, cell);
    }
}

fn metal(world: &mut World, at: TilePos, sheets: u32) {
    let _ = crate::objects::spawn_stack(world, "metal-sheet", at, sheets);
}

fn open_to_space(world: &mut World, at: TilePos) {
    replace_tile(world, at, "space", false);
    set_cell(world, at, Cell::Space);
}

fn light_up(world: &mut World, at: TilePos) {
    if let Some(mut atmos) = world.get_resource_mut::<StationAtmos>() {
        atmos.0.expose(at, 1000.0);
    }
}

/// Luna's `wall/ex_act` and `floor/ex_act`. Where it opens the floor to the deck below, there's no such
/// thing here yet, so it's open to space.
pub fn hit_tile(world: &mut World, at: TilePos, severity: u8) {
    let Some(tile) = tile_at(world, at) else {
        return;
    };
    if tile.id == "space" {
        return;
    }
    let mut chance = |percent: f32| world.resource_mut::<GameRng>().prob(percent);
    if tile.solid {
        match severity {
            1 => open_to_space(world, at),
            2 => {
                let devastated = chance(50.0);
                dismantle_wall(world, at, devastated);
            }
            _ => {
                if chance(20.0) {
                    dismantle_wall(world, at, false);
                }
            }
        }
        return;
    }
    let floored = tile.id == "floor";
    match severity {
        1 => open_to_space(world, at),
        2 => {
            let roll = world.resource_mut::<GameRng>().range(1, 275);
            if roll <= 175 {
                open_to_space(world, at);
            } else if floored {
                replace_tile(world, at, "plating", false);
                light_up(world, at);
            }
            if world.resource_mut::<GameRng>().prob(33.0) {
                metal(world, at, 1);
            }
        }
        _ => {
            if chance(50.0) {
                if floored {
                    replace_tile(world, at, "plating", false);
                }
                light_up(world, at);
            }
        }
    }
}

/// What a blast does to what's on a tile. Things held or inside go with what they're in, unless
/// that's opened up.
/// Hit what's here, and the tile itself, as a blast would (for meteors and the blob).
pub fn strike(world: &mut World, at: TilePos, severity: u8) {
    hit_things(world, at, severity, false);
    hit_tile(world, at, severity);
}

/// The same, leaving whoever's here to the caller (the blob has its own way with people).
pub fn strike_sparing_people(world: &mut World, at: TilePos, severity: u8) {
    hit_things(world, at, severity, true);
    hit_tile(world, at, severity);
}

fn hit_things(world: &mut World, at: TilePos, severity: u8, spare_people: bool) {
    let mut things = world.query::<(Entity, &GridPos, Has<Mob>, Has<Ghost>, Has<Item>, Has<PowerUser>)>();
    let here: Vec<(Entity, bool, bool, bool, bool)> = things
        .iter(world)
        .filter(|(_, p, ..)| p.0 == at)
        .map(|(e, _, mob, ghost, item, machine)| (e, mob, ghost, item, machine))
        .collect();
    for (thing, mob, ghost, item, machine) in here {
        if ghost {
            continue;
        }
        if mob {
            if !spare_people {
                hit_person(world, thing, severity);
            }
            continue;
        }
        if world.get::<crate::blob::BlobCell>(thing).is_some() {
            crate::blob::blasted(world, thing, severity);
            continue;
        }
        let chance = match (severity, item, machine) {
            (1, ..) => 100.0,
            (2, ..) => 50.0,
            (_, true, _) => 5.0,
            (_, _, true) => 25.0,
            _ => 0.0,
        };
        if world.resource_mut::<GameRng>().prob(chance) {
            destroy(world, thing, at);
        }
    }
}

/// Gone, and what was in it left on the floor.
fn destroy(world: &mut World, thing: Entity, at: TilePos) {
    let mut inside = world.query::<(Entity, Option<&InCloset>, Option<&InStorage>)>();
    let contents: Vec<Entity> = inside
        .iter(world)
        .filter(|(_, closet, storage)| {
            closet.is_some_and(|c| c.0 == thing) || storage.is_some_and(|s| s.0 == thing)
        })
        .map(|(e, ..)| e)
        .collect();
    for held in contents {
        world
            .entity_mut(held)
            .remove::<(InCloset, InStorage)>()
            .insert(GridPos(at));
    }
    if world.get::<Storage>(thing).is_some()
        && let Some(mut storage) = world.get_mut::<Storage>(thing)
    {
        storage.items.clear();
    }
    let door = world.get::<Airlock>(thing).is_some();
    world.entity_mut(thing).despawn();
    // Air goes through where a door was.
    if door {
        set_cell(world, at, Cell::Open);
    }
}

/// How much armour someone has against a bomb, on average over their body (Luna's `getarmor(null)`).
fn bomb_armour(world: &World, mob: Entity) -> f32 {
    let Some(equipment) = world.get::<Equipment>(mob) else {
        return 0.0;
    };
    let total: f32 = Zone::ALL
        .into_iter()
        .map(|zone| -> f32 {
            [Slot::Head, Slot::Mask, Slot::Suit, Slot::Uniform]
                .into_iter()
                .filter_map(|slot| equipment.get(slot))
                .filter_map(|item| world.get::<Wearable>(item))
                .filter(|worn| worn.covers & zone.cover_bit() != 0)
                .map(|worn| f32::from(worn.armor.bomb))
                .sum()
        })
        .sum();
    total / Zone::ALL.len() as f32
}

/// Luna's `human/ex_act`.
pub fn hit_person(world: &mut World, mob: Entity, severity: u8) {
    let armour = bomb_armour(world, mob);
    let mut rng = world.resource_mut::<GameRng>();
    let shielded = rng.prob(armour);
    let (mut brute, mut burn, knocked_out) = match severity {
        1 => (250.0, 0.0, false),
        2 => (60.0, 60.0, rng.prob(70.0)),
        _ => (30.0, 0.0, rng.prob(50.0)),
    };
    if shielded {
        let by = if severity == 3 { 4.0 } else { 2.0 };
        brute /= by;
        burn /= by;
    }
    // At full strength with nothing to stop it, there's nothing left to save.
    let gone = severity == 1 && !shielded;
    if let Some(mut body) = world.get_mut::<Body>(mob) {
        for zone in Zone::ALL {
            let share = match zone {
                Zone::Head => 0.2,
                Zone::Chest => 0.4,
                Zone::Groin => 0.1,
                Zone::LArm | Zone::RArm | Zone::LLeg | Zone::RLeg => 0.05,
                _ => 0.0225,
            };
            let scale = if gone { 10.0 } else { 1.0 };
            body.take_damage(zone, brute * share * scale, burn * share);
        }
    }
    if knocked_out {
        let until = world.resource::<WorldTime>().deciseconds() + 10.0 * COUNT_DS;
        world.entity_mut(mob).insert(Paralyzed(until));
    }
}

/// Where a tank is: on the floor, or on whoever has it.
fn tank_at(world: &World, tank: Entity) -> Option<TilePos> {
    let on = |e: Entity| world.get::<GridPos>(e).map(|p| p.0);
    on(tank)
        .or_else(|| world.get::<HeldBy>(tank).and_then(|h| on(h.0)))
        .or_else(|| world.get::<WornBy>(tank).and_then(|w| on(w.0)))
        .or_else(|| crate::devices::where_is(world, tank))
}

fn drop_from_hands(world: &mut World, tank: Entity) {
    if let Some(holder) = world.get::<HeldBy>(tank).map(|h| h.0)
        && let Some(mut hands) = world.get_mut::<Hands>(holder)
    {
        if hands.left == Some(tank) {
            hands.left = None;
        }
        if hands.right == Some(tank) {
            hands.right = None;
        }
    }
    if let Some(wearer) = world.get::<WornBy>(tank).map(|w| w.0)
        && let Some(mut equipment) = world.get_mut::<Equipment>(wearer)
    {
        equipment.0.retain(|_, item| *item != tank);
    }
}

/// What's in a tank goes on burning, and if the pressure gets too much it leaks, ruptures or blows
/// up (Luna's `tank/process`, `check_status`).
pub fn run_tanks(world: &mut World) {
    if !world.resource::<WorldTime>().tick.0.is_multiple_of(TANK_TICKS) {
        return;
    }
    let mut tanks = world.query::<(Entity, &TankGas)>();
    let all: Vec<Entity> = tanks.iter(world).map(|(e, _)| e).collect();
    for tank in all {
        let Some(mut gas) = world.get_mut::<TankGas>(tank) else {
            continue;
        };
        gas.mix.burn(1.0);
        let pressure = gas.pressure();
        if pressure > TANK_FRAGMENT {
            for _ in 0..3 {
                gas.mix.burn(1.0);
            }
            let range = ((gas.pressure() - TANK_FRAGMENT) / TANK_SCALE).min(14.0);
            let Some(at) = tank_at(world, tank) else {
                continue;
            };
            drop_from_hands(world, tank);
            world.entity_mut(tank).despawn();
            explode(
                world,
                at,
                (range * 0.24).round() as i32,
                (range * 0.5).round() as i32,
                range.round() as i32,
                (range * 1.5).round() as i32,
            );
        } else if pressure > TANK_LEAK {
            if gas.integrity > 0 {
                gas.integrity -= 1;
                continue;
            }
            let spill = if pressure > TANK_RUPTURE { None } else { Some(0.25) };
            let mix = match spill {
                Some(share) => {
                    let leaving = gas.mix.scaled(share);
                    gas.mix = gas.mix.scaled(1.0 - share);
                    leaving
                }
                None => gas.mix,
            };
            let at = tank_at(world, tank);
            if let (Some(at), Some(mut atmos)) = (at, world.get_resource_mut::<StationAtmos>()) {
                atmos.0.add_at(at, &mix);
            }
            if spill.is_none() {
                drop_from_hands(world, tank);
                world.entity_mut(tank).despawn();
            }
        } else if gas.integrity < 3 {
            gas.integrity += 1;
        }
    }
}

pub const C4_SMALL: &str = "c4-small";
pub const C4_LARGE: &str = "c4-large";
pub const DETONATOR: &str = "c4-detonator";
/// From pressing the button to the bang, ds (Luna's 50).
const C4_FUSE_DS: f32 = 50.0;

/// A charge, how big, and when it goes off if it's been set off.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Charge {
    pub power: i32,
    pub goes_off_at: Option<f32>,
}

/// What a detonator sets off, and whether its lid is open.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Detonator {
    pub charge: Entity,
    pub open: bool,
}

/// A new charge, with its detonator beside it (Luna's `c4explosive/New`). The charge, and the detonator.
pub fn spawn_charge(world: &mut World, proto: &str, at: TilePos) -> Option<(Entity, Entity)> {
    let charge = spawn_object(world, &ObjectSpec::new(proto), at).ok()?;
    let power = if proto == C4_LARGE { 2 } else { 1 };
    let idle = if proto == C4_LARGE {
        "c-4large_0"
    } else {
        "c-4small_0"
    };
    world.entity_mut(charge).insert((
        Charge {
            power,
            goes_off_at: None,
        },
        Appearance::new(proto, idle),
    ));
    let detonator = spawn_object(world, &ObjectSpec::new(DETONATOR), at).ok()?;
    world.entity_mut(detonator).insert((
        Detonator { charge, open: false },
        Appearance::new(DETONATOR, "c-4detonator_0"),
    ));
    Some((charge, detonator))
}

/// The detonator is a lighter until it's opened, and then its button sets the charge off.
pub fn use_detonator(world: &mut World, detonator: Entity) {
    let Some(state) = world.get::<Detonator>(detonator).copied() else {
        return;
    };
    if !state.open {
        world.entity_mut(detonator).insert((
            Detonator { open: true, ..state },
            Appearance::new(DETONATOR, "c-4detonator_1"),
        ));
        return;
    }
    let now = world.resource::<WorldTime>().deciseconds();
    if let Some(mut charge) = world.get_mut::<Charge>(state.charge)
        && charge.goes_off_at.is_none()
    {
        charge.goes_off_at = Some(now + C4_FUSE_DS);
        let lit = if charge.power > 1 {
            "c-4large_1"
        } else {
            "c-4small_1"
        };
        let proto = if charge.power > 1 { C4_LARGE } else { C4_SMALL };
        world.entity_mut(state.charge).insert(Appearance::new(proto, lit));
    }
}

/// Charges whose time has come. Walls on all sides come down, and doors fly open (Luna `detonate`).
pub fn run_charges(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    let mut charges = world.query::<(Entity, &Charge)>();
    let due: Vec<(Entity, i32)> = charges
        .iter(world)
        .filter(|(_, c)| c.goes_off_at.is_some_and(|at| now >= at))
        .map(|(e, c)| (e, c.power))
        .collect();
    for (charge, power) in due {
        let Some(at) = tank_at(world, charge) else {
            continue;
        };
        drop_from_hands(world, charge);
        world.entity_mut(charge).despawn();
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let next = TilePos::new(at.x + dx, at.y + dy, at.z);
            if tile_at(world, next).is_some_and(|t| t.solid) {
                dismantle_wall(world, next, true);
            }
            let mut doors = world.query::<(Entity, &GridPos, &Airlock)>();
            let door = doors
                .iter(world)
                .find(|(_, p, a)| p.0 == next && a.state == DoorState::Closed)
                .map(|(e, ..)| e);
            if let Some(door) = door
                && let Some(mut airlock) = world.get_mut::<Airlock>(door)
            {
                airlock.force(now);
            }
        }
        explode(world, at, power, power * 2, power * 3, power * 4);
    }
}
