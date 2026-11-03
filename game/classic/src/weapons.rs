//! Energy guns, their shots, stun batons and rechargers (Luna's `gun/energy`, `projectile`,
//! `melee/baton`, `machinery/recharger`). Spec: `docs/reference/weapons.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, Dense, DisplayName, GridPos, MapLayer, MobStat, NetId, TilePos};

use crate::actions::Outbox;
use crate::closets::InCloset;
use crate::combat::VisibleEvents;
use crate::flashes::Weakened;
use crate::hands::{Hands, HeldBy};
use crate::health::{Body, Vitals, Zone, ran_zone};
use crate::machines::MACHINE_TICKS;
use crate::power::{Channel, PowerUser};
use crate::rng::GameRng;
use crate::sound::SoundEvents;
use crate::time::WorldTime;

pub const LASER_GUN: &str = "laser-gun";
pub const ENERGY_GUN: &str = "energy-gun";
pub const TASER: &str = "taser";
pub const BATON: &str = "stun-baton";
pub const RECHARGER: &str = "recharger";
pub const SHOT: &str = "shot";

/// What one shot uses (Luna `charge_cost`).
pub const SHOT_COST: i32 = 100;
/// A laser or energy gun's cell, and the taser's weaker one (Luna `cell/gun`, `cell/crap`).
pub const GUN_CELL: i32 = 1000;
pub const TASER_CELL: i32 = 500;
/// A baton's cell, and what a hit takes (Luna `cell/high`, `hitcost`).
pub const BATON_CELL: i32 = 10_000;
pub const BATON_HIT: i32 = 1000;
/// What a beam burns (Luna `projectile/beam`).
pub const BEAM_BURN: f32 = 20.0;
/// How long a taser shot knocks you down, ds (Luna `weaken = 10`, at 2 s a life tick), and a baton.
pub const ELECTRODE_WEAKEN_DS: f32 = 200.0;
pub const BATON_WEAKEN_DS: f32 = 200.0;
/// A shot's reach in tiles (Luna `kill_count`), and how fast it goes, in ticks a tile.
pub const SHOT_RANGE: i32 = 50;
pub const TICKS_PER_TILE: u64 = 2;
/// Rest between shots, ds (Luna `next_move`).
pub const SHOT_REST_DS: f32 = 4.0;
/// What a recharger gives each cycle: a gun, and a baton (Luna `give`).
pub const RECHARGE_GUN: i32 = 100;
pub const RECHARGE_BATON: i32 = 250;
/// What a recharger draws, W, charging and idle.
pub const RECHARGE_WATTS: f64 = 250.0;
pub const RECHARGER_IDLE_WATTS: f64 = 4.0;

/// What a gun fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fires {
    Beam,
    Electrode,
}

/// An energy gun: its charge, and for the two-way one, its setting.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gun {
    pub charge: i32,
    pub capacity: i32,
    pub fires: Fires,
    pub switchable: bool,
    pub last_shot_ds: Option<u32>,
}

/// A stun baton: its charge, and whether it is on.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Baton {
    pub charge: i32,
    pub on: bool,
}

/// A shot in flight, along its path. It hits what it first runs into.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct Shot {
    pub firer: Entity,
    pub fires: Fires,
    pub path: Vec<TilePos>,
    pub next: usize,
    /// Where it was aimed: someone lying here is hit as well, not stepped over.
    pub aimed_at: TilePos,
}

/// A recharger, and what is charging in it.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Recharger {
    pub charging: Option<Entity>,
}

