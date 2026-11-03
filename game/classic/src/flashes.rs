//! Flashes (Luna's `device/flash`) and mounted and portable flashers (`machinery/flasher`):
//! a burst of light that knocks people down unless their eyes are covered. Spec:
//! `docs/reference/flashes.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, Dense, DisplayName, GridPos, NetId, ObjectSpec, Opaque, Slot, TilePos};

use crate::actions::Outbox;
use crate::combat::VisibleEvents;
use crate::equipment::Equipment;
use crate::health::{Body, Vitals};
use crate::movement::{Mob, MoveIntent};
use crate::power::{Channel, PowerUser};
use crate::rng::GameRng;
use crate::sound::SoundEvents;
use crate::time::WorldTime;

pub const FLASH: &str = "flash";
pub const FLASHER: &str = "flasher";
pub const PORTABLE_FLASHER: &str = "portable-flasher";
/// What a burnt-out flash shows.
pub const BURNT: &str = "flashburnt";

/// How long a flash in the face knocks you down, ds (Luna `Weaken(10)`, at 2 s a life tick).
pub const FLASH_WEAKEN_DS: f32 = 200.0;
/// The same for a mounted flasher, and a portable one (Luna `strength`).
pub const FLASHER_WEAKEN_DS: f32 = 200.0;
pub const PORTABLE_WEAKEN_DS: f32 = 160.0;
/// How far a flasher reaches, and how long until it can flash again, ds.
pub const FLASHER_RANGE: i32 = 4;
pub const FLASHER_COOLDOWN_DS: f32 = 150.0;
/// What a flash of a flasher costs, in watts for that moment.
pub const FLASHER_POWER: f64 = 1000.0;
/// Chance in percent that a flash burns out when used on someone.
pub const BURN_OUT_CHANCE: f32 = 10.0;
/// How far a flash fired into the air dazzles, without knocking anyone down.
pub const DAZZLE_RANGE: i32 = 3;

/// Knocked down until this time, ds: lying there, unable to move or act (Luna `weakened`).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Weakened(pub f32);

/// How many times this mob has been dazzled: its client whites out when this changes.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Dazzled(pub u8);

/// A mounted or portable flasher.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Flasher {
    pub id: String,
    /// Bulb disconnected with wirecutters.
    pub disabled: bool,
    pub portable: bool,
    /// Wrenched down: a portable one only works so.
    pub anchored: bool,
    pub last_flash_ds: Option<f32>,
}

pub fn is_flasher(proto: &str) -> bool {
    matches!(proto, FLASHER | PORTABLE_FLASHER)
}

pub fn spawn_flasher(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    let portable = spec.id == PORTABLE_FLASHER;
    let flasher = Flasher {
        id: spec
            .vars
            .get("id")
            .map(|v| v.trim().trim_matches('"').to_owned())
            .unwrap_or_default(),
        disabled: false,
        portable,
        anchored: !portable,
        last_flash_ds: None,
    };
    let state = if portable { "pflash1" } else { "mflash1" };
    let mut entity = world.spawn((
        GridPos(pos),
        id,
        Appearance::new(spec.id.clone(), state),
        flasher,
        PowerUser::new(Channel::Equipment, 0.0),
    ));
    if portable {
        entity.insert(Dense);
    }
    entity.id()
}

/// Sunglasses or a lowered welding helmet save your eyes.
fn eyes_covered(world: &World, mob: Entity) -> bool {
    let Some(worn) = world.get::<Equipment>(mob) else {
        return false;
    };
    let wearing = |slot: Slot, sprite: &str| {
        worn.get(slot)
            .and_then(|item| world.get::<Appearance>(item))
            .is_some_and(|a| a.sprite == sprite)
    };
    wearing(Slot::Glasses, "sunglasses") || wearing(Slot::Head, "welding-helmet")
}

