//! The station's shield generator (Luna's `shielding`): capacitors charged by energy
//! converters feed emitter plates under the decks, which raise a field over the space just
//! outside the hull. Spec: `docs/reference/shields.md`.

use std::collections::{HashMap, HashSet};

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, Dense, Dir, GridPos, Hidden, NetId, TileMap, TilePos};

use crate::actions::Outbox;
use crate::machines::MACHINE_TICKS;
use crate::power::{Channel, PowerUser};
use crate::time::WorldTime;

pub const PLATE: &str = "shield-plate";
pub const SHIELD: &str = "shield";
pub const CAPACITOR: &str = "shield-capacitor";
pub const CONVERTER: &str = "shield-converter";
pub const BUTTON: &str = "shields-button";

/// How far a plate reaches, in tiles (Luna `range_dist`).
pub const PLATE_RANGE: i32 = 10;
/// Capacitors that must hold charge for the shields to run (Luna `HasPower`).
pub const CAPACITORS_NEEDED: usize = 3;
/// A capacitor's charge when full, and at the start of the round.
pub const MAX_CHARGE: f64 = 10_000_000.0;
pub const START_CHARGE: f64 = 1_000_000.0;
/// What running shields take from each capacitor every cycle.
pub const CHARGE_PER_CYCLE: f64 = 80_000.0;
/// A converter's rates: manual, the most it can do, and how fast auto may climb.
pub const MANUAL_RATE: f64 = 60.0;
pub const MAX_RATE: f64 = 500.0;
pub const AUTO_RATE_STEP: f64 = 50.0;
/// Auto mode's target charge, in percent.
pub const AUTO_TARGET: f64 = 75.0;
/// The most a converter draws from the station, W.
pub const MAX_DRAW: f64 = 100_000.0;

/// Whether someone has asked for the shields, and whether they are up now.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ShieldNetwork {
    pub enabled: bool,
    pub up: bool,
}

/// An emitter plate, hidden under the floor.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShieldPlate;