/// Fit the components a new weapon needs.
pub fn fit_weapon(entity: &mut EntityWorldMut, proto: &str) {
    match proto {
        LASER_GUN | ENERGY_GUN | TASER => {
            let (capacity, fires, switchable) = match proto {
                LASER_GUN => (GUN_CELL, Fires::Beam, false),
                ENERGY_GUN => (GUN_CELL, Fires::Electrode, true),
                _ => (TASER_CELL, Fires::Electrode, false),
            };
            let gun = Gun {
                charge: capacity,
                capacity,
                fires,
                switchable,
                last_shot_ds: None,
            };
            let state = gun_look(proto, &gun);
            entity.insert((gun, Appearance::new(proto, state)));
        }
        BATON => {
            entity.insert((
                Baton {
                    charge: BATON_CELL,
                    on: false,
                },
                Appearance::new(BATON, "stunbaton"),
            ));
        }
        _ => {}
    }
}

pub fn is_gun(proto: &str) -> bool {
    matches!(proto, LASER_GUN | ENERGY_GUN | TASER)
}

/// Luna `gun/energy/update_icon`: its charge by quarters.
fn gun_look(proto: &str, gun: &Gun) -> String {
    let quarter = (f64::from(gun.charge) / f64::from(gun.capacity) * 4.0).round() as i32 * 25;
    match proto {
        ENERGY_GUN if gun.fires == Fires::Beam => format!("energykill{quarter}"),
        ENERGY_GUN => format!("energystun{quarter}"),
        LASER_GUN => format!("laser{quarter}"),
        _ => format!("taser{quarter}"),
    }
}

fn tell(world: &mut World, mob: Entity, msg: LocMsg) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(msg);
    }
}

fn name_of(world: &World, who: Entity) -> String {
    world
        .get::<DisplayName>(who)
        .map(|n| n.0.clone())
        .unwrap_or_default()
}

fn update_gun_look(world: &mut World, gun_entity: Entity) {
    let Some(sprite) = world.get::<Appearance>(gun_entity).map(|a| a.sprite.clone()) else {
        return;
    };
    let Some(gun) = world.get::<Gun>(gun_entity).copied() else {
        return;
    };
    if let Some(mut look) = world.get_mut::<Appearance>(gun_entity) {
        look.state = gun_look(&sprite, &gun);
    }
}

/// A click on the gun in hand (Luna `energy/gun/attack_self`): stun or kill.
pub fn switch_gun(world: &mut World, mob: Entity, gun_entity: Entity) {
    let Some(mut gun) = world.get_mut::<Gun>(gun_entity) else {
        return;
    };
    if !gun.switchable {
        return;
    }
    gun.fires = match gun.fires {
        Fires::Beam => Fires::Electrode,
        Fires::Electrode => Fires::Beam,
    };
    let id = match gun.fires {
        Fires::Beam => "gun-set-kill",
        Fires::Electrode => "gun-set-stun",
    };
    update_gun_look(world, gun_entity);
    tell(world, mob, LocMsg::new(id));
}

/// Luna `gun/afterattack`: fire at a place out of reach.
pub fn fire(world: &mut World, mob: Entity, gun_entity: Entity, to: TilePos) {
    let now = world.resource::<WorldTime>().deciseconds() as u32;
    let (Some(at), Some(gun)) = (
        world.get::<GridPos>(mob).map(|p| p.0),
        world.get::<Gun>(gun_entity).copied(),
    ) else {
        return;
    };
    if to.z != at.z || gun.last_shot_ds.is_some_and(|t| now < t + SHOT_REST_DS as u32) {
        return;
    }
    if gun.charge < SHOT_COST {
        tell(world, mob, LocMsg::new("gun-click"));
        return;
    }
    if let Some(mut gun) = world.get_mut::<Gun>(gun_entity) {
        gun.charge -= SHOT_COST;
        gun.last_shot_ds = Some(now);
    }
    update_gun_look(world, gun_entity);
    let sound = match gun.fires {
        Fires::Beam => crate::sound::LASER,
        Fires::Electrode => crate::sound::TASER,
    };
    world.resource_mut::<SoundEvents>().play(sound, at, 50.0);
    let item = world
        .get::<Appearance>(gun_entity)
        .map(|a| a.sprite.clone())
        .unwrap_or_default();
    let msg = LocMsg::new("gun-fires")
        .arg("name", name_of(world, mob))
        .arg("item", item);
    world.resource_mut::<VisibleEvents>().0.push((at, msg, None));
    launch(world, mob, at, to, gun.fires);
}

