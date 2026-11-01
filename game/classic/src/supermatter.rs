//! The engine core: the supermatter crystal, the lasers that feed it and the computer that
//! sets them. Spec: `docs/reference/engine.md`.

use std::collections::{HashMap, HashSet};

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_simulation::atmos::Gas;
use rs13_simulation::atmos::gas::{ONE_ATMOSPHERE, R, T0C, T20C, TILE_VOLUME};
use rs13_world::{
    Appearance, Dense, Dir, GridPos, LaserPanelView, LightSource, MobStat, NetId, ObjectSpec, TilePos,
};

use crate::actions::{Outbox, in_reach};
use crate::atmos::StationAtmos;
use crate::equipment::WornBy;
use crate::hands::HeldBy;
use crate::health::{Body, Vitals, Zone};
use crate::machines::MACHINE_TICKS;
use crate::movement::Mob;
use crate::objects::NetIds;
use crate::power::{Channel, PowerUser};
use crate::radio::{COMMON_FREQUENCY, Radio, format_frequency};
use crate::time::WorldTime;
use crate::wires::PanelViewer;

pub const SUPERMATTER: &str = "supermatter";
pub const LASER: &str = "engine-laser";
pub const LASER_COMPUTER: &str = "laser-computer";

/// A laser's power when built (Luna `power = 500`), and its range (Luna's clamp).
pub const LASER_POWER: u32 = 500;
pub const LASER_POWER_RANGE: (i32, i32) = (1, 3000);
/// Burn a laser beam does to each body part of whoever stands in it, per power, per cycle
/// (Luna `take_damage(0, power / 400)`).
pub const BEAM_BURN_PER_POWER: f32 = 1.0 / 400.0;
/// A beam's glow (Luna `ul_SetLuminosity(1, 1, 4)`).
pub const BEAM_LIGHT: [u8; 3] = [1, 1, 4];
/// The laser computer's draw, W (Luna `use_power(250)`).
pub const LASER_COMPUTER_POWER: f64 = 250.0;

/// Share of the air on its tile the crystal works on each cycle (Luna `gasefficency`).
pub const GAS_EFFICIENCY: f64 = 0.25;
/// Above this the crystal takes damage, K, at (T − 1100) / 150 a cycle (Luna `det`).
pub const DAMAGE_TEMPERATURE: f64 = 1100.0;
pub const DAMAGE_DIVISOR: f64 = 150.0;
/// Damage at which Luna's crystal explodes (`explosiondet`).
pub const EXPLOSION_DAMAGE: f64 = 3500.0;
/// Hottest the crystal heats gas, K (Luna `min(temperature, 1500)`).
pub const MAX_TEMPERATURE: f64 = 1500.0;
/// Luna's reaction constants (`SuperMatter.dm`).
pub const NITROGEN_RETARDATION: f64 = 4.0;
pub const THERMAL_RELEASE: f64 = 50.0;
pub const OXYGEN_RELEASE: f64 = 1500.0;
pub const REACTION_POWER: f64 = 1.4;
/// Within this many tiles the crystal kills (Luna `range(src, 3)` → `gib()`).
pub const KILL_RANGE: i32 = 3;
/// Seconds between the core computer's radio warnings (Luna `warningtime`).
pub const WARNING_SECONDS: f64 = 60.0;

/// The supermatter crystal.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Supermatter {
    /// Damage (Luna `det`).
    pub damage: f64,
    /// World time of the last radio warning, seconds.
    pub last_warning: Option<f64>,
}

/// A zero-point laser, firing `dir` while on.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct EngineLaser {
    pub id: String,
    pub dir: Dir,
    pub power: u32,
    pub on: bool,
}

/// One tile of a laser beam.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Beam {
    pub power: u32,
}

/// The laser control computer, and the ids of the lasers it sets.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct LaserComputer {
    pub ids: Vec<String>,
}

/// The beam tiles each laser has drawn.
#[derive(Resource, Debug, Default)]
pub struct Beams(pub HashMap<Entity, (Vec<TilePos>, Vec<Entity>)>);

/// Laser computer buttons waiting for the next tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaserOp {
    Power(i32),
    Toggle,
}

#[derive(Component, Debug, Default)]
pub struct PendingLaserOps(pub Vec<(Entity, LaserOp)>);

/// What the mob's open laser computer shows, for the server to send when it changes.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct LaserPanelOut(pub Option<LaserPanelView>);

/// Quoted names in a BYOND list, `list("north","west")`, or a single bare name.
pub fn id_list(text: &str) -> Vec<String> {
    let names: Vec<String> = text.split('"').skip(1).step_by(2).map(str::to_owned).collect();
    if names.is_empty() && !text.trim().is_empty() {
        vec![text.trim().to_owned()]
    } else {
        names
    }
}

