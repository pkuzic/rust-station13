//! Morgue drawers and crematoriums: a tray slides out, takes what lies on it back in, and
//! holds it out of sight; a crematorium also burns whoever is inside to ashes. Spec:
//! `docs/reference/morgue.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, Dense, Dir, GridPos, Hidden, MapLayer, MobStat, NetId, ObjectSpec, TilePos};

use crate::closets::InCloset;
use crate::combat::VisibleEvents;
use crate::equipment::{Equipment, WornBy};
use crate::hands::{Hands, HeldBy, Item};
use crate::health::{Body, Vitals};
use crate::movement::Mob;
use crate::rng::GameRng;
use crate::sound::SoundEvents;
use crate::storage::InStorage;
use crate::time::WorldTime;

pub const MORGUE: &str = "morgue";
pub const MORGUE_TRAY: &str = "morgue-tray";
/// Where the tray slides out (Luna `step(connected, EAST)`).
pub const TRAY_DIR: Dir = Dir::East;
pub const CREMATORIUM: &str = "crematorium";
pub const CREMATORIUM_TRAY: &str = "crematorium-tray";
pub const IGNITER: &str = "crematorium-igniter";
pub const URN: &str = "urn";
/// Where a crematorium's tray slides out (Luna `step(connected, SOUTH)`).
pub const CREMATORIUM_TRAY_DIR: Dir = Dir::South;
/// The access an igniter asks for (Luna `access_crematorium`).
pub const IGNITER_ACCESS: &str = "crematorium";
/// How long a cremation takes, in seconds of burning (Luna's ten `sleep(10)`, and
/// `spawn(100)` before it opens again).
pub const CREMATION_BURNS: u32 = 10;
/// Burn damage each second of it (Luna `fireloss += 30`).
pub const BURN_PER_SECOND: f32 = 30.0;
/// Of each thing someone carried, the chance in percent it is left with their ashes.
pub const KEPT_CHANCE: f32 = 10.0;

/// A morgue drawer: its tray while out, and what lies inside.
#[derive(Component, Debug, Clone, PartialEq, Eq, Default)]
pub struct Morgue {
    pub tray: Option<Entity>,
    pub contents: Vec<Entity>,
}

/// A morgue drawer's tray, pulled out.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MorgueTray(pub Entity);

/// A crematorium, on top of its `Morgue` drawer: the igniters with its id light it.
#[derive(Component, Debug, Clone, PartialEq, Default)]
pub struct Crematorium {
    pub id: String,
    /// While burning: when it was lit, ds. It stays shut meanwhile (Luna `locked`).
    pub lit_at: Option<f32>,
    /// Seconds of burning done so far.
    pub burns: u32,
}

/// A crematorium igniter on the wall, with the id of the crematoriums it lights.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct Igniter(pub String);

fn var(spec: &ObjectSpec, name: &str) -> Option<String> {
    spec.vars.get(name).map(|v| v.trim().trim_matches('"').to_owned())
}

pub fn spawn_morgue(world: &mut World, pos: TilePos, id: NetId) -> Entity {
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(MORGUE, "morgue1"),
            Dense,
            Morgue::default(),
        ))
        .id()
}

pub fn spawn_crematorium(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(CREMATORIUM, "crema1"),
            Dense,
            Morgue::default(),
            Crematorium {
                id: var(spec, "id").unwrap_or_default(),
                ..Crematorium::default()
            },
        ))
        .id()
}

pub fn spawn_igniter(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(IGNITER, "crema_switch"),
            Igniter(var(spec, "id").unwrap_or_default()),
        ))
        .id()
}

/// Whether this drawer is a crematorium burning, and so shut (Luna `locked`).
pub fn is_locked(world: &World, drawer: Entity) -> bool {
    world
        .get::<Crematorium>(drawer)
        .is_some_and(|c| c.lit_at.is_some())
}

