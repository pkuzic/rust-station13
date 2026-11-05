//! Jetpacks: a tank of oxygen worn on the back that pushes its wearer through open space.
//! Spec: `docs/reference/jetpacks.md`.

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_world::{Appearance, Slot};

use crate::actions::ClickOutcome;
use crate::equipment::{Equipment, TankGas};

/// Gas one push uses, mol (Luna `allow_thrust(0.005)`).
pub const THRUST_MOLES: f64 = 0.005;

/// Jetpack prototypes: Luna's plain one and the chief engineer's.
pub fn is_jetpack(proto: &str) -> bool {
    matches!(proto, "jetpack" | "jetpack-ce" | "jetpack-syndie")
}

/// A jetpack's switch. On, it pushes its wearer through open space and keeps them from
/// drifting (Luna's thrusters and stabiliser, worked by one switch here).
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Jetpack {
    pub on: bool,
}

/// The switched-on jetpack worn on the back, if any.
pub fn worn_jetpack(equipment: Option<&Equipment>, jetpacks: &Query<&Jetpack>) -> Option<Entity> {
    let back = equipment?.get(Slot::Back)?;
    jetpacks.get(back).ok().filter(|j| j.on).map(|_| back)
}

/// One push from the jetpack `pack`: uses [`THRUST_MOLES`] of its gas, or fails without
/// using any when it has less (Luna `allow_thrust`).
pub fn thrust(pack: Entity, tanks: &mut Query<&mut TankGas>) -> bool {
    let Ok(mut tank) = tanks.get_mut(pack) else {
        return false;
    };
    if tank.mix.total_moles() < THRUST_MOLES {
        return false;
    }
    tank.mix.remove_moles(THRUST_MOLES);
    true
}

/// Everything switching a jetpack needs.
#[derive(SystemParam)]
pub struct JetpackWork<'w, 's> {
    jetpacks: Query<'w, 's, &'static mut Jetpack>,
}

impl JetpackWork<'_, '_> {
    pub fn contains(&self, item: Entity) -> bool {
        self.jetpacks.contains(item)
    }

    /// Clicking the jetpack in the active hand switches it (Luna's "Toggle Jetpack").
    pub fn use_self(&mut self, item: Entity) -> Option<ClickOutcome> {
        let mut jetpack = self.jetpacks.get_mut(item).ok()?;
        jetpack.on = !jetpack.on;
        Some(ClickOutcome::told(if jetpack.on {
            "jetpack-on"
        } else {
            "jetpack-off"
        }))
    }
}

/// A switched-on jetpack looks it, in hand and on the back.
pub fn update_jetpack_looks(mut jetpacks: Query<(&Jetpack, &mut Appearance), Changed<Jetpack>>) {
    for (jetpack, mut look) in &mut jetpacks {
        let state = if jetpack.on { "on" } else { "" };
        if look.state != state {
            look.state = state.to_owned();
        }
    }
}