pub fn spawn_supermatter(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(spec.id.as_str(), ""),
            Supermatter {
                damage: 0.0,
                last_warning: None,
            },
            Dense,
        ))
        .id()
}

pub fn spawn_laser(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    let laser = EngineLaser {
        id: spec
            .vars
            .get("id")
            .map(|v| v.trim().to_owned())
            .unwrap_or_default(),
        dir: spec.dir.unwrap_or(Dir::South),
        power: LASER_POWER,
        on: false,
    };
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(spec.id.as_str(), ""),
            laser,
            Dense,
        ))
        .id()
}

pub fn spawn_laser_computer(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    let ids = spec.vars.get("id").map(|v| id_list(v)).unwrap_or_default();
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(spec.id.as_str(), "atmos"),
            LaserComputer { ids },
            PowerUser::new(Channel::Equipment, LASER_COMPUTER_POWER),
            Dense,
        ))
        .id()
}

/// Every machine cycle (Luna `laser/process` and `e_beam/updatebeam`): each laser that is on
/// fires a beam tile by tile until something dense, a wall or a person stops it. Whoever
/// stops it is burnt. The beam is redrawn only when its path or power changes.
#[allow(clippy::too_many_arguments)]
pub fn fire_lasers(
    time: Res<WorldTime>,
    map: crate::decks::Decks,
    mut beams: ResMut<Beams>,
    lasers: Query<(Entity, &GridPos, &EngineLaser)>,
    blockers: Query<&GridPos, (With<Dense>, Without<Mob>, Without<Beam>)>,
    mut mobs: Query<(&GridPos, &mut Body), With<Mob>>,
    mut ids: ResMut<NetIds>,
    mut commands: Commands,
) {
    if !time.tick.0.is_multiple_of(MACHINE_TICKS) {
        return;
    }
    let dense: HashSet<TilePos> = blockers.iter().map(|p| p.0).collect();
    let people: HashSet<TilePos> = mobs.iter().map(|(p, _)| p.0).collect();
    let mut live: HashSet<Entity> = HashSet::new();
    for (laser_entity, pos, laser) in &lasers {
        if !laser.on {
            continue;
        }
        live.insert(laser_entity);
        let mut path = Vec::new();
        let mut at = pos.0;
        let deck = map.at(pos.0.z);
        for _ in 0..deck.width.max(deck.height) {
            let next = at.step(laser.dir);
            if map.is_solid(next) || map.tile(next).is_none() || dense.contains(&next) {
                break;
            }
            if people.contains(&next) {
                // Luna `laser_act`: every body part takes burns.
                for (p, mut body) in &mut mobs {
                    if p.0 == next {
                        for zone in Zone::ALL {
                            body.take_damage(zone, 0.0, laser.power as f32 * BEAM_BURN_PER_POWER);
                        }
                    }
                }
                break;
            }
            path.push(next);
            at = next;
        }
        let redraw = beams.0.get(&laser_entity).is_none_or(|(old, _)| *old != path);
        if redraw {
            if let Some((_, old)) = beams.0.remove(&laser_entity) {
                for e in old {
                    commands.entity(e).despawn();
                }
            }
            let state = match laser.dir {
                Dir::North => "north",
                Dir::East => "east",
                Dir::West => "west",
                _ => "south",
            };
            let drawn = path
                .iter()
                .map(|tile| {
                    commands
                        .spawn((
                            GridPos(*tile),
                            ids.alloc(),
                            Appearance::new("laser-beam", state),
                            Beam { power: laser.power },
                            LightSource(BEAM_LIGHT),
                        ))
                        .id()
                })
                .collect();
            beams.0.insert(laser_entity, (path, drawn));
        }
    }
    // Lasers switched off lose their beams.
    let gone: Vec<Entity> = beams.0.keys().filter(|e| !live.contains(e)).copied().collect();
    for laser in gone {
        if let Some((_, old)) = beams.0.remove(&laser) {
            for e in old {
                commands.entity(e).despawn();
            }
        }
    }
}

/// Keep beam tiles' power in step with their laser.
pub fn update_beam_power(beams: Res<Beams>, lasers: Query<&EngineLaser>, mut tiles: Query<&mut Beam>) {
    for (laser, (_, drawn)) in &beams.0 {
        let Ok(laser) = lasers.get(*laser) else { continue };
        for e in drawn {
            if let Ok(mut beam) = tiles.get_mut(*e)
                && beam.power != laser.power
            {
                beam.power = laser.power;
            }
        }
    }
}

/// Moles in one tile of standard air (Luna `MOLES_CELLSTANDARD`).
fn moles_cell_standard() -> f64 {
    ONE_ATMOSPHERE * TILE_VOLUME / (R * T20C)
}