/// Luna `morgue/attack_hand` (and `relaymove` from inside): pull the tray out (east, or
/// south for a crematorium), if nothing dense is in the way, and lay out what was inside on
/// it. Luna's sound plays either way. A burning crematorium stays shut.
pub fn open_tray(world: &mut World, morgue: Entity) {
    let Some(at) = world.get::<GridPos>(morgue).map(|p| p.0) else {
        return;
    };
    let Some(drawer) = world.get::<Morgue>(morgue) else {
        return;
    };
    if drawer.tray.is_some() || is_locked(world, morgue) {
        return;
    }
    let crematorium = world.get::<Crematorium>(morgue).is_some();
    world
        .resource_mut::<SoundEvents>()
        .play(crate::sound::DECONSTRUCT, at, 50.0);
    let out = at.step(if crematorium {
        CREMATORIUM_TRAY_DIR
    } else {
        TRAY_DIR
    });
    let mut decks = world.query::<&MapLayer>();
    let solid = decks
        .iter(world)
        .find(|m| m.0.z == out.z)
        .is_none_or(|m| m.0.is_solid(out));
    let mut dense = world.query_filtered::<&GridPos, With<Dense>>();
    if solid || dense.iter(world).any(|p| p.0 == out) {
        return;
    }
    let id = world.resource_mut::<crate::objects::NetIds>().alloc();
    let look = if crematorium {
        Appearance::new(CREMATORIUM_TRAY, "cremat")
    } else {
        Appearance::new(MORGUE_TRAY, "morguet")
    };
    let tray = world
        .spawn((GridPos(out), id, look, Dense, MorgueTray(morgue)))
        .id();
    let contents = {
        let Some(mut drawer) = world.get_mut::<Morgue>(morgue) else {
            return;
        };
        drawer.tray = Some(tray);
        std::mem::take(&mut drawer.contents)
    };
    for thing in contents {
        let Ok(mut e) = world.get_entity_mut(thing) else {
            continue;
        };
        e.remove::<(InCloset, Hidden)>().insert(GridPos(out));
    }
}

/// Luna `morgue/attack_hand` with the tray out, or `m_tray/attack_hand`: everything loose
/// on the tray goes in and the tray slides back.
pub fn close_tray(world: &mut World, morgue: Entity) {
    let Some(at) = world.get::<GridPos>(morgue).map(|p| p.0) else {
        return;
    };
    let Some(tray) = world.get::<Morgue>(morgue).and_then(|d| d.tray) else {
        return;
    };
    let Some(out) = world.get::<GridPos>(tray).map(|p| p.0) else {
        return;
    };
    world
        .resource_mut::<SoundEvents>()
        .play(crate::sound::DECONSTRUCT, at, 50.0);
    let mut loose = world.query_filtered::<(Entity, &GridPos, Has<Mob>), Or<(With<Item>, With<Mob>)>>();
    let taken: Vec<(Entity, bool)> = loose
        .iter(world)
        .filter(|(_, p, _)| p.0 == out)
        .map(|(e, _, mob)| (e, mob))
        .collect();
    for (thing, is_mob) in &taken {
        // People keep their place on the map, hidden; things leave it.
        let mut e = world.entity_mut(*thing);
        if *is_mob {
            e.insert((InCloset(morgue), Hidden, GridPos(at)));
        } else {
            e.remove::<GridPos>().insert(InCloset(morgue));
        }
    }
    world.entity_mut(tray).despawn();
    if let Some(mut drawer) = world.get_mut::<Morgue>(morgue) {
        drawer.tray = None;
        drawer.contents.extend(taken.into_iter().map(|(e, _)| e));
    }
}

/// Someone inside trying to get out, awake: the tray comes out (Luna `morgue/relaymove`),
/// unless the crematorium is burning.
pub fn leave(world: &mut World, mob: Entity, morgue: Entity) {
    if world.get::<Morgue>(morgue).is_none() {
        return;
    }
    if world
        .get::<Vitals>(mob)
        .is_some_and(|v| v.stat != MobStat::Conscious)
    {
        return;
    }
    open_tray(world, morgue);
}

/// Luna `morgue/update` and `crematorium/update`: `morgue0` with the tray out, `morgue2`
/// with something inside, `morgue1` empty; `crema0`, `crema2` and `crema1` the same.
pub fn update_morgue_looks(
    mut drawers: Query<(&Morgue, &mut Appearance, Has<Crematorium>), Changed<Morgue>>,
) {
    for (drawer, mut look, crematorium) in &mut drawers {
        let prefix = if crematorium { "crema" } else { "morgue" };
        let state = if drawer.tray.is_some() {
            format!("{prefix}0")
        } else if drawer.contents.is_empty() {
            format!("{prefix}1")
        } else {
            format!("{prefix}2")
        };
        if look.state != state {
            look.state = state;
        }
    }
}

