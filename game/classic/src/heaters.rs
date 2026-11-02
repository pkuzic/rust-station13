//! Space heaters: cell-powered heaters that warm the air of their tile to a set temperature.
//! Spec: `docs/reference/space-heaters.md`.

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_i18n::LocMsg;
use rs13_simulation::atmos::gas::T0C;
use rs13_world::{Appearance, Dense, GridPos, HeaterPanelView, MobStat, NetId, ObjectSpec, TilePos};

use crate::actions::in_reach;
use crate::atmos::StationAtmos;
use crate::cells::{JOULES_PER_UNIT, PowerCell};
use crate::hands::{Hands, HeldBy};
use crate::health::Vitals;
use crate::machines::MACHINE_TICKS;
use crate::wires::PanelViewer;

pub const SPACE_HEATER: &str = "space-heater";
/// Heat put into the air each machine cycle while heating, J (Luna `heating_power`).
pub const HEATING_POWER: f64 = 100_000.0;
/// Share of the tile's air heated at a time (Luna `0.25 * env.total_moles()`).
pub const HEATED_SHARE: f64 = 0.25;
/// Cell drawn each heating cycle, J (Luna `cell.use(heating_power / 20000)`: 5 units).
pub const CELL_USE: f64 = HEATING_POWER / 20_000.0 * JOULES_PER_UNIT;
/// The temperature it heats to: Luna's default, and how far it is set (°C).
pub const DEFAULT_SETTING: i32 = 50;
pub const COOLEST: i32 = 15;
pub const WARMEST: i32 = 90;
pub const SETTING_STEP: i32 = 5;

/// A space heater: its cell, whether it is on and its hatch open, and the temperature it
/// heats to, °C.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct SpaceHeater {
    pub cell: Option<Entity>,
    pub on: bool,
    pub open: bool,
    pub setting: i32,
}

/// On a cell inside a space heater: which one.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InHeater(pub Entity);

/// What the mob's open heater window shows, for the server to send when it changes.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct HeaterPanelOut(pub Option<HeaterPanelView>);

/// Heater window buttons, waiting for the next tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaterOp {
    /// Take the cell out into the empty hand, or put the one in hand in.
    Cell,
    /// Change the setting by this many steps.
    Setting(i32),
}

#[derive(Component, Debug, Default)]
pub struct PendingHeaterOps(pub Vec<(Entity, HeaterOp)>);

/// A heater with a full standard cell inside (Luna `New`).
pub fn spawn_heater(world: &mut World, _spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    let heater = world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(SPACE_HEATER, "sheater0"),
            Dense,
            SpaceHeater {
                cell: None,
                on: false,
                open: false,
                setting: DEFAULT_SETTING,
            },
        ))
        .id();
    if let Ok(cell) = crate::objects::spawn_object(world, &ObjectSpec::new("power-cell"), pos) {
        world
            .entity_mut(cell)
            .remove::<GridPos>()
            .insert(InHeater(heater));
        if let Some(mut h) = world.get_mut::<SpaceHeater>(heater) {
            h.cell = Some(cell);
        }
    }
    heater
}

/// Everything clicks on space heaters need.
#[derive(SystemParam)]
pub struct HeaterWork<'w, 's> {
    heaters: Query<'w, 's, &'static mut SpaceHeater>,
    cells: Query<'w, 's, &'static PowerCell>,
}

/// What a click on a heater did: a message for the mob, and one for everyone else seeing it
/// (to take the mob's name).
pub struct HeaterOutcome {
    pub to_mob: LocMsg,
    pub seen: Option<LocMsg>,
}

impl HeaterWork<'_, '_> {
    pub fn contains(&self, target: Entity) -> bool {
        self.heaters.contains(target)
    }