/// One cycle of the crystal (Luna `supermatter/process`): it works on a quarter of the air
/// on its tile. Hot air damages it. Its power is the air's warmth plus twice the power of
/// every beam beside it, times how far oxygen outweighs nitrogen, times the air's
/// temperature over 0 °C; it heats the air by a fiftieth of that (up to 1500 K) and gives
/// off oxygen. It kills anyone within three tiles.
pub fn run_supermatter(
    time: Res<WorldTime>,
    atmos: Option<ResMut<StationAtmos>>,
    mut crystals: Query<(&GridPos, &mut Supermatter)>,
    beams: Query<(&GridPos, &Beam)>,
    mut mobs: Query<(&GridPos, &mut Body, &Vitals), With<Mob>>,
    mut alerts: ResMut<CoreAlerts>,
) {
    let Some(mut atmos) = atmos else { return };
    if !time.tick.0.is_multiple_of(MACHINE_TICKS) {
        return;
    }
    let now = time.deciseconds() as f64 / 10.0;
    for (pos, mut crystal) in &mut crystals {
        let tile_moles = atmos.0.air_at(pos.0).total_moles();
        let mut removed = atmos.0.remove_at(pos.0, GAS_EFFICIENCY * tile_moles);

        let previous = crystal.damage;
        crystal.damage =
            (crystal.damage + (removed.temperature - DAMAGE_TEMPERATURE) / DAMAGE_DIVISOR).max(0.0);
        if crystal.damage > 0.0
            && removed.temperature > DAMAGE_TEMPERATURE
            && crystal.last_warning.is_none_or(|t| now - t >= WARNING_SECONDS)
        {
            crystal.last_warning = Some(now);
            alerts.0.push(if EXPLOSION_DAMAGE - crystal.damage <= 300.0 {
                "core-explosion-imminent"
            } else if crystal.damage >= previous {
                "core-overload"
            } else {
                "core-safe"
            });
        }

        if removed.total_moles() > 0.0 {
            let mut power = ((removed.temperature - T0C) / 20.0).round().max(0.0);
            for (beam_pos, beam) in &beams {
                let d = (beam_pos.0.x - pos.0.x, beam_pos.0.y - pos.0.y);
                if beam_pos.0.z == pos.0.z && matches!(d, (0, 1) | (0, -1) | (1, 0) | (-1, 0)) {
                    power += f64::from(beam.power) * 2.0;
                }
            }
            let oxygen = ((removed.get(Gas::Oxygen) - removed.get(Gas::Nitrogen) * NITROGEN_RETARDATION)
                / moles_cell_standard())
            .clamp(0.0, 1.0);
            let energy = (oxygen * power * removed.temperature / T0C * REACTION_POWER).round();
            removed.temperature =
                (removed.temperature + (energy / THERMAL_RELEASE).max(0.0)).min(MAX_TEMPERATURE);
            let released = ((energy + removed.temperature - T0C) / OXYGEN_RELEASE)
                .round()
                .max(0.0);
            removed.set(Gas::Oxygen, removed.get(Gas::Oxygen) + released);
        }
        atmos.0.add_at(pos.0, &removed);

        for (mob_pos, mut body, vitals) in &mut mobs {
            let near = mob_pos.0.z == pos.0.z
                && (mob_pos.0.x - pos.0.x).abs() <= KILL_RANGE
                && (mob_pos.0.y - pos.0.y).abs() <= KILL_RANGE;
            if near && vitals.stat != MobStat::Dead {
                // Luna gibs them: every part is destroyed.
                for zone in Zone::ALL {
                    body.take_damage(zone, zone.max_damage(), 0.0);
                }
            }
        }
    }
}