/// Luna `crema_switch/attack_hand` and `crematorium/cremate`: every crematorium with this
/// igniter's id that isn't burning already lights if anything is inside, and stays shut
/// until it is done. Lit empty, it only crackles. Those near the igniter hear which. The
/// caller checks access.
pub fn ignite(world: &mut World, igniter: Entity) {
    let Some(id) = world.get::<Igniter>(igniter).map(|i| i.0.clone()) else {
        return;
    };
    let Some(at) = world.get::<GridPos>(igniter).map(|p| p.0) else {
        return;
    };
    let now = world.resource::<WorldTime>().deciseconds();
    let mut heard = Vec::new();
    let mut drawers = world.query::<(&Morgue, &mut Crematorium)>();
    for (drawer, mut crematorium) in drawers.iter_mut(world) {
        if crematorium.id != id || crematorium.lit_at.is_some() {
            continue;
        }
        if drawer.contents.is_empty() {
            heard.push("crematorium-crackle");
        } else {
            crematorium.lit_at = Some(now);
            crematorium.burns = 0;
            heard.push("crematorium-roar");
        }
    }
    let mut events = world.resource_mut::<VisibleEvents>();
    for message in heard {
        events.0.push((at, LocMsg::new(message), None));
    }
}

/// While a crematorium burns, everyone inside takes burns each second. After ten seconds
/// each of them is gone, an urn left in their place (see `finish_cremation`), and it dings
/// and opens again.
pub fn run_crematoriums(
    time: Res<WorldTime>,
    mut commands: Commands,
    mut drawers: Query<(Entity, &Morgue, &mut Crematorium)>,
    mut bodies: Query<&mut Body>,
) {
    let now = time.deciseconds();
    for (entity, drawer, mut crematorium) in &mut drawers {
        let Some(lit) = crematorium.lit_at else {
            continue;
        };
        while crematorium.burns < CREMATION_BURNS && now >= lit + 10.0 * (crematorium.burns + 1) as f32 {
            crematorium.burns += 1;
            for thing in &drawer.contents {
                if let Ok(mut body) = bodies.get_mut(*thing) {
                    body.take_damage_spread(0.0, BURN_PER_SECOND);
                }
            }
        }
        if crematorium.burns >= CREMATION_BURNS {
            crematorium.lit_at = None;
            commands.queue(move |world: &mut World| finish_cremation(world, entity));
        }
    }
}

/// Everyone inside turns to an urn. Each thing they held or wore is kept beside it one time
/// in ten, else burns with them, along with what it held. Things put in without anyone stay
/// as they were (Luna burns only mobs).
pub fn finish_cremation(world: &mut World, drawer: Entity) {
    let Some(at) = world.get::<GridPos>(drawer).map(|p| p.0) else {
        return;
    };
    let contents = world
        .get::<Morgue>(drawer)
        .map(|d| d.contents.clone())
        .unwrap_or_default();
    let mut left = Vec::new();
    for thing in contents {
        if world.get::<Mob>(thing).is_none() {
            left.push(thing);
            continue;
        }
        let mut carried: Vec<Entity> = Vec::new();
        if let Some(hands) = world.get::<Hands>(thing) {
            carried.extend(hands.left.into_iter().chain(hands.right));
        }
        if let Some(worn) = world.get::<Equipment>(thing) {
            carried.extend(worn.0.values().copied());
        }
        for item in carried {
            if world.resource_mut::<GameRng>().prob(KEPT_CHANCE) {
                world
                    .entity_mut(item)
                    .remove::<(HeldBy, WornBy)>()
                    .insert(InCloset(drawer));
                left.push(item);
            } else {
                burn_up(world, item);
            }
        }
        if let Ok(urn) = crate::objects::spawn_object(world, &ObjectSpec::new(URN), at) {
            world.entity_mut(urn).remove::<GridPos>().insert(InCloset(drawer));
            left.push(urn);
        }
        world.entity_mut(thing).despawn();
    }
    if let Some(mut d) = world.get_mut::<Morgue>(drawer) {
        d.contents = left;
    }
    world
        .resource_mut::<SoundEvents>()
        .play(crate::sound::DING, at, 50.0);
}

/// Despawn an item and everything stored in it.
fn burn_up(world: &mut World, item: Entity) {
    let mut stored = world.query::<(Entity, &InStorage)>();
    let inside: Vec<Entity> = stored
        .iter(world)
        .filter(|(_, s)| s.0 == item)
        .map(|(e, _)| e)
        .collect();
    for thing in inside {
        burn_up(world, thing);
    }
    if let Ok(e) = world.get_entity_mut(item) {
        e.despawn();
    }
}
