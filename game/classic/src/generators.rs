//! Portable generators: the P.A.C.M.A.N. burns plasma sheets and feeds the cable on its
//! tile. Spec: `docs/reference/generators.md`.

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, Dense, GridPos, NetId, ObjectSpec, TilePos};

use crate::actions::ClickOutcome;
use crate::hands::{Hands, Stack};
use crate::machines::MACHINE_TICKS;
use crate::power::{PowerNode, PowerSupply, Powernets};
use crate::time::WorldTime;

/// The generator prototype.
pub const PACMAN: &str = "pacman";
/// What it burns.
pub const PLASMA_SHEET: &str = "plasma-sheet";
/// Output while running, W (Luna `power_gen` 5 000 per cycle at the lowest setting).
pub const PACMAN_OUTPUT: f64 = 5_000.0;
/// Sheets it holds (Luna: 50 per matter bin rating squared, rating 1).
pub const PACMAN_MAX_SHEETS: u32 = 50;
/// Machine cycles one sheet lasts (Luna `time_per_sheet` at output 1).
pub const CYCLES_PER_SHEET: u32 = 100;

/// A P.A.C.M.A.N.-type portable generator.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pacman {
    /// Whole sheets waiting in the hopper.
    pub sheets: u32,
    /// Cycles left of the sheet being burned (Luna `sheet_left`).
    pub burning: u32,
    pub active: bool,
    /// Wrenched to the floor; only then does it join the cable on its tile.
    pub anchored: bool,
}

impl Pacman {
    pub fn has_fuel(&self) -> bool {
        self.burning > 0 || self.sheets > 0
    }

    /// Burn one cycle's worth, starting a new sheet when the last one is done.
    fn burn(&mut self) {
        if self.burning == 0 && self.sheets > 0 {
            self.sheets -= 1;
            self.burning = CYCLES_PER_SHEET;
        }
        self.burning = self.burning.saturating_sub(1);
    }
}

/// Luna's generators start unsecured and empty.
pub fn spawn_pacman(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(spec.id.as_str(), "portgen0"),
            Pacman {
                sheets: 0,
                burning: 0,
                active: false,
                anchored: false,
            },
            PowerSupply::default(),
            Dense,
        ))
        .id()
}

/// Everything clicking a generator needs.
#[derive(SystemParam)]
pub struct GeneratorWork<'w, 's> {
    generators: Query<'w, 's, &'static mut Pacman>,
}

impl GeneratorWork<'_, '_> {
    pub fn contains(&self, target: Entity) -> bool {
        self.generators.contains(target)
    }

    /// Luna `attackby` and the generator's panel. Plasma sheets go in the hopper. While it
    /// is off, a wrench secures or unsecures it. An empty hand switches a secured one on or
    /// off. `stacks` are the items that pile up. The caller checks reach.
    pub fn use_on(
        &mut self,
        target: Entity,
        held: Option<&str>,
        hands: &mut Hands,
        stacks: &mut Query<&mut Stack>,
        commands: &mut Commands,
    ) -> Option<ClickOutcome> {
        let mut generator = self.generators.get_mut(target).ok()?;
        match held {
            Some(PLASMA_SHEET) => {
                let item = hands.active_item()?;
                let mut stack = stacks.get_mut(item).ok()?;
                let amount = (PACMAN_MAX_SHEETS - generator.sheets.min(PACMAN_MAX_SHEETS)).min(stack.amount);
                if amount == 0 {
                    return Some(ClickOutcome::told("pacman-full"));
                }
                generator.sheets += amount;
                stack.amount -= amount;
                if stack.amount == 0 {
                    commands.entity(item).despawn();
                    *hands.slot_mut(hands.active) = None;
                }
                Some(ClickOutcome {
                    to_mob: Some(LocMsg::new("pacman-fuel-added").num("count", i64::from(amount))),
                    seen: None,
                })
            }
            Some("wrench") if !generator.active => {
                generator.anchored = !generator.anchored;
                let id = if generator.anchored {
                    commands.entity(target).insert(PowerNode);
                    "pacman-secured"
                } else {
                    commands.entity(target).remove::<PowerNode>();
                    "pacman-unsecured"
                };
                Some(ClickOutcome::told(id))
            }
            None if generator.anchored => Some(if generator.active {
                generator.active = false;
                ClickOutcome::told("pacman-off")
            } else if generator.has_fuel() {
                generator.active = true;
                ClickOutcome::told("pacman-on")
            } else {
                ClickOutcome::told("pacman-no-fuel")
            }),
            _ => Some(ClickOutcome::default()),
        }
    }

    /// What examining a generator adds (Luna `examine`).
    pub fn examine(&self, target: Entity) -> Option<LocMsg> {
        let generator = self.generators.get(target).ok()?;
        Some(
            LocMsg::new("examine-pacman")
                .arg("on", if generator.active { "yes" } else { "no" })
                .num("count", i64::from(generator.sheets))
                .num("watts", PACMAN_OUTPUT as i64),
        )
    }
}

/// Every machine cycle, before the power cycle: a running generator that is secured, has
/// fuel and sits on a network burns a cycle's worth and supplies it; otherwise it stops
/// (Luna `port_gen/process`).
pub fn run_generators(
    time: Res<WorldTime>,
    nets: Res<Powernets>,
    mut generators: Query<(Entity, &mut Pacman, &mut PowerSupply, &mut Appearance)>,
) {
    if !time.tick.0.is_multiple_of(MACHINE_TICKS) {
        return;
    }
    for (entity, mut generator, mut supply, mut look) in &mut generators {
        let running = generator.active
            && generator.has_fuel()
            && generator.anchored
            && nets.net_of.contains_key(&entity);
        if running {
            generator.burn();
        } else if generator.active {
            generator.active = false;
        }
        let watts = if running { PACMAN_OUTPUT } else { 0.0 };
        if supply.watts != watts {
            supply.watts = watts;
        }
        let state = if running { "portgen1" } else { "portgen0" };
        if look.state != state {
            look.state = state.to_owned();
        }
    }
}
