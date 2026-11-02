//! Morgue drawers: a tray slides out to the east, takes what lies on it back in, and holds
//! it out of sight. Spec: `docs/reference/morgue.md`.

use bevy_ecs::prelude::*;
use rs13_world::{Appearance, Dense, Dir, GridPos, Hidden, MapLayer, MobStat, NetId, TilePos};

use crate::closets::InCloset;
use crate::hands::Item;
use crate::health::Vitals;
use crate::movement::Mob;
use crate::sound::SoundEvents;

pub const MORGUE: &str = "morgue";
pub const MORGUE_TRAY: &str = "morgue-tray";
/// Where the tray slides out (Luna `step(connected, EAST)`).
pub const TRAY_DIR: Dir = Dir::East;

/// A morgue drawer: its tray while out, and what lies inside.
#[derive(Component, Debug, Clone, PartialEq, Eq, Default)]
pub struct Morgue {
    pub tray: Option<Entity>,
    pub contents: Vec<Entity>,
}

/// A morgue drawer's tray, pulled out.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MorgueTray(pub Entity);

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

/// Luna `morgue/attack_hand` (and `relaymove` from inside): pull the tray out east, if
/// nothing dense is in the way, and lay out what was inside on it. Luna's sound plays
/// either way.
pub fn open_tray(world: &mut World, morgue: Entity) {
    let Some(at) = world.get::<GridPos>(morgue).map(|p| p.0) else {
        return;
    };
    let Some(drawer) = world.get::<Morgue>(morgue) else {
        return;
    };
    if drawer.tray.is_some() {
        return;
    }
    world
        .resource_mut::<SoundEvents>()
        .play(crate::sound::DECONSTRUCT, at, 50.0);
    let out = at.step(TRAY_DIR);
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
    let tray = world
        .spawn((
            GridPos(out),
            id,
            Appearance::new(MORGUE_TRAY, "morguet"),
            Dense,
            MorgueTray(morgue),
        ))
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

/// Someone inside trying to get out, awake: the tray comes out (Luna `morgue/relaymove`).
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

/// Luna `morgue/update`: `morgue0` with the tray out, `morgue2` with something inside,
/// `morgue1` empty.
pub fn update_morgue_looks(mut drawers: Query<(&Morgue, &mut Appearance), Changed<Morgue>>) {
    for (drawer, mut look) in &mut drawers {
        let state = if drawer.tray.is_some() {
            "morgue0"
        } else if drawer.contents.is_empty() {
            "morgue1"
        } else {
            "morgue2"
        };
        if look.state != state {
            look.state = state.to_owned();
        }
    }
}
