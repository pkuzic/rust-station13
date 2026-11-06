//! Spells (Luna's `datums/spell.dm` and `datums/spells`): known by a wizard an admin has given
//! them to, and the mime's invisible wall. Cast from the spell window (F7). Spec:
//! `docs/reference/spells.md`.

use std::sync::OnceLock;

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{
    Appearance, Dense, Dir, Facing, GridPos, HumanLook, MapLayer, MobStat, Opaque, Slot, SpellAim, SpellLine,
    SpellPanelView, TilePos,
};
use serde::Deserialize;

use crate::actions::Outbox;
use crate::combat::{COUNT_DS, Muted};
use crate::doors::{Airlock, DoorState};
use crate::equipment::Equipment;
use crate::explosions::{empulse, explode};
use crate::health::{Body, Vitals, Zone};
use crate::movement::Mob;
use crate::objects::NetIds;
use crate::rng::GameRng;
use crate::speech::{PendingSpeech, can_see};
use crate::time::WorldTime;

/// How a spell's words are said.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Say {
    Shout,
    Whisper,
}

/// What a spell does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Effect {
    MagicMissile,
    DisableTech,
    Blink,
    Teleport,
    Forcewall,
    Fireball,
    Knock,
    InvisibleWall,
}

/// One of `content/spells.ron`.
#[derive(Debug, Deserialize)]
pub struct Spell {
    pub id: String,
    pub cooldown: f32,
    pub clothes: bool,
    pub words: Option<(String, Say)>,
    pub effect: Effect,
}

pub fn spells() -> &'static [Spell] {
    static SPELLS: OnceLock<Vec<Spell>> = OnceLock::new();
    SPELLS.get_or_init(|| {
        rs13_content::parse_ron("content/spells.ron", include_str!("../content/spells.ron"))
            .expect("built-in spell list is valid")
    })
}

pub fn spell(id: &str) -> Option<&'static Spell> {
    spells().iter().find(|s| s.id == id)
}

/// The mime's spell (Luna `jobprocs`).
pub const MIME_SPELL: &str = "invisible-wall";
/// How far Magic Missile looks for targets (Luna `range`, in view).
pub const MISSILE_RANGE: i32 = 7;
/// A missile's steps and how often it takes one, ds (Luna `proj_lifespan`, `proj_step_delay`).
pub const MISSILE_STEPS: u8 = 20;
pub const MISSILE_STEP_DS: f32 = 5.0;
/// What a missile does: knocked down for 5 counts, and 10 burn (Luna `inflict_handler`).
pub const MISSILE_WEAKEN_COUNTS: f32 = 5.0;
pub const MISSILE_BURN: f32 = 10.0;
/// A fireball's steps, one each ds (Luna `dumbfire`), and its blast.
pub const FIREBALL_STEPS: u16 = 200;
pub const FIREBALL_STEP_DS: f32 = 1.0;
/// How long a forcewall stands, ds (Luna `summon_lifespan`).
pub const WALL_DS: f32 = 300.0;
/// How far Knock reaches, and how far Blink can go and how close to the edge of the map it lands.
pub const KNOCK_RANGE: i32 = 3;
pub const BLINK_RANGE: i32 = 6;

/// Sprites.
pub const MISSILE_SPRITE: &str = "magic-missile";
pub const FIREBALL_SPRITE: &str = "fireball";
pub const FORCEWALL_SPRITE: &str = "forcewall";

/// One spell someone knows, and when it's next ready.
#[derive(Debug, Clone, PartialEq)]
pub struct Known {
    pub id: String,
    pub ready_at: f32,
}

/// The spells someone knows (Luna `spell_list`), in the order they were learned.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct Spellcaster {
    pub known: Vec<Known>,
}

/// Has the spell window open.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellViewer;

/// The window as last worked out, for the server to send.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellPanelOut(pub Option<SpellPanelView>);

/// A magic missile, homing in.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Missile {
    pub target: Entity,
    pub steps_left: u8,
    pub next_at: f32,
}

/// A fireball, flying straight.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Fireball {
    pub caster: Entity,
    pub dir: Dir,
    pub steps_left: u16,
    pub next_at: f32,
}

/// A conjured wall, gone at `until`.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Forcefield {
    pub until: f32,
}

/// What's asked of the spell window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellOp {
    Open,
    Cast {
        spell: u8,
        target: Option<Entity>,
        area: Option<u16>,
    },
}

fn tell(world: &mut World, mob: Entity, msg: LocMsg) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(msg);
    }
}

