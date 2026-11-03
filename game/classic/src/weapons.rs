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
pub const PISTOL: &str = "pistol";
pub const REVOLVER: &str = "revolver";
pub const COMBAT_SHOTGUN: &str = "combat-shotgun";
pub const DOUBLE_BARREL: &str = "double-barrel";
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
    Bullet(Round),
}

/// What a bullet gun has loaded (Luna's `projectile/bullet` subtypes, by casing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Round {
    /// `.12` slugs.
    Slug,
    /// `.12` beanbags and `.38`.
    Weak,
    /// `12mm`.
    Mid,
    /// `9mm`.
    Nine,
}

impl Round {
    /// Brute damage, and how long it knocks someone down, ds (Luna `damage`, `weaken`).
    pub fn effect(self) -> (f32, f32) {
        match self {
            Round::Slug => (60.0, 0.0),
            Round::Weak => (10.0, 200.0),
            Round::Mid => (25.0, 60.0),
            Round::Nine => (30.0, 0.0),
        }
    }
}

/// A magazine, speed loader or box: what's in it, and what takes it.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ammo {
    pub caliber: &'static str,
    pub round: Round,
    pub rounds: i32,
}

/// A bullet gun's caliber.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Caliber(pub &'static str);

/// What a prototype of ammunition holds when new.
fn ammo(proto: &str) -> Option<Ammo> {
    let (caliber, round, rounds) = match proto {
        "magazine-9mm" => ("9mm", Round::Nine, 8),
        "magazine-12mm" => ("12mm", Round::Mid, 20),
        "speed-loader-38" => (".38", Round::Weak, 6),
        "slug-box" => (".12", Round::Slug, 8),
        "beanbag-box" => (".12", Round::Weak, 8),
        _ => return None,
    };
    Some(Ammo {
        caliber,
        round,
        rounds,
    })
}

pub fn is_ammo(proto: &str) -> bool {
    ammo(proto).is_some()
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
    if let Some(ammo) = ammo(proto) {
        entity.insert(ammo);
        return;
    }
    match proto {
        // Bullet guns come loaded (Luna `mag_type`).
        PISTOL | REVOLVER | COMBAT_SHOTGUN | DOUBLE_BARREL => {
            let (caliber, round, rounds, look) = match proto {
                PISTOL => ("9mm", Round::Nine, 8, "pistol"),
                REVOLVER => (".38", Round::Weak, 6, "detective"),
                COMBAT_SHOTGUN => (".12", Round::Slug, 8, "cshotgun"),
                _ => (".12", Round::Weak, 2, "dshotgun"),
            };
            entity.insert((
                Gun {
                    charge: rounds * SHOT_COST,
                    capacity: rounds * SHOT_COST,
                    fires: Fires::Bullet(round),
                    switchable: false,
                    last_shot_ds: None,
                },
                Caliber(caliber),
                Appearance::new(proto, look),
            ));
        }
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
    matches!(
        proto,
        LASER_GUN | ENERGY_GUN | TASER | PISTOL | REVOLVER | COMBAT_SHOTGUN | DOUBLE_BARREL
    )
}

/// Luna `gun/energy/update_icon`: its charge by quarters.
fn gun_look(proto: &str, gun: &Gun) -> String {
    // Bullet guns look the same however many rounds they have.
    match proto {
        PISTOL => return "pistol".to_owned(),
        REVOLVER => return "detective".to_owned(),
        COMBAT_SHOTGUN => return "cshotgun".to_owned(),
        DOUBLE_BARREL => return "dshotgun".to_owned(),
        _ => {}
    }
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
        bullet @ Fires::Bullet(_) => bullet,
    };
    let id = match gun.fires {
        Fires::Bullet(_) => return,
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
        Fires::Bullet(_) => crate::sound::GUNSHOT,
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

/// A click with ammunition on a gun: as many rounds go in as fit (Luna `gun/projectile/attackby`).
pub fn load(world: &mut World, mob: Entity, gun_entity: Entity) {
    let Some(item) = world.get::<Hands>(mob).and_then(Hands::active_item) else {
        return;
    };
    let (Some(ammo), Some(gun), Some(caliber)) = (
        world.get::<Ammo>(item).copied(),
        world.get::<Gun>(gun_entity).copied(),
        world.get::<Caliber>(gun_entity).copied(),
    ) else {
        return;
    };
    if ammo.caliber != caliber.0 {
        tell(world, mob, LocMsg::new("gun-wrong-ammo"));
        return;
    }
    // One kind at a time: this keeps a count, not a row of casings.
    if gun.charge >= SHOT_COST && gun.fires != Fires::Bullet(ammo.round) {
        tell(world, mob, LocMsg::new("gun-other-ammo"));
        return;
    }
    let moved = ammo.rounds.min((gun.capacity - gun.charge) / SHOT_COST);
    if moved <= 0 {
        tell(
            world,
            mob,
            LocMsg::new(if ammo.rounds <= 0 {
                "ammo-empty"
            } else {
                "gun-full"
            }),
        );
        return;
    }
    if let Some(mut gun) = world.get_mut::<Gun>(gun_entity) {
        gun.charge += moved * SHOT_COST;
        gun.fires = Fires::Bullet(ammo.round);
    }
    if let Some(mut ammo) = world.get_mut::<Ammo>(item) {
        ammo.rounds -= moved;
    }
    tell(
        world,
        mob,
        LocMsg::new("gun-loaded").num("value", i64::from(moved)),
    );
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
        Fires::Bullet(_) => "bullet",
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
        Fires::Bullet(round) => {
            let (brute, weaken) = round.effect();
            let zone = {
                let mut rng = world.resource_mut::<GameRng>();
                ran_zone(Zone::Chest, &mut rng)
            };
            if let Some(mut body) = world.get_mut::<Body>(person) {
                body.take_damage(zone, brute, 0.0);
            }
            if weaken > 0.0 {
                let until = world.resource::<WorldTime>().deciseconds() + weaken;
                world.entity_mut(person).insert(Weakened(until));
            }
            let msg = LocMsg::new("shot-hit-bullet").arg("name", name);
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
            let fits = world
                .get::<Gun>(item)
                .is_some_and(|g| !matches!(g.fires, Fires::Bullet(_)))
                || world.get::<Baton>(item).is_some();
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