/// Send a shot from `at` through `to`, on along the same line to the end of its reach.
pub fn launch(world: &mut World, firer: Entity, at: TilePos, to: TilePos, fires: Fires) {
    let (dx, dy) = (to.x - at.x, to.y - at.y);
    let longest = dx.abs().max(dy.abs()).max(1);
    let far = TilePos::new(
        at.x + dx * SHOT_RANGE / longest,
        at.y + dy * SHOT_RANGE / longest,
        at.z,
    );
    let path: Vec<TilePos> = crate::speech::line(at, far).skip(1).collect();
    let id = world.resource_mut::<crate::objects::NetIds>().alloc();
    let look = match fires {
        Fires::Beam => "laser",
        Fires::Electrode => "spark",
    };
    world.spawn((
        GridPos(at),
        id,
        Appearance::new(SHOT, look),
        Shot {
            firer,
            fires,
            path,
            next: 0,
            aimed_at: to,
        },
    ));
}

/// What stops a shot on this tile: a wall, or anything dense but a table or rack (Luna
/// `PASSTABLE`). Glass panes and window doors stop what isn't a beam (`PASSGLASS`).
fn blocked(world: &mut World, decks: &[TilePosDeck], from: TilePos, at: TilePos, beam: bool) -> bool {
    let solid = decks
        .iter()
        .find(|d| d.0 == at.z)
        .is_none_or(|d| d.1.is_solid(at));
    if solid {
        return true;
    }
    let mut things = world.query_filtered::<(
        &GridPos,
        Has<crate::furniture::Table>,
        Has<crate::furniture::Rack>,
        Has<Body>,
    ), With<Dense>>();
    if things
        .iter(world)
        .any(|(p, table, rack, body)| p.0 == at && !table && !rack && !body)
    {
        return true;
    }
    if beam {
        return false;
    }
    let edge = (from, at);
    world.resource::<crate::windows::PaneEdges>().0.contains(&edge)
        || world
            .resource::<crate::windoors::WindoorEdges>()
            .0
            .contains(&edge)
}

/// A deck's height and map, for looking tiles up without borrowing the world.
type TilePosDeck = (i32, rs13_world::TileMap);

/// Who a shot arriving on this tile hits: someone standing, or anyone at all where it was aimed.
fn victim(world: &mut World, shot: &Shot, at: TilePos) -> Option<Entity> {
    let aimed = at == shot.aimed_at;
    let mut people =
        world.query_filtered::<(Entity, &GridPos, Option<&Vitals>, Has<Weakened>), (With<Body>, Without<InCloset>)>();
    people
        .iter(world)
        .filter(|(e, p, ..)| p.0 == at && *e != shot.firer)
        .find(|(_, _, vitals, down)| aimed || (!down && vitals.is_none_or(|v| v.stat == MobStat::Conscious)))
        .map(|(e, ..)| e)
}

fn hit(world: &mut World, shot: &Shot, person: Entity, at: TilePos) {
    let name = name_of(world, person);
    match shot.fires {
        Fires::Beam => {
            let zone = {
                let mut rng = world.resource_mut::<GameRng>();
                ran_zone(Zone::Chest, &mut rng)
            };
            if let Some(mut body) = world.get_mut::<Body>(person) {
                body.take_damage(zone, 0.0, BEAM_BURN);
            }
            let msg = LocMsg::new("shot-hit-laser").arg("name", name);
            world.resource_mut::<VisibleEvents>().0.push((at, msg, None));
        }
        Fires::Electrode => {
            let until = world.resource::<WorldTime>().deciseconds() + ELECTRODE_WEAKEN_DS;
            world.entity_mut(person).insert(Weakened(until));
            let msg = LocMsg::new("shot-hit-electrode").arg("name", name);
            world.resource_mut::<VisibleEvents>().0.push((at, msg, None));
        }
    }
}

