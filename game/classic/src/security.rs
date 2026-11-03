//! Handcuffs, and the AI core's turrets with their controls (Luna's `weapon/handcuffs`,
//! `machinery/turret`, `turretid`). Spec: `docs/reference/security.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{
    Appearance, Dense, DisplayName, GridPos, Hidden, MapLayer, MobStat, NetId, ObjectSpec, Slot, TilePos,
};

use crate::access::{IdCard, allowed};
use crate::actions::Outbox;
use crate::closets::InCloset;
use crate::combat::VisibleEvents;
use crate::equipment::{Equipment, WornBy};
use crate::flashes::Weakened;
use crate::hands::{Hands, HeldBy};
use crate::health::{Body, Vitals};
use crate::machines::MACHINE_TICKS;
use crate::power::{Channel, PowerUser};
use crate::rng::GameRng;
use crate::sound::SoundEvents;
use crate::time::WorldTime;
use crate::weapons::{Fires, launch};

pub const HANDCUFFS: &str = "handcuffs";
pub const TURRET: &str = "turret";
pub const TURRET_CONTROL: &str = "turret-control";
/// How long cuffing or uncuffing someone takes, ds (Luna's `equip_e` delay).
pub const CUFF_DS: f32 = 40.0;
/// How long breaking out of cuffs takes, ds (Luna `breakouttime`).
pub const BREAKOUT_DS: f32 = 1000.0;
/// Rest between a turret's shots, ds (Luna `shot_delay`).
pub const SHOT_DELAY_DS: f32 = 30.0;
/// Who a turret control answers to (Luna `access_ai_upload`).
pub const CONTROL_ACCESS: &str = "ai_upload";
/// What a turret draws: waiting, and when shooting, W.
pub const IDLE_WATTS: f64 = 50.0;
pub const ACTIVE_WATTS: f64 = 300.0;
/// The area a turret guards must be one of Luna's `/area/turret_protected`.
pub const PROTECTED_AREA: &str = "turret-protected";

/// Wearing these handcuffs (Luna `handcuffed`): no using your hands.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cuffed(pub Entity);

/// Someone at work on someone's cuffs: done at this time, if both stay put.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct CuffWork {
    pub target: Entity,
    pub from: TilePos,
    pub target_from: TilePos,
    pub done_at: f32,
    /// Taking them off, not putting them on.
    pub removing: bool,
}

/// Breaking out of your own cuffs: done at this time, if you stay put (Luna `do_after`).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct BreakingOut {
    pub from: TilePos,
    pub done_at: f32,
}

/// Asking to break out, from the Resist key.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WantsResist(pub bool);

/// A turret, hidden in the floor until it has someone to shoot.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct Turret {
    pub raised: bool,
    pub enabled: bool,
    pub lethal: bool,
    pub last_shot_ds: Option<f32>,
}

/// A wall panel working the turrets in its area.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurretControl {
    pub enabled: bool,
    pub lethal: bool,
}

fn name_of(world: &World, who: Entity) -> String {
    world
        .get::<DisplayName>(who)
        .map(|n| n.0.clone())
        .unwrap_or_default()
}

fn seen(world: &mut World, at: TilePos, msg: LocMsg) {
    world.resource_mut::<VisibleEvents>().0.push((at, msg, None));
}

/// Luna `handcuffs/attack`: start putting the cuffs in hand on someone.
pub fn start_cuffing(world: &mut World, mob: Entity, target: Entity) {
    if world.get::<Cuffed>(target).is_some() {
        return;
    }
    begin(world, mob, target, false);
}

/// Start taking someone's cuffs off, with an empty hand (Luna's stripping panel).
pub fn start_uncuffing(world: &mut World, mob: Entity, target: Entity) {
    if world.get::<Cuffed>(target).is_some() {
        begin(world, mob, target, true);
    }
}

fn begin(world: &mut World, mob: Entity, target: Entity, removing: bool) {
    let (Some(from), Some(target_from)) = (
        world.get::<GridPos>(mob).map(|p| p.0),
        world.get::<GridPos>(target).map(|p| p.0),
    ) else {
        return;
    };
    let now = world.resource::<WorldTime>().deciseconds();
    world.entity_mut(mob).insert(CuffWork {
        target,
        from,
        target_from,
        done_at: now + CUFF_DS,
        removing,
    });
    let id = if removing {
        "cuffs-removing"
    } else {
        "cuffs-applying"
    };
    let msg = LocMsg::new(id)
        .arg("name", name_of(world, mob))
        .arg("victim", name_of(world, target));
    seen(world, from, msg);
    world
        .resource_mut::<SoundEvents>()
        .play(crate::sound::HANDCUFFS_SOUND, from, 30.0);
}

