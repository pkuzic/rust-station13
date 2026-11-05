//! The blob: a thing that grows over the station until it's stopped or the station's written off
//! (Luna's `gamemodes/blob`). Spec: `docs/reference/meteor-blob.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, Dense, GridPos, MapLayer, TilePos};

use crate::actions::Outbox;
use crate::explosions::strike;
use crate::hands::{Hands, Item};
use crate::objects::NetIds;
use crate::rng::GameRng;
use crate::round::{Phase, Round};
use crate::sound::SoundEvents;
use crate::time::WorldTime;
use crate::welding::Welder;

pub const MODE: &str = "blob";
pub const BLOB: &str = "blob";
/// How much health a blob starts with (Luna's `New`).
pub const START_HEALTH: f32 = 30.0;
/// How often it grows, ds, and how many of it are looked at each time.
const LIFE_DS: f32 = 20.0;
const CELLS_EACH_LIFE: usize = 25;
/// How many it takes to have the station written off (Luna `stage`).
pub const OUT_OF_CONTROL: usize = 500;
/// What a lit welding tool does to it.
const WELDER_DAMAGE: f32 = 15.0;
/// Chance in percent a meteor falls, at each life (Luna's `process`).
const METEOR_CHANCE: f32 = 2.0;
const MINUTE_DS: f32 = 600.0;

/// One cell of it.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct BlobCell {
    pub health: f32,
}

/// How it stands.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq)]
pub struct BlobState {
    /// There's been one here.
    pub started: bool,
    pub stage: u8,
    pub next_stage: f32,
    pub next_life: f32,
    /// Over: whether the staff won.
    pub staff_won: Option<bool>,
}

fn state_for(health: f32) -> &'static str {
    if health < 10.0 {
        "c"
    } else if health < 20.0 {
        "b"
    } else {
        "a"
    }
}

pub fn spawn_blob(world: &mut World, at: TilePos, health: f32) -> Entity {
    let id = world.resource_mut::<NetIds>().alloc();
    world
        .spawn((
            GridPos(at),
            id,
            Appearance::new(BLOB, state_for(health)),
            Dense,
            BlobCell { health },
        ))
        .id()
}

/// Hurt a cell: gone at no health.
pub fn damage(world: &mut World, cell: Entity, amount: f32) {
    let Some(mut blob) = world.get_mut::<BlobCell>(cell) else {
        return;
    };
    blob.health -= amount;
    let health = blob.health;
    if health <= 0.0 {
        world.entity_mut(cell).despawn();
    } else if let Some(mut look) = world.get_mut::<Appearance>(cell) {
        look.state = state_for(health).to_owned();
    }
}

/// Hit with what's in the hand: a quarter of its force, or 15 with a lit welding tool (Luna's `attackby`).
pub fn attacked(world: &mut World, mob: Entity, cell: Entity) {
    let Some(item) = world.get::<Hands>(mob).and_then(Hands::active_item) else {
        return;
    };
    let lit = world.get::<Welder>(item).is_some_and(|w| w.lit);
    let force = world.get::<Item>(item).map_or(0.0, |i| i.force);
    let amount = if lit { WELDER_DAMAGE } else { force / 4.0 };
    if let Some(at) = world.get::<GridPos>(cell).map(|p| p.0) {
        world
            .resource_mut::<SoundEvents>()
            .play(crate::sound::BLOB_ATTACKED, at, 50.0);
    }
    damage(world, cell, amount);
}

/// What an explosion does to a cell (Luna's `ex_act`).
pub fn blasted(world: &mut World, cell: Entity, severity: u8) {
    let amount = match severity {
        1 => f32::MAX,
        2 => world.resource_mut::<GameRng>().range(20, 30) as f32,
        _ => world.resource_mut::<GameRng>().range(15, 25) as f32,
    };
    damage(world, cell, amount);
}

fn tell_everyone(world: &mut World, msg: LocMsg) {
    let mut people = world.query::<&mut Outbox>();
    for mut outbox in people.iter_mut(world) {
        outbox.0.push(msg.clone());
    }
}

fn is_blob_round(world: &World) -> bool {
    world
        .get_resource::<Round>()
        .is_some_and(|round| matches!(round.phase, Phase::Playing { mode: MODE }))
}

/// Start it, on a random floor of the station (Luna's map has no `blobstart`, so its own would fail here).
pub fn setup_round(world: &mut World) -> bool {
    let mut maps = world.query::<&MapLayer>();
    let mut floors: Vec<TilePos> = Vec::new();
    for layer in maps.iter(world) {
        for y in 0..layer.0.height as i32 {
            for x in 0..layer.0.width as i32 {
                let at = TilePos::new(x, y, layer.0.z);
                if layer.0.tile(at).is_some_and(|t| !t.solid && t.id != "space") {
                    floors.push(at);
                }
            }
        }
    }
    if floors.is_empty() {
        return false;
    }
    let mut dense = world.query_filtered::<&GridPos, With<Dense>>();
    let taken: Vec<TilePos> = dense.iter(world).map(|p| p.0).collect();
    floors.retain(|at| !taken.contains(at));
    if floors.is_empty() {
        return false;
    }
    let mut rng = world.resource_mut::<GameRng>();
    let at = floors[rng.range(0, floors.len() as i32 - 1) as usize];
    let wait = rng.range(200, 900) as f32;
    spawn_blob(world, at, START_HEALTH);
    let now = world.resource::<WorldTime>().deciseconds();
    world.insert_resource(BlobState {
        started: true,
        stage: 0,
        next_stage: now + wait,
        next_life: now,
        staff_won: None,
    });
    true
}