/// Teach someone a spell (Luna's admin "Give Spell"). False if there's no such spell or they know it.
pub fn give(world: &mut World, mob: Entity, id: &str) -> bool {
    if spell(id).is_none() {
        return false;
    }
    if world.get::<Spellcaster>(mob).is_none() {
        world.entity_mut(mob).insert(Spellcaster::default());
    }
    let Some(mut caster) = world.get_mut::<Spellcaster>(mob) else {
        return false;
    };
    if caster.known.iter().any(|k| k.id == id) {
        return false;
    }
    // Spells start ready (Luna `New`).
    caster.known.push(Known {
        id: id.to_owned(),
        ready_at: 0.0,
    });
    true
}

/// Whether someone knows a spell.
pub fn knows(world: &World, mob: Entity, id: &str) -> bool {
    world
        .get::<Spellcaster>(mob)
        .is_some_and(|c| c.known.iter().any(|k| k.id == id))
}

/// Forget every spell (Luna `spellremove`), and close the window.
pub fn forget_all(world: &mut World, mob: Entity) {
    if let Some(mut caster) = world.get_mut::<Spellcaster>(mob) {
        caster.known.clear();
    }
    world.entity_mut(mob).remove::<SpellViewer>();
}

pub fn command(world: &mut World, mob: Entity, op: SpellOp) {
    match op {
        SpellOp::Open => {
            if world.get::<SpellViewer>(mob).is_some() {
                world.entity_mut(mob).remove::<SpellViewer>();
            } else if world.get::<Spellcaster>(mob).is_some_and(|c| !c.known.is_empty()) {
                world
                    .entity_mut(mob)
                    .insert((SpellViewer, SpellPanelOut::default()));
            }
        }
        SpellOp::Cast { spell, target, area } => cast(world, mob, usize::from(spell), target, area),
    }
}

fn worn(world: &World, mob: Entity, slot: Slot) -> Option<String> {
    let item = world.get::<Equipment>(mob)?.get(slot)?;
    world.get::<Appearance>(item).map(|a| a.sprite.clone())
}

/// What a wizard must wear (Luna's `clothes_req`): the first thing missing, if any.
fn missing_clothes(world: &World, mob: Entity) -> Option<&'static str> {
    // The spellbook's hardsuit and its helmet count too (Luna `rig/wizard`).
    if !matches!(
        worn(world, mob, Slot::Suit).as_deref(),
        Some("wizard-robe" | "wizard-hardsuit")
    ) {
        return Some("spell-no-robe");
    }
    if worn(world, mob, Slot::Shoes).as_deref() != Some("sandals") {
        return Some("spell-no-sandals");
    }
    if !matches!(
        worn(world, mob, Slot::Head).as_deref(),
        Some("wizard-hat" | "wizard-hardsuit-helmet")
    ) {
        return Some("spell-no-hat");
    }
    None
}

/// Luna's `cast_check`, `choose_targets`, `invocation` and `cast`.
fn cast(world: &mut World, mob: Entity, line: usize, _target: Option<Entity>, area: Option<u16>) {
    let Some(known) = world
        .get::<Spellcaster>(mob)
        .and_then(|c| c.known.get(line).cloned())
    else {
        return;
    };
    let Some(def) = spell(&known.id) else {
        return;
    };
    let now = world.resource::<WorldTime>().deciseconds();
    let name = || LocMsg::new("spell-recharging").msg("name", format!("spell-{}", def.id));
    if now < known.ready_at {
        tell(world, mob, name());
        return;
    }
    if world
        .get::<Vitals>(mob)
        .is_some_and(|v| v.stat != MobStat::Conscious)
    {
        tell(world, mob, LocMsg::new("spell-incapacitated"));
        return;
    }
    // Luna only checks people, and only charges them: anyone else casts for free.
    let person = world.get::<HumanLook>(mob).is_some();
    if person {
        let silenced =
            world.get::<Muted>(mob).is_some() || world.get::<crate::changeling::Silenced>(mob).is_some();
        if def.words.is_some() && silenced {
            tell(world, mob, LocMsg::new("spell-cant-speak"));
            return;
        }
        if def.clothes
            && let Some(id) = missing_clothes(world, mob)
        {
            tell(world, mob, LocMsg::new(id));
            return;
        }
        set_ready(world, mob, line, now + def.cooldown);
    }
    let Some(extra_words) = perform(world, mob, def.effect, area) else {
        // No targets: the charge comes back.
        if person {
            set_ready(world, mob, line, known.ready_at);
        }
        return;
    };
    say(world, mob, def, extra_words.as_deref());
}

fn set_ready(world: &mut World, mob: Entity, line: usize, at: f32) {
    if let Some(mut caster) = world.get_mut::<Spellcaster>(mob)
        && let Some(known) = caster.known.get_mut(line)
    {
        known.ready_at = at;
    }
}