fn put_on(world: &mut World, mob: Entity, target: Entity, cuffs: Entity) {
    if let Some(mut hands) = world.get_mut::<Hands>(mob) {
        let active = hands.active;
        *hands.slot_mut(active) = None;
    }
    world
        .entity_mut(cuffs)
        .remove::<(HeldBy, GridPos)>()
        .insert(WornBy(target));
    if let Some(mut worn) = world.get_mut::<Equipment>(target) {
        worn.0.insert(Slot::Cuffs, cuffs);
    }
    world.entity_mut(target).insert(Cuffed(cuffs));
}

/// Take someone's cuffs off, onto the floor.
fn take_off(world: &mut World, person: Entity) {
    let Some(Cuffed(cuffs)) = world.get::<Cuffed>(person).copied() else {
        return;
    };
    let Some(at) = world.get::<GridPos>(person).map(|p| p.0) else {
        return;
    };
    world.entity_mut(person).remove::<Cuffed>();
    if let Some(mut worn) = world.get_mut::<Equipment>(person) {
        worn.0.remove(&Slot::Cuffs);
    }
    world.entity_mut(cuffs).remove::<WornBy>().insert(GridPos(at));
}

/// Cuffing and breaking out, as time passes.
pub fn run_cuffs(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    let mut workers = world.query::<(Entity, &CuffWork, &GridPos)>();
    let work: Vec<(Entity, CuffWork, TilePos)> = workers.iter(world).map(|(e, w, p)| (e, *w, p.0)).collect();
    for (mob, job, at) in work {
        let target_at = world.get::<GridPos>(job.target).map(|p| p.0);
        let still = at == job.from && target_at == Some(job.target_from);
        if !still {
            world.entity_mut(mob).remove::<CuffWork>();
            continue;
        }
        if now < job.done_at {
            continue;
        }
        world.entity_mut(mob).remove::<CuffWork>();
        let msg = LocMsg::new(if job.removing {
            "cuffs-removed"
        } else {
            "cuffs-applied"
        })
        .arg("name", name_of(world, mob))
        .arg("victim", name_of(world, job.target));
        if job.removing {
            take_off(world, job.target);
        } else {
            let held = world.get::<Hands>(mob).and_then(Hands::active_item);
            let Some(cuffs) =
                held.filter(|h| world.get::<Appearance>(*h).is_some_and(|a| a.sprite == HANDCUFFS))
            else {
                continue;
            };
            put_on(world, mob, job.target, cuffs);
        }
        seen(world, at, msg);
    }

    // The Resist key: only cuffs to resist so far.
    let mut wanting = world.query::<(Entity, &mut WantsResist, &GridPos, Has<Cuffed>, Has<BreakingOut>)>();
    let asked: Vec<(Entity, TilePos)> = wanting
        .iter_mut(world)
        .filter_map(|(e, mut w, p, cuffed, busy)| {
            let asked = std::mem::take(&mut w.0);
            (asked && cuffed && !busy).then_some((e, p.0))
        })
        .collect();
    for (mob, at) in asked {
        world.entity_mut(mob).insert(BreakingOut {
            from: at,
            done_at: now + BREAKOUT_DS,
        });
        let msg = LocMsg::new("cuffs-resisting").arg("name", name_of(world, mob));
        seen(world, at, msg);
    }
    let mut breaking = world.query::<(Entity, &BreakingOut, &GridPos)>();
    let breaking: Vec<(Entity, BreakingOut, TilePos)> =
        breaking.iter(world).map(|(e, b, p)| (e, *b, p.0)).collect();
    for (mob, job, at) in breaking {
        if at != job.from {
            world.entity_mut(mob).remove::<BreakingOut>();
        } else if now >= job.done_at {
            world.entity_mut(mob).remove::<BreakingOut>();
            take_off(world, mob);
            let msg = LocMsg::new("cuffs-broken-out").arg("name", name_of(world, mob));
            seen(world, at, msg);
        }
    }
}

pub fn is_turret_part(proto: &str) -> bool {
    matches!(proto, TURRET | TURRET_CONTROL)
}

pub fn spawn_turret_part(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    if spec.id == TURRET {
        return world
            .spawn((
                GridPos(pos),
                id,
                Appearance::new(TURRET, "grey_target_prism"),
                Hidden,
                Dense,
                Turret {
                    enabled: true,
                    ..Turret::default()
                },
                PowerUser::new(Channel::Equipment, IDLE_WATTS),
            ))
            .id();
    }
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(TURRET_CONTROL, "motion3"),
            TurretControl {
                enabled: true,
                lethal: false,
            },
        ))
        .id()
}

