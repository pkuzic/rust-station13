//! The energy sword (Luna's `melee/energy/sword`): a small, harmless hilt until it's switched on,
//! then a 30-force blade too big to pocket that can also block blows and shots. Spec:
//! `docs/reference/weapons.md`, Energy sword.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, GridPos};

use crate::actions::Outbox;
use crate::hands::{HeldBy, Item};
use crate::rng::GameRng;
use crate::sound::SoundEvents;

pub const ENERGY_SWORD: &str = "energy-sword";
/// Its force and size switched on, and off (Luna `force`, `w_class`).
pub const ON_FORCE: f32 = 30.0;
pub const OFF_FORCE: f32 = 3.0;
pub const ON_SIZE: u8 = 4;
pub const OFF_SIZE: u8 = 1;
const COLOURS: [&str; 4] = ["red", "blue", "green", "purple"];

/// An energy sword: whether its blade is out, and its colour (picked when it's made).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnergySword {
    pub active: bool,
    pub colour: &'static str,
}

/// Give a new sword its colour.
pub fn equip_item(world: &mut World, item: Entity, proto: &str) {
    if proto != ENERGY_SWORD {
        return;
    }
    let i = world.resource_mut::<GameRng>().range(0, COLOURS.len() as i32 - 1) as usize;
    world.entity_mut(item).insert(EnergySword {
        active: false,
        colour: COLOURS[i.min(COLOURS.len() - 1)],
    });
}

/// Whether this held item blocks like a shield (Luna `IsShield`): a riot shield, or a sword with
/// its blade out.
pub fn is_shield(world: &World, item: Entity) -> bool {
    world
        .get::<Appearance>(item)
        .is_some_and(|a| a.sprite == "riot-shield")
        || world.get::<EnergySword>(item).is_some_and(|s| s.active)
}

/// Luna `energy/sword/attack_self`: the blade out or away.
pub fn toggle(world: &mut World, mob: Entity, sword: Entity) {
    let Some(mut state) = world.get_mut::<EnergySword>(sword) else {
        return;
    };
    state.active = !state.active;
    let EnergySword { active, colour } = *state;
    if let Some(mut item) = world.get_mut::<Item>(sword) {
        item.force = if active { ON_FORCE } else { OFF_FORCE };
        item.size = if active { ON_SIZE } else { OFF_SIZE };
    }
    if let Some(mut look) = world.get_mut::<Appearance>(sword) {
        look.state = if active {
            format!("sword{colour}")
        } else {
            "sword0".to_owned()
        };
    }
    let at = world
        .get::<HeldBy>(sword)
        .and_then(|h| world.get::<GridPos>(h.0))
        .or_else(|| world.get::<GridPos>(sword))
        .map(|p| p.0);
    if let Some(at) = at {
        let sound = if active {
            crate::sound::SABER_ON
        } else {
            crate::sound::SABER_OFF
        };
        world.resource_mut::<SoundEvents>().play(sound, at, 50.0);
    }
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox
            .0
            .push(LocMsg::new(if active { "sword-on" } else { "sword-off" }));
    }
}

/// Whether someone blocks an attack doing `damage` with what they hold (Luna `check_shields`:
/// 50% less a third of the damage), and with what.
pub fn shield_block(world: &mut World, person: Entity, damage: f32) -> Option<Entity> {
    let hands = world.get::<crate::hands::Hands>(person).copied()?;
    let chance = 50.0 - (damage / 3.0).round();
    [hands.left, hands.right]
        .into_iter()
        .flatten()
        .find(|item| is_shield(world, *item) && world.resource_mut::<GameRng>().prob(chance))
}