    /// Luna `space_heater/attackby` and `attack_hand`: a cell goes in through the open
    /// hatch; a screwdriver opens or shuts the hatch; an empty hand opens the window
    /// through the open hatch, or switches it on or off. `None` for anything else. The
    /// caller checks reach.
    pub fn use_on(
        &mut self,
        mob: Entity,
        hands: &mut Hands,
        target: Entity,
        screwdriver: bool,
        commands: &mut Commands,
    ) -> Option<Option<HeaterOutcome>> {
        let active = hands.active_item();
        let is_cell = active.is_some_and(|i| self.cells.contains(i));
        let mut heater = self.heaters.get_mut(target).ok()?;
        if let Some(item) = active {
            if is_cell {
                if !heater.open {
                    return Some(Some(told("heater-hatch-closed")));
                }
                if heater.cell.is_some() {
                    return Some(Some(told("heater-has-cell")));
                }
                insert_cell(&mut heater, target, item, hands, commands);
                return Some(Some(seen("heater-cell-in")));
            }
            if screwdriver {
                heater.open = !heater.open;
                if !heater.open {
                    commands.entity(mob).remove::<PanelViewer>();
                }
                let id = if heater.open {
                    "heater-hatch-opened"
                } else {
                    "heater-hatch-shut"
                };
                return Some(Some(seen(id)));
            }
            return None;
        }
        if heater.open {
            commands.entity(mob).insert(PanelViewer(target));
            return Some(None);
        }
        heater.on = !heater.on;
        let id = if heater.on {
            "heater-switched-on"
        } else {
            "heater-switched-off"
        };
        Some(Some(seen(id)))
    }

    /// Luna `space_heater/examine`.
    pub fn examine(&self, target: Entity) -> Option<LocMsg> {
        let heater = self.heaters.get(target).ok()?;
        let msg = LocMsg::new("examine-heater")
            .arg("on", if heater.on { "yes" } else { "no" })
            .arg("open", if heater.open { "yes" } else { "no" });
        Some(if heater.open {
            msg.arg("cell", if heater.cell.is_some() { "yes" } else { "no" })
        } else {
            let percent = heater
                .cell
                .and_then(|c| self.cells.get(c).ok())
                .map_or(0, |c| c.percent().round() as i64);
            msg.arg("cell", "meter").num("value", percent)
        })
    }
}

fn told(id: &str) -> HeaterOutcome {
    HeaterOutcome {
        to_mob: LocMsg::new(id),
        seen: None,
    }
}

/// "You …" for the mob, "<name> …" for those who see it (the `-seen` message).
fn seen(id: &str) -> HeaterOutcome {
    HeaterOutcome {
        to_mob: LocMsg::new(id),
        seen: Some(LocMsg::new(format!("{id}-seen"))),
    }
}

fn insert_cell(
    heater: &mut SpaceHeater,
    target: Entity,
    cell: Entity,
    hands: &mut Hands,
    commands: &mut Commands,
) {
    let active = hands.active;
    *hands.slot_mut(active) = None;
    commands.entity(cell).remove::<HeldBy>().insert(InHeater(target));
    heater.cell = Some(cell);
}

/// Luna `space_heater/Topic`: the cell taken out into an empty hand or put in from the hand
/// while the hatch is open, and the setting changed 5 °C at a time between 15 and 90.
#[allow(clippy::type_complexity)]
pub fn process_heater_ops(
    mut mobs: Query<(
        Entity,
        &GridPos,
        &mut Hands,
        &mut PendingHeaterOps,
        &mut crate::actions::Outbox,
        Option<&Vitals>,
        Option<&rs13_world::DisplayName>,
    )>,
    mut heaters: Query<(&GridPos, &mut SpaceHeater)>,
    cells: Query<(), With<PowerCell>>,
    mut visible: ResMut<crate::combat::VisibleEvents>,
    mut commands: Commands,
) {
    for (mob, pos, mut hands, mut pending, mut outbox, vitals, name) in &mut mobs {
        let awake = vitals.is_none_or(|v| v.stat == MobStat::Conscious);
        for (target, op) in std::mem::take(&mut pending.0) {
            let Ok((at, mut heater)) = heaters.get_mut(target) else {
                continue;
            };
            if !awake || !in_reach(pos.0, at.0) {
                continue;
            }
            match op {
                HeaterOp::Setting(steps) => {
                    heater.setting = (heater.setting + steps * SETTING_STEP).clamp(COOLEST, WARMEST);
                }
                HeaterOp::Cell if heater.open => {
                    let active = hands.active;
                    let id = match (heater.cell, hands.active_item()) {
                        (Some(cell), None) => {
                            heater.cell = None;
                            *hands.slot_mut(active) = Some(cell);
                            commands.entity(cell).remove::<InHeater>().insert(HeldBy(mob));
                            "heater-cell-out"
                        }
                        (None, Some(item)) if cells.contains(item) => {
                            insert_cell(&mut heater, target, item, &mut hands, &mut commands);
                            "heater-cell-in"
                        }
                        _ => continue,
                    };
                    outbox.0.push(LocMsg::new(id));
                    let name = name.map(|n| n.0.clone()).unwrap_or_default();
                    visible.0.push((
                        pos.0,
                        LocMsg::new(format!("{id}-seen")).arg("name", name),
                        Some(mob),
                    ));
                }
                HeaterOp::Cell => {}
            }
        }
    }
}

