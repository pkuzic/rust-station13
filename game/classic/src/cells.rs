//! Power cells and cell chargers. Spec: `docs/reference/cells.md`.

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, GridPos, NetId, ObjectSpec, TilePos};

use crate::actions::ClickOutcome;
use crate::hands::{Hands, HeldBy};
use crate::machines::MACHINE_TICKS;
use crate::power::{Channel, PowerUser};
use crate::time::WorldTime;

/// The charger prototype.
pub const CELL_CHARGER: &str = "cell-charger";
/// Joules in one of Luna's cell units (the APC cell's 2 500 units are 2.5 MJ, `power.md`).
pub const JOULES_PER_UNIT: f64 = 1_000.0;
/// What a charger puts into its cell each machine cycle, J (Luna `give(175)`).
pub const CHARGE_PER_CYCLE: f64 = 175.0 * JOULES_PER_UNIT;
/// What a charging charger draws from its area's equipment channel, W (Luna
/// `use_power(200)`).
pub const CHARGER_POWER: f64 = 200.0;

/// A rechargeable power cell (Luna `item/weapon/cell`), in joules.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct PowerCell {
    pub charge: f64,
    pub capacity: f64,
}

impl PowerCell {
    pub fn percent(&self) -> f64 {
        100.0 * self.charge / self.capacity
    }

    /// Luna `update_icon`: a full meter at 99.5%, a part-full one above nothing.
    pub fn look(&self) -> &'static str {
        if self.charge < 0.01 * JOULES_PER_UNIT {
            ""
        } else if self.charge / self.capacity >= 0.995 {
            "+cell-o2"
        } else {
            "+cell-o1"
        }
    }
}

/// The cell an item prototype is, full (Luna `maxcharge`: 2 500 or 10 000 units).
pub fn kind(proto: &str) -> Option<PowerCell> {
    let units = match proto {
        "power-cell" => 2_500.0,
        "high-power-cell" => 10_000.0,
        _ => return None,
    };
    let capacity = units * JOULES_PER_UNIT;
    Some(PowerCell {
        charge: capacity,
        capacity,
    })
}

/// A cell charger (Luna `machinery/cell_charger`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellCharger {
    pub charging: Option<Entity>,
    pub anchored: bool,
}

/// On a cell sitting in a charger: which one.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InCharger(pub Entity);

pub fn spawn_cell_charger(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(spec.id.as_str(), "ccharger0"),
            CellCharger {
                charging: None,
                anchored: true,
            },
            PowerUser::new(Channel::Equipment, 0.0),
        ))
        .id()
}

/// Everything clicking a charger needs.
#[derive(SystemParam)]
pub struct ChargerWork<'w, 's> {
    chargers: Query<'w, 's, (&'static mut CellCharger, &'static PowerUser)>,
    cells: Query<'w, 's, &'static PowerCell>,
}

impl ChargerWork<'_, '_> {
    pub fn contains(&self, target: Entity) -> bool {
        self.chargers.contains(target)
    }

    /// Luna `attackby` and `attack_hand`. A cell goes in a secured charger in a powered
    /// area; an empty hand takes it out; a wrench secures or frees an empty charger. The
    /// caller checks reach; `seen` messages want the clicker's `name`.
    pub fn use_on(
        &mut self,
        mob: Entity,
        target: Entity,
        held: Option<&str>,
        hands: &mut Hands,
        commands: &mut Commands,
    ) -> Option<ClickOutcome> {
        let (mut charger, power) = self.chargers.get_mut(target).ok()?;
        let active = hands.active_item();
        Some(match (held, active) {
            (Some("wrench"), _) if charger.charging.is_some() => {
                ClickOutcome::told("charger-remove-cell-first")
            }
            (Some("wrench"), _) => {
                charger.anchored = !charger.anchored;
                ClickOutcome::told(if charger.anchored {
                    "charger-attached"
                } else {
                    "charger-detached"
                })
            }
            (Some(_), Some(cell)) if self.cells.contains(cell) && charger.anchored => {
                if charger.charging.is_some() {
                    ClickOutcome::told("charger-occupied")
                } else if !power.powered {
                    ClickOutcome::told("charger-blinks")
                } else {
                    charger.charging = Some(cell);
                    commands.entity(cell).remove::<HeldBy>().insert(InCharger(target));
                    *hands.slot_mut(hands.active) = None;
                    ClickOutcome {
                        to_mob: Some(LocMsg::new("charger-inserted-self")),
                        seen: Some(LocMsg::new("charger-inserted")),
                    }
                }
            }
            (None, None) => {
                let cell = charger.charging.take()?;
                commands.entity(cell).remove::<InCharger>().insert(HeldBy(mob));
                *hands.slot_mut(hands.active) = Some(cell);
                ClickOutcome {
                    to_mob: Some(LocMsg::new("charger-removed-self")),
                    seen: Some(LocMsg::new("charger-removed")),
                }
            }
            _ => return None,
        })
    }

    /// What examining a charger or a cell adds (Luna `examine`).
    pub fn examine(&self, target: Entity) -> Option<LocMsg> {
        if let Ok(cell) = self.cells.get(target) {
            return Some(
                LocMsg::new("examine-cell")
                    .num("rating", (cell.capacity / JOULES_PER_UNIT).round() as i64)
                    .num("value", cell.percent().round() as i64),
            );
        }
        let (charger, _) = self.chargers.get(target).ok()?;
        let charge = charger
            .charging
            .and_then(|c| self.cells.get(c).ok())
            .map_or(-1, |c| (c.charge / JOULES_PER_UNIT).round() as i64);
        Some(LocMsg::new("examine-cell-charger").num("charge", charge))
    }
}

/// Every machine cycle, a secured, powered charger with a cell charges it and draws power
/// for it (Luna `process`). Every tick its sprite shows the cell and its level.
pub fn run_chargers(
    time: Res<WorldTime>,
    mut chargers: Query<(&CellCharger, &mut PowerUser, &mut Appearance), Without<PowerCell>>,
    mut cells: Query<&mut PowerCell>,
) {
    let cycle = time.tick.0.is_multiple_of(MACHINE_TICKS);
    for (charger, mut power, mut look) in &mut chargers {
        let working = charger.anchored && power.powered && charger.charging.is_some();
        let watts = if working { CHARGER_POWER } else { 0.0 };
        if power.watts != watts {
            power.watts = watts;
        }
        let cell = charger.charging.and_then(|c| cells.get_mut(c).ok());
        let mut state = if cell.is_some() { "ccharger1" } else { "ccharger0" }.to_owned();
        if let Some(mut cell) = cell {
            if cycle && working {
                cell.charge = (cell.charge + CHARGE_PER_CYCLE).min(cell.capacity);
            }
            // Luna: round(percent × 4 / 99), rounding down, while it has power.
            if power.powered {
                let level = (cell.percent() * 4.0 / 99.0).floor() as i64;
                state.push_str(&format!("+ccharger-o{}", level.clamp(0, 4)));
            }
        }
        if look.state != state {
            look.state = state;
        }
    }
}

/// A cell's meter shows its charge.
pub fn update_cell_looks(mut cells: Query<(&PowerCell, &mut Appearance), Changed<PowerCell>>) {
    for (cell, mut look) in &mut cells {
        let state = cell.look();
        if look.state != state {
            look.state = state.to_owned();
        }
    }
}