fn area_of(world: &mut World, at: TilePos) -> Option<String> {
    let mut decks = world.query::<&MapLayer>();
    decks
        .iter(world)
        .find(|m| m.0.z == at.z)
        .and_then(|m| m.0.area(at).map(str::to_owned))
}

/// A click on a turret control (Luna's panel, as one button): on and stunning, on and lethal,
/// off. It wants the right card.
pub fn click_control(world: &mut World, mob: Entity, control: Entity) {
    let held = world.get::<Hands>(mob).and_then(Hands::active_item);
    let worn = world.get::<Equipment>(mob).and_then(|e| e.get(Slot::Id));
    let card = [held, worn]
        .into_iter()
        .flatten()
        .find_map(|item| world.get::<IdCard>(item).cloned());
    if !allowed(&[CONTROL_ACCESS.to_owned()], card.as_ref()) {
        if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
            outbox.0.push(LocMsg::new("turret-denied"));
        }
        return;
    }
    let Some(at) = world.get::<GridPos>(control).map(|p| p.0) else {
        return;
    };
    let Some(mut state) = world.get_mut::<TurretControl>(control) else {
        return;
    };
    let (enabled, lethal) = match (state.enabled, state.lethal) {
        (true, false) => (true, true),
        (true, true) => (false, false),
        (false, _) => (true, false),
    };
    state.enabled = enabled;
    state.lethal = lethal;
    let area = area_of(world, at);
    let mut turrets = world.query::<(&GridPos, &mut Turret)>();
    let places: Vec<TilePos> = turrets.iter(world).map(|(p, _)| p.0).collect();
    let areas: Vec<Option<String>> = places.iter().map(|p| area_of(world, *p)).collect();
    let mut turrets = world.query::<&mut Turret>();
    for (mut turret, there) in turrets.iter_mut(world).zip(areas) {
        if there == area {
            turret.enabled = enabled;
            turret.lethal = lethal;
        }
    }
    let id = match (enabled, lethal) {
        (true, true) => "turret-set-lethal",
        (true, false) => "turret-set-stun",
        _ => "turret-set-off",
    };
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(LocMsg::new(id));
    }
}

/// Turrets rise for anyone in their area who is up (or anyone at all, lethal), and shoot one at a
/// time; with nobody there they sink back (Luna `turret/process`).
pub fn run_turrets(world: &mut World) {
    let tick = world.resource::<WorldTime>().tick.0;
    if !tick.is_multiple_of(MACHINE_TICKS / 4) {
        return;
    }
    let now = world.resource::<WorldTime>().deciseconds();
    let mut q = world.query::<(Entity, &GridPos, &Turret, &PowerUser)>();
    let turrets: Vec<(Entity, TilePos, Turret, bool)> = q
        .iter(world)
        .map(|(e, p, t, power)| (e, p.0, *t, power.powered))
        .collect();
    if turrets.is_empty() {
        return;
    }
    let mut people = world.query_filtered::<(Entity, &GridPos, Option<&Vitals>, Has<Weakened>), (With<Body>, Without<InCloset>)>();
    let people: Vec<(Entity, TilePos, bool, bool)> = people
        .iter(world)
        .map(|(e, p, v, down)| (e, p.0, v.is_none_or(|v| v.stat == MobStat::Conscious), down))
        .collect();
    for (entity, at, turret, powered) in turrets {
        let area = area_of(world, at);
        let guarded = area.as_deref().is_some_and(|a| a.starts_with(PROTECTED_AREA));
        let mut targets: Vec<(Entity, TilePos)> = Vec::new();
        if turret.enabled && powered && guarded {
            for (person, pos, awake, down) in &people {
                if *awake && (!*down || turret.lethal) && area_of(world, *pos) == area {
                    targets.push((*person, *pos));
                }
            }
        }
        let raise = !targets.is_empty();
        if raise != turret.raised {
            let mut e = world.entity_mut(entity);
            if raise {
                e.remove::<Hidden>();
            } else {
                e.insert(Hidden);
            }
            if let Some(mut t) = e.get_mut::<Turret>() {
                t.raised = raise;
            }
            if let Some(mut power) = e.get_mut::<PowerUser>() {
                power.watts = if raise { ACTIVE_WATTS } else { IDLE_WATTS };
            }
            continue;
        }
        if !raise || turret.last_shot_ds.is_some_and(|t| now < t + SHOT_DELAY_DS) {
            continue;
        }
        let index = (world.resource_mut::<GameRng>().range(1, targets.len() as i32) - 1) as usize;
        let (_, to) = targets[index];
        if let Some(mut t) = world.get_mut::<Turret>(entity) {
            t.last_shot_ds = Some(now);
        }
        let fires = if turret.lethal {
            Fires::Beam
        } else {
            Fires::Electrode
        };
        launch(world, entity, at, to, fires);
    }
}