/// Each machine cycle, a heater that is on and has charge warms a quarter of its tile's
/// air by 100 kJ while the tile is colder than its setting, using 5 units of its cell; with
/// no cell or an empty one it switches off (Luna `process`).
pub fn run_heaters(
    time: Res<crate::time::WorldTime>,
    atmos: Option<ResMut<StationAtmos>>,
    mut heaters: Query<(&GridPos, &mut SpaceHeater)>,
    mut cells: Query<&mut PowerCell>,
) {
    let Some(mut atmos) = atmos else { return };
    if !time.tick.0.is_multiple_of(MACHINE_TICKS) {
        return;
    }
    for (at, mut heater) in &mut heaters {
        if !heater.on {
            continue;
        }
        let Some(mut cell) = heater
            .cell
            .and_then(|c| cells.get_mut(c).ok())
            .filter(|c| c.charge > 0.0)
        else {
            heater.on = false;
            continue;
        };
        let air = atmos.0.air_at(at.0);
        if air.temperature >= f64::from(heater.setting) + T0C {
            continue;
        }
        let mut part = atmos.0.remove_at(at.0, HEATED_SHARE * air.total_moles());
        let capacity = part.heat_capacity();
        if capacity > 0.0 {
            part.temperature = (part.temperature * capacity + HEATING_POWER) / capacity;
            cell.charge = (cell.charge - CELL_USE).max(0.0);
        }
        atmos.0.add_at(at.0, &part);
    }
}

/// Keep each viewer's heater window in step; close it once out of reach or the hatch shuts.
pub fn update_heater_views(
    mut viewers: Query<(
        Entity,
        &GridPos,
        Option<&PanelViewer>,
        &mut HeaterPanelOut,
        Option<&Vitals>,
    )>,
    heaters: Query<(&GridPos, &SpaceHeater, &NetId)>,
    cells: Query<&PowerCell>,
    mut commands: Commands,
) {
    for (mob, pos, viewer, mut out, vitals) in &mut viewers {
        let target = viewer.and_then(|v| heaters.get(v.0).ok());
        let view = target.and_then(|(at, heater, id)| {
            let usable =
                heater.open && in_reach(pos.0, at.0) && vitals.is_none_or(|v| v.stat == MobStat::Conscious);
            if !usable {
                return None;
            }
            Some(HeaterPanelView {
                target: *id,
                has_cell: heater.cell.is_some(),
                charge_percent: heater
                    .cell
                    .and_then(|c| cells.get(c).ok())
                    .map_or(0, |c| c.percent().round().clamp(0.0, 100.0) as u8),
                setting: heater.setting as i16,
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

/// Luna `update_icon`: `sheater-open` with the hatch open, else `sheater1` on, `sheater0`
/// off.
pub fn update_heater_looks(mut heaters: Query<(&SpaceHeater, &mut Appearance), Changed<SpaceHeater>>) {
    for (heater, mut look) in &mut heaters {
        let state = match (heater.open, heater.on) {
            (true, _) => "sheater-open",
            (false, true) => "sheater1",
            (false, false) => "sheater0",
        };
        if look.state != state {
            look.state = state.to_owned();
        }
    }
}