/// The core computer's warnings waiting to go out on the common channel (Luna
/// `radioalert("Core control computer", ...)`): message ids.
#[derive(Resource, Debug, Default)]
pub struct CoreAlerts(pub Vec<&'static str>);

/// Play the core computer's warnings on every listening radio on the common channel, to
/// the mobs that can hear it.
pub fn broadcast_core_alerts(
    mut alerts: ResMut<CoreAlerts>,
    radios: Query<(&Radio, Option<&GridPos>, Option<&HeldBy>, Option<&WornBy>)>,
    mut listeners: Query<(Entity, &GridPos, &mut Outbox), With<Mob>>,
    map: crate::decks::Decks,
) {
    if alerts.0.is_empty() {
        return;
    }
    let at_of: HashMap<Entity, TilePos> = listeners.iter().map(|(e, p, _)| (e, p.0)).collect();
    let speakers: Vec<(TilePos, i32)> = radios
        .iter()
        .filter(|(r, ..)| r.listening && r.frequency == COMMON_FREQUENCY)
        .filter_map(|(r, pos, held, worn)| {
            let at = pos
                .map(|p| p.0)
                .or_else(|| held.and_then(|h| at_of.get(&h.0).copied()))
                .or_else(|| worn.and_then(|w| at_of.get(&w.0).copied()))?;
            Some((at, r.range))
        })
        .collect();
    for id in alerts.0.drain(..) {
        let msg = LocMsg::new(id).arg("frequency", format_frequency(COMMON_FREQUENCY));
        for (_, pos, mut outbox) in &mut listeners {
            if speakers
                .iter()
                .any(|(at, range)| crate::speech::can_see(map.at(at.z), &[], *at, pos.0, *range))
            {
                outbox.0.push(msg.clone());
            }
        }
    }
}

/// Clicking a working laser computer in reach opens its window. `None` when `target` is
/// no laser computer.
pub fn open_computer(
    computers: &Query<(&GridPos, &PowerUser), With<LaserComputer>>,
    mob: Entity,
    mob_pos: TilePos,
    target: Entity,
    commands: &mut Commands,
) -> Option<()> {
    let (pos, power) = computers.get(target).ok()?;
    if power.powered && in_reach(mob_pos, pos.0) {
        commands.entity(mob).insert(PanelViewer(target));
    }
    Some(())
}

/// Apply laser computer buttons (Luna `lasercon/Topic`): the computer must work and be in
/// reach. Power changes apply to every laser it controls, kept within 1–3000.
pub fn process_laser_ops(
    mut mobs: Query<(&GridPos, &mut PendingLaserOps, Option<&Vitals>)>,
    computers: Query<(&GridPos, &LaserComputer, &PowerUser)>,
    mut lasers: Query<&mut EngineLaser>,
) {
    for (pos, mut pending, vitals) in &mut mobs {
        let awake = vitals.is_none_or(|v| v.stat == MobStat::Conscious);
        for (target, op) in pending.0.drain(..) {
            let Ok((at, computer, power)) = computers.get(target) else {
                continue;
            };
            if !awake || !power.powered || !in_reach(pos.0, at.0) {
                continue;
            }
            for mut laser in &mut lasers {
                if !computer.ids.contains(&laser.id) {
                    continue;
                }
                match op {
                    LaserOp::Power(delta) => {
                        let (lo, hi) = LASER_POWER_RANGE;
                        laser.power = (laser.power as i32 + delta).clamp(lo, hi) as u32;
                    }
                    LaserOp::Toggle => laser.on = !laser.on,
                }
            }
        }
    }
}

/// Keep each viewer's laser computer window in step; close it out of reach or without power.
pub fn update_laser_views(
    mut viewers: Query<(
        Entity,
        &GridPos,
        Option<&PanelViewer>,
        &mut LaserPanelOut,
        Option<&Vitals>,
    )>,
    computers: Query<(&GridPos, &LaserComputer, &PowerUser, &NetId)>,
    lasers: Query<&EngineLaser>,
    mut commands: Commands,
) {
    for (mob, pos, viewer, mut out, vitals) in &mut viewers {
        let target = viewer.and_then(|v| computers.get(v.0).ok());
        let view = target.and_then(|(at, computer, power, id)| {
            let usable =
                power.powered && in_reach(pos.0, at.0) && vitals.is_none_or(|v| v.stat == MobStat::Conscious);
            if !usable {
                return None;
            }
            let mine: Vec<&EngineLaser> = lasers.iter().filter(|l| computer.ids.contains(&l.id)).collect();
            // Luna shows the first of its lasers.
            let first = computer
                .ids
                .iter()
                .find_map(|id| mine.iter().find(|l| &l.id == id))
                .copied();
            Some(LaserPanelView {
                target: *id,
                lasers: mine.len().min(255) as u8,
                power: first.map_or(0, |l| l.power),
                online: first.is_some_and(|l| l.on),
            })
        });
        if view.is_none() && target.is_some() {
            commands.entity(mob).remove::<PanelViewer>();
        }
        if out.0 != view {
            out.0 = view;
        }
    }
}

/// A laser computer without power goes dark.
pub fn update_computer_looks(mut computers: Query<(&PowerUser, &mut Appearance), With<LaserComputer>>) {
    for (power, mut look) in &mut computers {
        let state = if power.powered { "atmos" } else { "unpowered" };
        if look.state != state {
            look.state = state.to_owned();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byond_lists_of_ids_are_read() {
        assert_eq!(
            id_list(r#"list("north","west","east","south")"#),
            ["north", "west", "east", "south"]
        );
        assert_eq!(id_list("north"), ["north"]);
        assert!(id_list("").is_empty());
    }
}