/// A cell's life: if it's lucky, spreading into every open tile beside it, and at what's in the way.
fn life(world: &mut World, cell: Entity, at: TilePos, taken: &mut Vec<TilePos>, blobs: &mut Vec<TilePos>) {
    let health = world.get::<BlobCell>(cell).map_or(0.0, |b| b.health);
    let on_floor = tile_at(world, at).is_some_and(|t| t.id != "space");
    let chance = health + if on_floor { 3.0 } else { 0.0 };
    if !world.resource_mut::<GameRng>().prob(chance) {
        return;
    }
    for (dx, dy) in [(0, 1), (0, -1), (1, 0), (-1, 0)] {
        let to = TilePos::new(at.x + dx, at.y + dy, at.z);
        let Some(tile) = tile_at(world, to) else {
            continue;
        };
        if area_at(world, to).is_some_and(|area| area.starts_with("arrival")) {
            continue;
        }
        if !tile.solid && !taken.contains(&to) {
            spawn_blob(world, to, health);
            taken.push(to);
            blobs.push(to);
        } else if !blobs.contains(&to) && world.resource_mut::<GameRng>().prob(50.0) {
            // Luna's `blob_act`: as a light blast, only here.
            strike(world, to, 3);
        }
    }
}

fn tile_at(world: &mut World, at: TilePos) -> Option<rs13_world::TileDef> {
    let mut maps = world.query::<&MapLayer>();
    maps.iter(world)
        .find(|m| m.0.z == at.z)
        .and_then(|m| m.0.tile(at).cloned())
}

fn area_at(world: &mut World, at: TilePos) -> Option<String> {
    let mut maps = world.query::<&MapLayer>();
    maps.iter(world)
        .find(|m| m.0.z == at.z)
        .and_then(|m| m.0.area(at).map(str::to_owned))
}

/// Growth, the alerts as it gets worse, and the end of it.
pub fn run_blobs(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    let state = *world.resource::<BlobState>();
    let mut cells = world.query_filtered::<(Entity, &GridPos), With<BlobCell>>();
    let all: Vec<(Entity, TilePos)> = cells.iter(world).map(|(e, p)| (e, p.0)).collect();
    if now >= state.next_life && !all.is_empty() {
        world.resource_mut::<BlobState>().next_life = now + LIFE_DS;
        let mut dense = world.query_filtered::<&GridPos, With<Dense>>();
        let mut taken: Vec<TilePos> = dense.iter(world).map(|p| p.0).collect();
        let mut blobs: Vec<TilePos> = all.iter().map(|(_, p)| *p).collect();
        for _ in 0..CELLS_EACH_LIFE {
            let i = world.resource_mut::<GameRng>().range(0, all.len() as i32 - 1) as usize;
            let (cell, at) = all[i];
            if world.get_entity(cell).is_ok() {
                life(world, cell, at, &mut taken, &mut blobs);
            }
        }
        if is_blob_round(world) && world.resource_mut::<GameRng>().prob(METEOR_CHANCE) {
            crate::meteor::spawn_meteor(world);
        }
    }
    if !state.started || state.staff_won.is_some() || !is_blob_round(world) {
        return;
    }
    if all.is_empty() {
        world.resource_mut::<BlobState>().staff_won = Some(true);
        tell_everyone(world, LocMsg::new("blob-staff-won"));
        return;
    }
    if now < state.next_stage {
        return;
    }
    match state.stage {
        0 => {
            tell_everyone(world, LocMsg::new("blob-alert"));
            let wait = world.resource_mut::<GameRng>().range(5, 10) as f32 * MINUTE_DS;
            let mut next = world.resource_mut::<BlobState>();
            next.stage = 1;
            next.next_stage = now + wait;
        }
        1 => {
            tell_everyone(world, LocMsg::new("blob-confirmed"));
            let mut next = world.resource_mut::<BlobState>();
            next.stage = 2;
            next.next_stage = now + MINUTE_DS;
        }
        2 => {
            let out_of_control = all.len() > OUT_OF_CONTROL;
            if out_of_control {
                tell_everyone(world, LocMsg::new("blob-directive"));
            }
            let mut next = world.resource_mut::<BlobState>();
            next.next_stage = now + MINUTE_DS;
            if out_of_control {
                next.stage = 3;
            }
        }
        _ => {
            let mut next = world.resource_mut::<BlobState>();
            next.stage = 4;
            next.staff_won = Some(false);
            tell_everyone(world, LocMsg::new("blob-staff-lost"));
        }
    }
}

/// Whether the round's over.
pub fn over(world: &World) -> bool {
    world
        .get_resource::<BlobState>()
        .is_some_and(|state| state.staff_won.is_some())
}