/// The words, half the time with backticks for spaces (Luna's way round its own spam filter).
fn say(world: &mut World, mob: Entity, def: &Spell, extra: Option<&str>) {
    let Some((words, how)) = &def.words else {
        return;
    };
    let mut line = match extra {
        Some(extra) => format!("{words} {extra}"),
        None => words.clone(),
    };
    if world.resource_mut::<GameRng>().prob(50.0) {
        line = line.replace(' ', "`");
    }
    let line = match how {
        Say::Shout => line,
        Say::Whisper => format!(":w {line}"),
    };
    if let Some(mut speech) = world.get_mut::<PendingSpeech>(mob) {
        speech.0.push(line);
    }
}

fn at(world: &World, mob: Entity) -> Option<TilePos> {
    world.get::<GridPos>(mob).map(|p| p.0)
}

/// Do it. `None` if it found nothing to act on; otherwise any words to add to the spell's.
fn perform(world: &mut World, mob: Entity, effect: Effect, area: Option<u16>) -> Option<Option<String>> {
    let here = at(world, mob)?;
    match effect {
        Effect::MagicMissile => {
            let targets = living_in_view(world, mob, here, MISSILE_RANGE);
            if targets.is_empty() {
                return None;
            }
            let now = world.resource::<WorldTime>().deciseconds();
            for target in targets {
                let id = world.resource_mut::<NetIds>().alloc();
                world.spawn((
                    GridPos(here),
                    id,
                    Appearance::new(MISSILE_SPRITE, ""),
                    Missile {
                        target,
                        steps_left: MISSILE_STEPS,
                        next_at: now + MISSILE_STEP_DS,
                    },
                ));
            }
        }
        Effect::DisableTech => empulse(world, here, 6, 10),
        Effect::Blink => blink(world, mob, here),
        Effect::Teleport => {
            let name = teleport_areas(world).get(usize::from(area?)).cloned()?;
            if let Some(to) = clear_tile_in(world, &name) {
                world.entity_mut(mob).insert(GridPos(to));
            }
            return Some(Some(name.replace('-', " ").to_uppercase()));
        }
        Effect::Forcewall => {
            wall(world, here, true, WALL_DS);
        }
        Effect::InvisibleWall => {
            wall(world, here, false, WALL_DS);
        }
        Effect::Fireball => {
            let dir = facing(world, mob);
            let id = world.resource_mut::<NetIds>().alloc();
            let now = world.resource::<WorldTime>().deciseconds();
            world.spawn((
                GridPos(here),
                id,
                Appearance::new(FIREBALL_SPRITE, ""),
                Fireball {
                    caster: mob,
                    dir,
                    steps_left: FIREBALL_STEPS,
                    next_at: now + FIREBALL_STEP_DS,
                },
            ));
        }
        Effect::Knock => knock(world, here, KNOCK_RANGE),
    }
    Some(None)
}

fn facing(world: &World, mob: Entity) -> Dir {
    match world.get::<Facing>(mob).map(|f| f.0) {
        Some(1) => Dir::North,
        Some(2) => Dir::East,
        Some(3) => Dir::West,
        _ => Dir::South,
    }
}

fn opaque_things(world: &mut World) -> Vec<TilePos> {
    let mut query = world.query_filtered::<&GridPos, (With<Opaque>, Without<Mob>)>();
    query.iter(world).map(|p| p.0).collect()
}

fn map_of(world: &mut World, z: i32) -> Option<rs13_world::TileMap> {
    let mut layers = world.query::<&MapLayer>();
    layers.iter(world).find(|l| l.0.z == z).map(|l| l.0.clone())
}

/// Every living mob but the caster in sight within `range` (Luna `view`).
fn living_in_view(world: &mut World, mob: Entity, here: TilePos, range: i32) -> Vec<Entity> {
    let Some(map) = map_of(world, here.z) else {
        return Vec::new();
    };
    let opaque = opaque_things(world);
    let mut mobs = world.query_filtered::<(Entity, &GridPos, Option<&Vitals>), With<Mob>>();
    let mut found: Vec<Entity> = mobs
        .iter(world)
        .filter(|(e, p, v)| {
            *e != mob
                && p.0.z == here.z
                && v.is_some_and(|v| v.stat != MobStat::Dead)
                && can_see(&map, &opaque, here, p.0, range)
        })
        .map(|(e, ..)| e)
        .collect();
    found.sort();
    found
}

fn dense_at(world: &mut World) -> Vec<TilePos> {
    let mut dense = world.query_filtered::<&GridPos, With<Dense>>();
    dense.iter(world).map(|p| p.0).collect()
}