/// A tile of the field: the edges it seals against the air inside.
#[derive(Component, Debug, Clone, PartialEq, Eq, Default)]
pub struct ShieldTile {
    pub edges: Vec<Dir>,
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ShieldCapacitor {
    pub charge: f64,
}

/// What a converter is doing (Luna `OperatingMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConverterMode {
    Off,
    Manual,
    Auto,
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ShieldConverter {
    pub mode: ConverterMode,
    pub rate: f64,
}

/// The button that raises and lowers the shields.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShieldsButton;

pub fn is_shield_part(proto: &str) -> bool {
    matches!(proto, PLATE | CAPACITOR | CONVERTER | BUTTON)
}

pub fn spawn_shield_part(world: &mut World, proto: &str, pos: TilePos, id: NetId) -> Entity {
    match proto {
        PLATE => world
            .spawn((GridPos(pos), id, Appearance::new(PLATE, "plate-p"), ShieldPlate))
            .id(),
        CAPACITOR => world
            .spawn((
                GridPos(pos),
                id,
                Appearance::new(CAPACITOR, capacitor_look(START_CHARGE, true)),
                Dense,
                ShieldCapacitor { charge: START_CHARGE },
            ))
            .id(),
        CONVERTER => {
            let converter = ShieldConverter {
                mode: ConverterMode::Manual,
                rate: MANUAL_RATE,
            };
            world
                .spawn((
                    GridPos(pos),
                    id,
                    Appearance::new(CONVERTER, converter_look(&converter, true)),
                    Dense,
                    converter,
                    PowerUser::new(Channel::Equipment, 0.0),
                ))
                .id()
        }
        _ => world
            .spawn((
                GridPos(pos),
                id,
                Appearance::new(BUTTON, "access_button_standby"),
                ShieldsButton,
            ))
            .id(),
    }
}

/// Luna `cap` and its charge lights; `cap-o` while its converter isn't running.
fn capacitor_look(charge: f64, converter_running: bool) -> String {
    let level = (charge * 5.0 / MAX_CHARGE).round() as u32;
    if converter_running {
        format!("cap+c{level}")
    } else {
        format!("cap+c{level}+cap-o")
    }
}

/// Luna `econ`, with lights for its rate; `econ-p` without power.
fn converter_look(converter: &ShieldConverter, powered: bool) -> String {
    if !powered {
        return "econ-p".to_owned();
    }
    if converter.mode == ConverterMode::Off {
        return "econ".to_owned();
    }
    format!("econ+o{}", (converter.rate * 11.0 / MAX_RATE).round() as u32)
}

/// Whether a space tile gets a field: anything but space next to it (Luna `UpdateTurf`).
fn borders_hull(map: &TileMap, at: TilePos) -> bool {
    [
        Dir::North,
        Dir::South,
        Dir::East,
        Dir::West,
        Dir::NorthEast,
        Dir::NorthWest,
        Dir::SouthEast,
        Dir::SouthWest,
    ]
    .into_iter()
    .any(|d| map.tile(at.step(d)).is_some_and(|t| t.id != "space"))
}

/// Lay the field's tiles, down until raised, on the hull within reach of a plate. Returns the
/// edges they seal, for the air to know about.
pub fn lay_field(world: &mut World, decks: &[TileMap]) -> Vec<(TilePos, Dir)> {
    let mut plates = world.query_filtered::<&GridPos, With<ShieldPlate>>();
    let plates: Vec<TilePos> = plates.iter(world).map(|p| p.0).collect();
    let mut tiles: HashSet<TilePos> = HashSet::new();
    for plate in plates {
        let Some(map) = decks.iter().find(|m| m.z == plate.z) else {
            continue;
        };
        for y in plate.y - PLATE_RANGE..=plate.y + PLATE_RANGE {
            for x in plate.x - PLATE_RANGE..=plate.x + PLATE_RANGE {
                let at = TilePos::new(x, y, plate.z);
                if map.tile(at).is_some_and(|t| t.id == "space") && borders_hull(map, at) {
                    tiles.insert(at);
                }
            }
        }
    }
    let mut tiles: Vec<TilePos> = tiles.into_iter().collect();
    tiles.sort_by_key(|p| (p.z, p.y, p.x));
    let mut all_edges = Vec::new();
    for at in tiles {
        let Some(map) = decks.iter().find(|m| m.z == at.z) else {
            continue;
        };
        // Only where air could reach it: walls need no sealing.
        let edges: Vec<Dir> = [Dir::North, Dir::South, Dir::East, Dir::West]
            .into_iter()
            .filter(|d| {
                map.tile(at.step(*d))
                    .is_some_and(|t| t.id != "space" && !t.is_airtight())
            })
            .collect();
        all_edges.extend(edges.iter().map(|d| (at, *d)));
        let id = world.resource_mut::<crate::objects::NetIds>().alloc();
        world.spawn((
            GridPos(at),
            id,
            Appearance::new(SHIELD, "shieldsparkles0"),
            Hidden,
            ShieldTile { edges },
        ));
    }
    all_edges
}

/// Each cycle: converters charge their capacitors, running shields drain them, and the field
/// goes up or down with the charge (Luna `energyconverter/process`, `capacitor/process`,
/// `emitter/process`).
#[allow(clippy::type_complexity)]
pub fn run_shields(
    time: Res<WorldTime>,
    mut network: ResMut<ShieldNetwork>,
    mut converters: Query<
        (&GridPos, &mut ShieldConverter, &mut PowerUser, &mut Appearance),
        Without<ShieldCapacitor>,
    >,
    mut capacitors: Query<(Entity, &GridPos, &mut ShieldCapacitor, &mut Appearance)>,
    mut plates: Query<
        &mut Appearance,
        (
            With<ShieldPlate>,
            Without<ShieldCapacitor>,
            Without<ShieldConverter>,
        ),
    >,
    tiles: Query<Entity, With<ShieldTile>>,
    mut commands: Commands,
) {
    if !time.tick.0.is_multiple_of(MACHINE_TICKS) {
        return;
    }
    let at: HashMap<TilePos, Entity> = capacitors.iter().map(|(e, p, ..)| (p.0, e)).collect();
    let mut linked: HashMap<Entity, bool> = HashMap::new();
    for (pos, mut converter, mut user, mut look) in &mut converters {
        // A converter feeds the capacitor to its west.
        let capacitor = at.get(&pos.0.step(Dir::West)).copied();
        if let Some(capacitor) = capacitor {
            linked.insert(capacitor, converter.mode != ConverterMode::Off && user.powered);
        }
        let state = converter_look(&converter, user.powered);
        if look.state != state {
            look.state = state;
        }
        let Some(Ok((.., mut store, _))) = capacitor.map(|c| capacitors.get_mut(c)) else {
            user.watts = 0.0;
            continue;
        };
        if converter.mode == ConverterMode::Off || store.charge >= MAX_CHARGE {
            user.watts = 0.0;
            continue;
        }
        converter.rate = match converter.mode {
            ConverterMode::Auto => {
                let filled = store.charge / MAX_CHARGE;
                let needed = (AUTO_TARGET - filled).max(0.0) * MAX_CHARGE;
                (converter.rate + AUTO_RATE_STEP).max(needed).min(MAX_RATE)
            }
            _ => MANUAL_RATE,
        };
        let energy = converter.rate.powf(2.15).round();
        user.watts = energy.min(MAX_DRAW);
        if user.powered {
            store.charge = (store.charge + energy).min(MAX_CHARGE);
        }
    }

    let mut charged = 0;
    for (entity, _, mut store, mut look) in &mut capacitors {
        if network.up {
            store.charge = (store.charge - CHARGE_PER_CYCLE).max(0.0);
        }
        let Some(running) = linked.get(&entity) else {
            continue;
        };
        if store.charge > 0.0 {
            charged += 1;
        }
        let state = capacitor_look(store.charge, *running);
        if look.state != state {
            look.state = state;
        }
    }

    let up = network.enabled && charged >= CAPACITORS_NEEDED;
    if network.up == up {
        return;
    }
    network.up = up;
    for mut look in &mut plates {
        look.state = if up { "plate" } else { "plate-p" }.to_owned();
    }
    for tile in &tiles {
        if up {
            commands.entity(tile).insert(Dense).remove::<Hidden>();
        } else {
            commands.entity(tile).remove::<Dense>().insert(Hidden);
        }
    }
}

/// A click on the button or a converter (Luna `shieldsbutton/attack_hand`; a converter's
/// window is only its mode here, `shields.md`). Hands are all it takes.
pub fn click(world: &mut World, mob: Entity, target: Entity) {
    let message = if world.get::<ShieldsButton>(target).is_some() {
        let mut capacitors = world.query::<&ShieldCapacitor>();
        let charged = capacitors.iter(world).filter(|c| c.charge > 0.0).count();
        let mut network = world.resource_mut::<ShieldNetwork>();
        if network.enabled {
            network.enabled = false;
            LocMsg::new("shields-lowered")
        } else if charged >= CAPACITORS_NEEDED {
            network.enabled = true;
            LocMsg::new("shields-raised")
        } else {
            LocMsg::new("shields-no-charge")
        }
    } else if let Some(mut converter) = world.get_mut::<ShieldConverter>(target) {
        converter.mode = match converter.mode {
            ConverterMode::Off => ConverterMode::Manual,
            ConverterMode::Manual => ConverterMode::Auto,
            ConverterMode::Auto => ConverterMode::Off,
        };
        LocMsg::new("shield-converter-mode").arg(
            "mode",
            match converter.mode {
                ConverterMode::Off => "off",
                ConverterMode::Manual => "manual",
                ConverterMode::Auto => "auto",
            },
        )
    } else {
        return;
    };
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(message);
    }
}

/// What looking at a capacitor tells.
pub fn examine(world: &World, target: Entity) -> Option<LocMsg> {
    let capacitor = world.get::<ShieldCapacitor>(target)?;
    let percent = (capacitor.charge * 100.0 / MAX_CHARGE).round() as i64;
    Some(LocMsg::new("shield-capacitor-charge").num("value", percent))
}
