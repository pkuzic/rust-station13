//! Welding tools and fuel tanks. Spec: `docs/reference/welding.md`.

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_i18n::LocMsg;
use rs13_world::Appearance;

use crate::actions::ClickOutcome;

/// The welding tool prototype.
pub const WELDER: &str = "welder";
/// Fuel in a new welding tool, and all it holds (Luna: 30 units).
pub const WELDER_FUEL: u32 = 30;
/// Fuel in a new fuel tank (Luna: 1000 units).
pub const TANK_FUEL: u32 = 1000;
/// A lit welding tool burns instead of bruising, and harder (Luna `force = 15`,
/// `damtype = "fire"`).
pub const LIT_FORCE: f32 = 15.0;
/// Welding an airlock shut or open takes more than this (Luna: more than 2, then uses 2).
pub const DOOR_FUEL: u32 = 2;

/// A welding tool: its fuel and whether it is lit (Luna `welding`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Welder {
    pub fuel: u32,
    pub lit: bool,
}

impl Default for Welder {
    fn default() -> Self {
        Self {
            fuel: WELDER_FUEL,
            lit: false,
        }
    }
}

/// A tank of welding fuel to refill welding tools from.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FuelTank {
    pub fuel: u32,
}

/// Everything welding tools need.
#[derive(SystemParam)]
pub struct WeldWork<'w, 's> {
    welders: Query<'w, 's, &'static mut Welder>,
    tanks: Query<'w, 's, &'static mut FuelTank>,
}

impl WeldWork<'_, '_> {
    pub fn is_welder(&self, item: Entity) -> bool {
        self.welders.contains(item)
    }

    /// The welding tool, if `item` is one and it is lit.
    pub fn lit(&self, item: Option<Entity>) -> Option<Welder> {
        item.and_then(|e| self.welders.get(e).ok())
            .copied()
            .filter(|w| w.lit)
    }

    /// Clicking the welding tool in the active hand lights it or puts it out (Luna
    /// `attack_self`). `None` when `item` is no welding tool.
    pub fn use_self(&mut self, item: Entity) -> Option<ClickOutcome> {
        let mut welder = self.welders.get_mut(item).ok()?;
        Some(if welder.lit {
            welder.lit = false;
            ClickOutcome::told("welder-off")
        } else if welder.fuel == 0 {
            ClickOutcome::told("welder-no-fuel")
        } else {
            welder.lit = true;
            ClickOutcome::told("welder-on")
        })
    }

    /// A welding tool on a fuel tank: an unlit one is filled up (Luna `afterattack`).
    /// `None` when these rules don't apply. The caller checks reach.
    pub fn use_on(&mut self, item: Entity, target: Entity) -> Option<ClickOutcome> {
        let Ok(mut tank) = self.tanks.get_mut(target) else {
            return None;
        };
        let mut welder = self.welders.get_mut(item).ok()?;
        if welder.lit {
            // Luna blows the tank up; there are no explosions yet.
            return Some(ClickOutcome::default());
        }
        let moved = (WELDER_FUEL - welder.fuel.min(WELDER_FUEL)).min(tank.fuel);
        tank.fuel -= moved;
        welder.fuel += moved;
        Some(ClickOutcome::told("welder-refueled"))
    }

    /// Weld with a lit tool, using `fuel` of it, if it has more than that (Luna's
    /// airlock). Returns false, and uses nothing, when it is short.
    pub fn weld(&mut self, item: Entity, fuel: u32) -> bool {
        match self.welders.get_mut(item) {
            Ok(mut welder) if welder.lit && welder.fuel > fuel => {
                welder.fuel -= fuel;
                true
            }
            _ => false,
        }
    }

    /// Every click with a lit welding tool on something in reach burns one unit; the
    /// last one puts it out (Luna `afterattack`). The message for the mob, if any.
    pub fn after_use(&mut self, item: Entity) -> Option<LocMsg> {
        let mut welder = self.welders.get_mut(item).ok()?;
        if !welder.lit {
            return None;
        }
        welder.fuel = welder.fuel.saturating_sub(1);
        if welder.fuel > 0 {
            return None;
        }
        welder.lit = false;
        Some(LocMsg::new("welder-no-fuel"))
    }

    /// What examining a welding tool or a fuel tank adds.
    pub fn examine(&self, target: Entity) -> Option<LocMsg> {
        if let Ok(welder) = self.welders.get(target) {
            return Some(LocMsg::new("examine-welder").num("count", i64::from(welder.fuel)));
        }
        let tank = self.tanks.get(target).ok()?;
        Some(LocMsg::new("examine-fuel-tank").num("count", i64::from(tank.fuel)))
    }
}

/// A lit welding tool shows its flame.
pub fn update_welder_looks(mut welders: Query<(&Welder, &mut Appearance), Changed<Welder>>) {
    for (welder, mut look) in &mut welders {
        let state = if welder.lit { "lit" } else { "" };
        if look.state != state {
            look.state = state.to_owned();
        }
    }
}