/// Move every shot on, and end those that hit something or run out of reach.
pub fn run_shots(world: &mut World) {
    let tick = world.resource::<WorldTime>().tick.0;
    if !tick.is_multiple_of(TICKS_PER_TILE) {
        return;
    }
    let mut shots = world.query::<(Entity, &Shot)>();
    let flying: Vec<(Entity, Shot)> = shots.iter(world).map(|(e, s)| (e, s.clone())).collect();
    if flying.is_empty() {
        return;
    }
    let mut maps = world.query::<&MapLayer>();
    let decks: Vec<TilePosDeck> = maps.iter(world).map(|m| (m.0.z, m.0.clone())).collect();
    for (entity, shot) in flying {
        let Some(&to) = shot.path.get(shot.next) else {
            world.entity_mut(entity).despawn();
            continue;
        };
        if let Some(person) = victim(world, &shot, to) {
            hit(world, &shot, person, to);
            world.entity_mut(entity).despawn();
            continue;
        }
        let from = world.get::<GridPos>(entity).map_or(to, |p| p.0);
        if blocked(world, &decks, from, to, shot.fires == Fires::Beam) {
            world.entity_mut(entity).despawn();
            continue;
        }
        let mut live = world.entity_mut(entity);
        if let Some(mut pos) = live.get_mut::<GridPos>() {
            pos.0 = to;
        }
        if let Some(mut s) = live.get_mut::<Shot>() {
            s.next += 1;
        }
    }
}

/// A click on the baton in hand (Luna `baton/attack_self`): on or off, if it has the charge.
pub fn toggle_baton(world: &mut World, mob: Entity, baton: Entity) {
    let Some(mut state) = world.get_mut::<Baton>(baton) else {
        return;
    };
    let id = if state.charge > BATON_HIT {
        state.on = !state.on;
        if state.on { "baton-on" } else { "baton-off" }
    } else {
        state.on = false;
        "baton-empty"
    };
    let on = state.on;
    if let Some(mut look) = world.get_mut::<Appearance>(baton) {
        look.state = if on { "stunbaton_active" } else { "stunbaton" }.to_owned();
    }
    tell(world, mob, LocMsg::new(id));
}

/// Luna `baton/attack`: on, it knocks down whoever it touches; off, it only prods them, unless
/// you mean to hit them (which the usual blow does on harm intent).
pub fn baton_hit(world: &mut World, mob: Entity, baton: Entity, target: Entity) -> bool {
    let Some(state) = world.get::<Baton>(baton).copied() else {
        return false;
    };
    let Some(at) = world.get::<GridPos>(target).map(|p| p.0) else {
        return false;
    };
    let (user, victim) = (name_of(world, mob), name_of(world, target));
    if !state.on {
        let msg = LocMsg::new("baton-prod").arg("name", user).arg("victim", victim);
        world.resource_mut::<VisibleEvents>().0.push((at, msg, None));
        return false;
    }
    let until = world.resource::<WorldTime>().deciseconds() + BATON_WEAKEN_DS;
    world.entity_mut(target).insert(Weakened(until));
    let msg = LocMsg::new("baton-stun").arg("name", user).arg("victim", victim);
    world.resource_mut::<VisibleEvents>().0.push((at, msg, None));
    world
        .resource_mut::<SoundEvents>()
        .play(crate::sound::BATON_HIT_SOUND, at, 50.0);
    let empty = {
        let Some(mut s) = world.get_mut::<Baton>(baton) else {
            return true;
        };
        s.charge = (s.charge - BATON_HIT).max(0);
        if s.charge < BATON_HIT {
            s.on = false;
            true
        } else {
            false
        }
    };
    if empty && let Some(mut look) = world.get_mut::<Appearance>(baton) {
        look.state = "stunbaton".to_owned();
    }
    true
}

