//! Emergency floodlights: a lamp on a stand that lights ten tiles around it from its own
//! power cell. Spec: `docs/reference/floodlights.md`.

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_world::{Appearance, Dense, GridPos, LightSource, NetId, ObjectSpec, TilePos};

use crate::actions::ClickOutcome;
use crate::cells::{JOULES_PER_UNIT, PowerCell};
use crate::hands::{Hands, HeldBy};
use crate::machines::MACHINE_TICKS;
use crate::time::WorldTime;

/// The floodlight prototype.
pub const FLOODLIGHT: &str = "floodlight";
/// What a lit floodlight takes from its cell each machine cycle, J (Luna `use = 1`).
pub const FLOODLIGHT_USE: f64 = JOULES_PER_UNIT;
/// The cell it comes with, J (Luna `maxcharge = 2500`).
pub const FLOODLIGHT_CELL: f64 = 2_500.0 * JOULES_PER_UNIT;
/// Its light (Luna `ul_SetLuminosity(10)`).
pub const FLOODLIGHT_LIGHT: [u8; 3] = [10, 10, 10];

/// A floodlight: whether it is on, its cell, and its battery panel.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Floodlight {
    pub on: bool,
    pub cell: Option<Entity>,
    /// The panel's screws are out (Luna `unlocked`).
    pub unlocked: bool,
    /// The panel is off (Luna `open`).
    pub open: bool,
}

/// On a cell inside a floodlight: which one.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InFloodlight(pub Entity);

/// A floodlight with a full cell of its own, off and shut.
pub fn spawn_floodlight(world: &mut World, pos: TilePos, id: NetId) -> Entity {
    let light = world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(FLOODLIGHT, "flood00"),
            Floodlight {
                on: false,
                cell: None,
                unlocked: false,
                open: false,
            },
            Dense,
        ))
        .id();
    let cell = crate::objects::spawn_object(world, &ObjectSpec::new("power-cell"), pos)
        .expect("the power cell prototype is known");
    world.entity_mut(cell).remove::<GridPos>().insert((
        InFloodlight(light),
        PowerCell {
            charge: FLOODLIGHT_CELL,
            capacity: FLOODLIGHT_CELL,
        },
    ));
    if let Some(mut flood) = world.get_mut::<Floodlight>(light) {
        flood.cell = Some(cell);
    }
    light
}

/// `flood` + `o` with its panel off + `b` with a cell showing + `0` + `1` while lit (Luna
/// `update_icon`).
pub fn floodlight_look(flood: &Floodlight) -> String {
    format!(
        "flood{}{}0{}",
        if flood.open { "o" } else { "" },
        if flood.open && flood.cell.is_some() {
            "b"
        } else {
            ""
        },
        u8::from(flood.on)
    )
}

/// Everything clicking a floodlight needs.
#[derive(SystemParam)]
pub struct FloodlightWork<'w, 's> {
    lights: Query<'w, 's, &'static mut Floodlight>,
    cells: Query<'w, 's, &'static PowerCell>,
}

impl FloodlightWork<'_, '_> {
    pub fn contains(&self, target: Entity) -> bool {
        self.lights.contains(target)
    }

    /// Luna `attack_hand` and `attackby`. With the panel off, an empty hand takes the cell
    /// out; otherwise it switches the light, on only with a charged cell. A screwdriver
    /// undoes or does up the panel's screws while it is on; a crowbar takes the unscrewed
    /// panel off or puts it back; a cell goes in while the panel is off. The caller checks
    /// reach.
    pub fn use_on(
        &mut self,
        mob: Entity,
        target: Entity,
        held: Option<&str>,
        hands: &mut Hands,
        commands: &mut Commands,
    ) -> Option<ClickOutcome> {
        let mut flood = self.lights.get_mut(target).ok()?;
        let active = hands.active_item();
        Some(match (held, active) {
            (None, None) if flood.open && flood.cell.is_some() => {
                let cell = flood.cell.take()?;
                flood.on = false;
                commands.entity(cell).remove::<InFloodlight>().insert(HeldBy(mob));
                *hands.slot_mut(hands.active) = Some(cell);
                ClickOutcome::told("floodlight-cell-removed")
            }
            (None, None) if flood.on => {
                flood.on = false;
                ClickOutcome::told("floodlight-off")
            }
            (None, None) => {
                let charged = flood
                    .cell
                    .and_then(|c| self.cells.get(c).ok())
                    .is_some_and(|c| c.charge > 0.0);
                if !charged {
                    return Some(ClickOutcome::default());
                }
                flood.on = true;
                ClickOutcome::told("floodlight-on")
            }
            (Some("screwdriver"), _) if !flood.open => {
                flood.unlocked = !flood.unlocked;
                ClickOutcome::told(if flood.unlocked {
                    "floodlight-unscrewed"
                } else {
                    "floodlight-screwed"
                })
            }
            (Some("crowbar"), _) if flood.unlocked => {
                flood.open = !flood.open;
                ClickOutcome::told(if flood.open {
                    "floodlight-panel-removed"
                } else {
                    "floodlight-panel-replaced"
                })
            }
            (Some(_), Some(cell)) if self.cells.contains(cell) && flood.open => {
                if flood.cell.is_some() {
                    ClickOutcome::told("floodlight-cell-already")
                } else {
                    flood.cell = Some(cell);
                    commands
                        .entity(cell)
                        .remove::<HeldBy>()
                        .insert(InFloodlight(target));
                    *hands.slot_mut(hands.active) = None;
                    ClickOutcome::told("floodlight-cell-inserted")
                }
            }
            _ => ClickOutcome::default(),
        })
    }
}

/// Every machine cycle a lit floodlight takes its share from its cell and goes out when the
/// cell is flat or gone (Luna `process`). Every tick it shows its state, and gives light
/// while lit.
pub fn run_floodlights(
    time: Res<WorldTime>,
    mut commands: Commands,
    mut lights: Query<(Entity, &mut Floodlight, &mut Appearance, Has<LightSource>)>,
    mut cells: Query<&mut PowerCell>,
) {
    let cycle = time.tick.0.is_multiple_of(MACHINE_TICKS);
    for (entity, mut flood, mut look, lit) in &mut lights {
        if flood.on {
            match flood.cell.and_then(|c| cells.get_mut(c).ok()) {
                Some(mut cell) => {
                    if cycle {
                        cell.charge = (cell.charge - FLOODLIGHT_USE).max(0.0);
                    }
                    if cell.charge <= 0.0 {
                        flood.on = false;
                    }
                }
                None => flood.on = false,
            }
        }
        let state = floodlight_look(&flood);
        if look.state != state {
            look.state = state;
        }
        if flood.on && !lit {
            commands.entity(entity).insert(LightSource(FLOODLIGHT_LIGHT));
        } else if !flood.on && lit {
            commands.entity(entity).remove::<LightSource>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn its_look_shows_the_panel_the_cell_and_the_light() {
        let mut flood = Floodlight {
            on: false,
            cell: Some(Entity::PLACEHOLDER),
            unlocked: false,
            open: false,
        };
        assert_eq!(floodlight_look(&flood), "flood00");
        flood.on = true;
        assert_eq!(floodlight_look(&flood), "flood01");
        flood.open = true;
        assert_eq!(floodlight_look(&flood), "floodob01");
        flood.on = false;
        flood.cell = None;
        assert_eq!(floodlight_look(&flood), "floodo00");
    }
}