/// Luna's `turf_teleport` for Blink: a random floor 1 to 6 tiles away, not within 6 of the map's
/// edge. With none, nothing happens.
fn blink(world: &mut World, mob: Entity, here: TilePos) {
    let Some(map) = map_of(world, here.z) else {
        return;
    };
    let (w, h) = (map.width as i32, map.height as i32);
    let mut picks = Vec::new();
    for y in here.y - BLINK_RANGE..=here.y + BLINK_RANGE {
        for x in here.x - BLINK_RANGE..=here.x + BLINK_RANGE {
            let to = TilePos::new(x, y, here.z);
            if to == here
                || x < BLINK_RANGE
                || y < BLINK_RANGE
                || x >= w - BLINK_RANGE
                || y >= h - BLINK_RANGE
            {
                continue;
            }
            if map.tile(to).is_some_and(|t| !t.solid && t.id != "space") {
                picks.push(to);
            }
        }
    }
    if picks.is_empty() {
        return;
    }
    let i = world.resource_mut::<GameRng>().range(0, picks.len() as i32 - 1) as usize;
    world.entity_mut(mob).insert(GridPos(picks[i]));
}

/// Where Teleport can go (Luna `teleportlocs`): every area but shuttles, the syndicate's, the
/// wizard's and space, by name.
pub fn teleport_areas(world: &mut World) -> Vec<String> {
    let mut layers = world.query::<&MapLayer>();
    let mut names: Vec<String> = layers
        .iter(world)
        .flat_map(|l| l.0.area_names.iter().cloned())
        .filter(|n| {
            !n.is_empty()
                && !n.starts_with("shuttle")
                && !n.contains("syndicate")
                && !n.contains("wizard")
                && n != "space"
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

/// A random floor of the area with nothing dense on it.
pub(crate) fn clear_tile_in(world: &mut World, area: &str) -> Option<TilePos> {
    let dense = dense_at(world);
    let mut layers = world.query::<&MapLayer>();
    let mut clear = Vec::new();
    for layer in layers.iter(world) {
        for y in 0..layer.0.height as i32 {
            for x in 0..layer.0.width as i32 {
                let at = TilePos::new(x, y, layer.0.z);
                if layer.0.area(at) == Some(area)
                    && layer.0.tile(at).is_some_and(|t| !t.solid)
                    && !dense.contains(&at)
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

/// A wall on `here` for `ds`: seen, or not (the mime's).
pub(crate) fn wall(world: &mut World, here: TilePos, seen: bool, ds: f32) {
    let until = world.resource::<WorldTime>().deciseconds() + ds;
    let wall = world.spawn((GridPos(here), Dense, Forcefield { until })).id();
    if seen {
        let id = world.resource_mut::<NetIds>().alloc();
        world
            .entity_mut(wall)
            .insert((id, Appearance::new(FORCEWALL_SPRITE, "")));
    }
}

/// Every airlock within `range` unbolted and opened (Luna `knock`).
pub(crate) fn knock(world: &mut World, here: TilePos, range: i32) {
    let now = world.resource::<WorldTime>().deciseconds();
    let mut doors = world.query::<(&GridPos, &mut Airlock)>();
    for (pos, mut door) in doors.iter_mut(world) {
        let p = pos.0;
        if p.z != here.z || (p.x - here.x).abs().max((p.y - here.y).abs()) > range {
            continue;
        }
        door.bolted = false;
        if door.state == DoorState::Closed {
            door.toggle(now);
        }
    }
}

/// Missiles home in, fireballs fly, conjured walls fade.
pub fn run_spells(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    let mut walls = world.query::<(Entity, &Forcefield)>();
    let gone: Vec<Entity> = walls
        .iter(world)
        .filter(|(_, f)| now >= f.until)
        .map(|(e, _)| e)
        .collect();
    for wall in gone {
        world.entity_mut(wall).despawn();
    }
    run_missiles(world, now);
    run_fireballs(world, now);
}

fn run_missiles(world: &mut World, now: f32) {
    let mut missiles = world.query::<(Entity, &Missile, &GridPos)>();
    let mut due: Vec<(Entity, Missile, TilePos)> = missiles
        .iter(world)
        .filter(|(_, m, _)| now >= m.next_at)
        .map(|(e, m, p)| (e, *m, p.0))
        .collect();
    due.sort_by_key(|(e, ..)| *e);
    for (missile, state, pos) in due {
        let Some(goal) = at(world, state.target) else {
            world.entity_mut(missile).despawn();
            continue;
        };
        // It goes through whatever's in the way (Luna's homing projectile).
        let next = if goal.z == pos.z {
            TilePos::new(
                pos.x + (goal.x - pos.x).signum(),
                pos.y + (goal.y - pos.y).signum(),
                pos.z,
            )
        } else {
            pos
        };
        if next == goal {
            world.entity_mut(missile).despawn();
            crate::combat::weaken(world, state.target, MISSILE_WEAKEN_COUNTS * COUNT_DS);
            if let Some(mut body) = world.get_mut::<Body>(state.target) {
                body.take_damage(Zone::Chest, 0.0, MISSILE_BURN);
            }
            continue;
        }
        if state.steps_left <= 1 {
            world.entity_mut(missile).despawn();
            continue;
        }
        world.entity_mut(missile).insert((
            GridPos(next),
            Missile {
                steps_left: state.steps_left - 1,
                next_at: now + MISSILE_STEP_DS,
                ..state
            },
        ));
    }
}

fn run_fireballs(world: &mut World, now: f32) {
    let mut balls = world.query::<(Entity, &Fireball, &GridPos)>();
    let mut due: Vec<(Entity, Fireball, TilePos)> = balls
        .iter(world)
        .filter(|(_, f, _)| now >= f.next_at)
        .map(|(e, f, p)| (e, *f, p.0))
        .collect();
    due.sort_by_key(|(e, ..)| *e);
    if due.is_empty() {
        return;
    }
    let dense = dense_at(world);
    let mut mobs = world.query_filtered::<(Entity, &GridPos, Option<&Vitals>), With<Mob>>();
    let living: Vec<(Entity, TilePos)> = mobs
        .iter(world)
        .filter(|(_, _, v)| v.is_some_and(|v| v.stat != MobStat::Dead))
        .map(|(e, p, _)| (e, p.0))
        .collect();
    for (ball, state, pos) in due {
        let near = living.iter().any(|(e, p)| {
            *e != state.caster && p.z == pos.z && (p.x - pos.x).abs().max((p.y - pos.y).abs()) <= 1
        });
        let next = pos.step(state.dir);
        let blocked = map_of(world, pos.z).is_none_or(|m| m.is_solid(next))
            || dense
                .iter()
                .any(|p| *p == next && !living.iter().any(|(e, lp)| *lp == next && *e == state.caster));
        if near || blocked || state.steps_left <= 1 {
            world.entity_mut(ball).despawn();
            explode(world, pos, -1, 0, 2, 3);
            continue;
        }
        world.entity_mut(ball).insert((
            GridPos(next),
            Fireball {
                steps_left: state.steps_left - 1,
                next_at: now + FIREBALL_STEP_DS,
                ..state
            },
        ));
    }
}

/// Work out each open window.
pub fn update_spell_views(world: &mut World) {
    let mut viewers = world.query_filtered::<Entity, (With<SpellViewer>, With<Spellcaster>)>();
    let list: Vec<Entity> = viewers.iter(world).collect();
    if list.is_empty() {
        return;
    }
    let now = world.resource::<WorldTime>().deciseconds();
    let areas = teleport_areas(world);
    for mob in list {
        let Some(caster) = world.get::<Spellcaster>(mob).cloned() else {
            continue;
        };
        let spells = caster
            .known
            .iter()
            .filter_map(|k| {
                let def = spell(&k.id)?;
                let max = (def.cooldown / 10.0).round() as u16;
                let so_far = ((now - (k.ready_at - def.cooldown)) / 10.0)
                    .floor()
                    .clamp(0.0, f32::from(max)) as u16;
                Some(SpellLine {
                    id: k.id.clone(),
                    now: if now >= k.ready_at { max } else { so_far },
                    max,
                    charges: false,
                    aim: if def.effect == Effect::Teleport {
                        SpellAim::Area
                    } else {
                        SpellAim::None
                    },
                })
            })
            .collect();
        let view = SpellPanelView {
            spells,
            nearby: Vec::new(),
            areas: areas.clone(),
        };
        if let Some(mut out) = world.get_mut::<SpellPanelOut>(mob)
            && out.0.as_ref() != Some(&view)
        {
            out.0 = Some(view);
        }
    }
}

pub fn clear_closed_spell_views(mut out: Query<&mut SpellPanelOut, Without<SpellViewer>>) {
    for mut out in &mut out {
        if out.0.is_some() {
            out.0 = None;
        }
    }
}

/// The mime starts with the invisible wall (Luna `jobprocs`).
pub fn learn_for_job(world: &mut World, mob: Entity, job: &str) {
    if job == "mime" {
        give(world, mob, MIME_SPELL);
    }
}