pub fn spawn_recharger(world: &mut World, pos: TilePos, id: NetId) -> Entity {
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(RECHARGER, "recharger0"),
            Recharger::default(),
            PowerUser::new(Channel::Equipment, RECHARGER_IDLE_WATTS),
        ))
        .id()
}

/// A click on a recharger (Luna `attackby`, `attack_hand`): a gun or baton goes in if the room
/// has power; an empty hand takes out what is there.
pub fn click_recharger(world: &mut World, mob: Entity, recharger: Entity) {
    let Some(state) = world.get::<Recharger>(recharger).copied() else {
        return;
    };
    let held = world.get::<Hands>(mob).and_then(Hands::active_item);
    match (held, state.charging) {
        (Some(item), None) => {
            let fits = world.get::<Gun>(item).is_some() || world.get::<Baton>(item).is_some();
            if !fits {
                return;
            }
            if !world.get::<PowerUser>(recharger).is_some_and(|p| p.powered) {
                tell(world, mob, LocMsg::new("recharger-no-power"));
                return;
            }
            if let Some(mut hands) = world.get_mut::<Hands>(mob) {
                let active = hands.active;
                *hands.slot_mut(active) = None;
            }
            world
                .entity_mut(item)
                .remove::<HeldBy>()
                .insert(InCloset(recharger));
            if let Some(mut r) = world.get_mut::<Recharger>(recharger) {
                r.charging = Some(item);
            }
        }
        (None, Some(item)) => {
            if let Some(mut hands) = world.get_mut::<Hands>(mob) {
                let active = hands.active;
                *hands.slot_mut(active) = Some(item);
            }
            world.entity_mut(item).remove::<InCloset>().insert(HeldBy(mob));
            if let Some(mut r) = world.get_mut::<Recharger>(recharger) {
                r.charging = None;
            }
            if let Some(mut look) = world.get_mut::<Appearance>(recharger) {
                look.state = "recharger0".to_owned();
            }
            if let Some(mut power) = world.get_mut::<PowerUser>(recharger) {
                power.watts = RECHARGER_IDLE_WATTS;
            }
        }
        _ => {}
    }
}

/// Every 2 s rechargers top up what is in them (Luna `recharger/process`).
pub fn run_rechargers(world: &mut World) {
    if !world.resource::<WorldTime>().tick.0.is_multiple_of(MACHINE_TICKS) {
        return;
    }
    let mut rechargers = world.query::<(Entity, &Recharger, &PowerUser)>();
    let busy: Vec<(Entity, Entity)> = rechargers
        .iter(world)
        .filter(|(_, _, p)| p.powered)
        .filter_map(|(e, r, _)| r.charging.map(|c| (e, c)))
        .collect();
    for (recharger, item) in busy {
        let mut filling = false;
        if let Some(mut gun) = world.get_mut::<Gun>(item) {
            if gun.charge < gun.capacity {
                gun.charge = (gun.charge + RECHARGE_GUN).min(gun.capacity);
                filling = true;
            }
            update_gun_look(world, item);
        } else if let Some(mut baton) = world.get_mut::<Baton>(item)
            && baton.charge < BATON_CELL
        {
            baton.charge = (baton.charge + RECHARGE_BATON).min(BATON_CELL);
            filling = true;
        }
        if let Some(mut look) = world.get_mut::<Appearance>(recharger) {
            let state = if filling { "recharger1" } else { "recharger2" };
            if look.state != state {
                look.state = state.to_owned();
            }
        }
        if let Some(mut power) = world.get_mut::<PowerUser>(recharger) {
            power.watts = if filling {
                RECHARGE_WATTS
            } else {
                RECHARGER_IDLE_WATTS
            };
        }
    }
}
