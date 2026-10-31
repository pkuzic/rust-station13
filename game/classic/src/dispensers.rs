//! Tank storage units: a rack of oxygen tanks to take from and put back. Spec:
//! `docs/reference/tank-dispenser.md`.

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, Dense, GridPos, NetId, ObjectSpec, TilePos};

use crate::actions::ClickOutcome;
use crate::hands::{Hands, HeldBy};

/// The dispenser prototype.
pub const TANK_DISPENSER: &str = "tank-dispenser";
/// What it holds.
pub const OXYGEN_TANK: &str = "oxygen-tank";
/// Tanks it holds, and starts with (Luna `oxygentanks`).
pub const DISPENSER_TANKS: u32 = 10;

/// A tank storage unit.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct TankDispenser {
    /// Tanks in it, counting the ones it started with.
    pub tanks: u32,
    /// Tanks people put in, given out again first, newest first. The rest are new.
    pub stored: Vec<Entity>,
}

impl TankDispenser {
    /// Overlay showing how full it is (Luna `update_icon`): "oxygen-5" full, down to
    /// "oxygen-1"; nothing when empty.
    pub fn look(&self) -> String {
        match self.tanks {
            0 => String::new(),
            DISPENSER_TANKS => "+oxygen-5".to_owned(),
            n => format!("+oxygen-{}", n / 2 + 1),
        }
    }
}

/// On a tank put into a dispenser: which one.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InDispenser(pub Entity);

pub fn spawn_tank_dispenser(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    let dispenser = TankDispenser {
        tanks: DISPENSER_TANKS,
        stored: Vec::new(),
    };
    let look = dispenser.look();
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(spec.id.as_str(), look),
            dispenser,
            Dense,
        ))
        .id()
}

/// Everything clicking a dispenser needs.
#[derive(SystemParam)]
pub struct DispenserWork<'w, 's> {
    dispensers: Query<'w, 's, (&'static GridPos, &'static mut TankDispenser)>,
}

impl DispenserWork<'_, '_> {
    pub fn contains(&self, target: Entity) -> bool {
        self.dispensers.contains(target)
    }

    /// Luna `attack_hand` (its window's "Dispense") and `attackby`. An empty hand takes a
    /// tank into that hand; an oxygen tank goes back in. The caller checks reach.
    pub fn use_on(
        &mut self,
        mob: Entity,
        target: Entity,
        held: Option<&str>,
        hands: &mut Hands,
        commands: &mut Commands,
    ) -> Option<ClickOutcome> {
        let (pos, mut dispenser) = self.dispensers.get_mut(target).ok()?;
        let outcome = match (held, hands.active_item()) {
            (None, _) if dispenser.tanks == 0 => ClickOutcome::told("dispenser-empty"),
            (None, _) => {
                dispenser.tanks -= 1;
                let hand = hands.active;
                if let Some(tank) = dispenser.stored.pop() {
                    commands.entity(tank).remove::<InDispenser>().insert(HeldBy(mob));
                    *hands.slot_mut(hand) = Some(tank);
                } else {
                    let at = pos.0;
                    commands.queue(move |world: &mut World| {
                        let spec = ObjectSpec::new(OXYGEN_TANK);
                        let Ok(tank) = crate::objects::spawn_object(world, &spec, at) else {
                            return;
                        };
                        // Into the hand if it is still free, else on the floor.
                        let free = world.get::<Hands>(mob).is_some_and(|h| h.get(hand).is_none());
                        if free {
                            world.entity_mut(tank).remove::<GridPos>().insert(HeldBy(mob));
                            if let Some(mut hands) = world.get_mut::<Hands>(mob) {
                                *hands.slot_mut(hand) = Some(tank);
                            }
                        }
                    });
                }
                ClickOutcome {
                    to_mob: Some(LocMsg::new("dispenser-taken").arg("item", OXYGEN_TANK)),
                    seen: None,
                }
            }
            (Some(OXYGEN_TANK), _) if dispenser.tanks >= DISPENSER_TANKS => {
                ClickOutcome::told("dispenser-full")
            }
            (Some(OXYGEN_TANK), Some(tank)) => {
                dispenser.tanks += 1;
                dispenser.stored.push(tank);
                commands
                    .entity(tank)
                    .remove::<HeldBy>()
                    .insert(InDispenser(target));
                *hands.slot_mut(hands.active) = None;
                ClickOutcome {
                    to_mob: Some(LocMsg::new("dispenser-put").arg("item", OXYGEN_TANK)),
                    seen: None,
                }
            }
            _ => return None,
        };
        Some(outcome)
    }

    /// What examining a dispenser adds.
    pub fn examine(&self, target: Entity) -> Option<LocMsg> {
        let (_, dispenser) = self.dispensers.get(target).ok()?;
        Some(LocMsg::new("examine-tank-dispenser").num("count", i64::from(dispenser.tanks)))
    }
}

/// A dispenser shows how full it is.
pub fn update_dispenser_looks(
    mut dispensers: Query<(&TankDispenser, &mut Appearance), Changed<TankDispenser>>,
) {
    for (dispenser, mut look) in &mut dispensers {
        let state = dispenser.look();
        if look.state != state {
            look.state = state;
        }
    }
}