/// Knock this mob down and white out its view, if its eyes are bare. Whether it happened.
fn blind(world: &mut World, mob: Entity, for_ds: f32) -> bool {
    if eyes_covered(world, mob) {
        return false;
    }
    let until = world.resource::<WorldTime>().deciseconds() + for_ds;
    dazzle(world, mob);
    world.entity_mut(mob).insert(Weakened(until));
    true
}

pub(crate) fn dazzle(world: &mut World, mob: Entity) {
    let count = world.get::<Dazzled>(mob).map_or(0, |d| d.0);
    world.entity_mut(mob).insert(Dazzled(count.wrapping_add(1)));
}

fn name(world: &World, who: Entity) -> String {
    world
        .get::<DisplayName>(who)
        .map(|n| n.0.clone())
        .unwrap_or_default()
}

fn play_flash_sound(world: &mut World, at: TilePos) {
    world
        .resource_mut::<SoundEvents>()
        .play(crate::sound::FLASH, at, 100.0);
}

fn click_empty(world: &mut World, user: Entity) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(user) {
        outbox.0.push(LocMsg::new("flash-click"));
    }
}

fn is_burnt(world: &World, flash: Entity) -> bool {
    world.get::<Appearance>(flash).is_some_and(|a| a.state == BURNT)
}

/// A flash used on someone (Luna `flash/attack`).
pub fn flash_someone(world: &mut World, user: Entity, flash: Entity, target: Entity) {
    if is_burnt(world, flash) {
        click_empty(world, user);
        return;
    }
    let Some(at) = world.get::<GridPos>(user).map(|p| p.0) else {
        return;
    };
    play_flash_sound(world, at);
    if blind(world, target, FLASH_WEAKEN_DS) && world.resource_mut::<GameRng>().prob(BURN_OUT_CHANCE) {
        if let Some(mut look) = world.get_mut::<Appearance>(flash) {
            look.state = BURNT.to_owned();
        }
        if let Some(mut outbox) = world.get_mut::<Outbox>(user) {
            outbox.0.push(LocMsg::new("flash-burnt-out"));
        }
    }
    let msg = LocMsg::new("flash-blinds")
        .arg("name", name(world, user))
        .arg("victim", name(world, target));
    world.resource_mut::<VisibleEvents>().0.push((at, msg, None));
}

/// A flash fired into the air (Luna `flash/attack_self`): everyone close whose eyes are bare
/// is dazzled, but stays up.
pub fn flash_around(world: &mut World, user: Entity, flash: Entity) {
    if is_burnt(world, flash) {
        click_empty(world, user);
        return;
    }
    let Some(at) = world.get::<GridPos>(user).map(|p| p.0) else {
        return;
    };
    play_flash_sound(world, at);
    for mob in seeing(world, at, DAZZLE_RANGE) {
        if mob != user && !eyes_covered(world, mob) {
            dazzle(world, mob);
        }
    }
}

/// People who can see `at`.
fn seeing(world: &mut World, at: TilePos, range: i32) -> Vec<Entity> {
    let mut walls = world.query_filtered::<&GridPos, (With<Opaque>, Without<Mob>)>();
    let opaque: Vec<TilePos> = walls.iter(world).map(|p| p.0).collect();
    let mut decks = world.query::<&rs13_world::MapLayer>();
    let Some(map) = decks.iter(world).find(|m| m.0.z == at.z).map(|m| m.0.clone()) else {
        return Vec::new();
    };
    let mut mobs = world.query_filtered::<(Entity, &GridPos, Option<&Vitals>), With<Body>>();
    mobs.iter(world)
        .filter(|(_, pos, vitals)| {
            vitals.is_none_or(|v| v.stat == rs13_world::MobStat::Conscious)
                && crate::speech::can_see(&map, &opaque, at, pos.0, range)
        })
        .map(|(mob, ..)| mob)
        .collect()
}

/// Luna `flasher/flash`: needs power and a bulb, and rests between flashes.
pub fn fire_flasher(world: &mut World, flasher: Entity) {
    let now = world.resource::<WorldTime>().deciseconds();
    let (Some(state), Some(power), Some(at)) = (
        world.get::<Flasher>(flasher),
        world.get::<PowerUser>(flasher),
        world.get::<GridPos>(flasher).map(|p| p.0),
    ) else {
        return;
    };
    let resting = state.last_flash_ds.is_some_and(|t| now < t + FLASHER_COOLDOWN_DS);
    if !power.powered || state.disabled || resting {
        return;
    }
    let weaken = if state.portable {
        PORTABLE_WEAKEN_DS
    } else {
        FLASHER_WEAKEN_DS
    };
    play_flash_sound(world, at);
    if let Some(mut state) = world.get_mut::<Flasher>(flasher) {
        state.last_flash_ds = Some(now);
    }
    if let Some(mut power) = world.get_mut::<PowerUser>(flasher) {
        power.watts = FLASHER_POWER;
    }
    for mob in seeing(world, at, FLASHER_RANGE) {
        blind(world, mob, weaken);
    }
}

/// Wirecutters on a flasher (Luna `flasher/attackby`): its bulb out or back in.
pub fn cut_bulb(world: &mut World, user: Entity, flasher: Entity) {
    let Some(mut state) = world.get_mut::<Flasher>(flasher) else {
        return;
    };
    state.disabled = !state.disabled;
    let id = if state.disabled {
        "flasher-disconnected"
    } else {
        "flasher-connected"
    };
    if let Some(mut outbox) = world.get_mut::<Outbox>(user) {
        outbox.0.push(LocMsg::new(id));
    }
}

/// A wrench on a portable flasher (Luna `portable/attackby`).
pub fn wrench_portable(world: &mut World, user: Entity, flasher: Entity) {
    let Some(mut state) = world.get_mut::<Flasher>(flasher) else {
        return;
    };
    if !state.portable {
        return;
    }
    state.anchored = !state.anchored;
    let anchored = state.anchored;
    if let Some(mut look) = world.get_mut::<Appearance>(flasher) {
        look.state = if anchored { "pflash1+pflash-s" } else { "pflash1" }.to_owned();
    }
    let id = if anchored {
        "flasher-secured"
    } else {
        "flasher-loose"
    };
    if let Some(mut outbox) = world.get_mut::<Outbox>(user) {
        outbox.0.push(LocMsg::new(id));
    }
}

/// Portable flashers go off when someone runs past (Luna `portable/HasProximity`; walking
/// slips by), and flashers stop drawing the power of their last flash.
pub fn run_flashers(world: &mut World) {
    let mut flashers = world.query::<(Entity, &Flasher, &GridPos, &mut PowerUser)>();
    let mut set: Vec<(Entity, TilePos)> = Vec::new();
    for (entity, state, pos, mut power) in flashers.iter_mut(world) {
        if power.watts != 0.0 {
            power.watts = 0.0;
        }
        if state.portable && state.anchored {
            set.push((entity, pos.0));
        }
    }
    if set.is_empty() {
        return;
    }
    let mut runners = world.query::<(&GridPos, &MoveIntent)>();
    let running: Vec<TilePos> = runners
        .iter(world)
        .filter(|(_, intent)| intent.run && intent.dir.is_some())
        .map(|(pos, _)| pos.0)
        .collect();
    for (flasher, at) in set {
        if running.iter().any(|p| crate::actions::in_reach(*p, at)) {
            fire_flasher(world, flasher);
        }
    }
}

/// Get up again once the time is up.
pub fn recover(time: Res<WorldTime>, down: Query<(Entity, &Weakened)>, mut commands: Commands) {
    let now = time.deciseconds();
    for (mob, weakened) in &down {
        if now >= weakened.0 {
            commands.entity(mob).remove::<Weakened>();
        }
    }
}
