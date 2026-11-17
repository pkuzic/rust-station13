//! The captain's display case (Luna's `objects/displaycase.dm`): glass round the captain's
//! antique laser gun, held up in a hover field. Blows, shots and blasts wear the glass down; once
//! it shatters, a hand deactivates the field and takes the gun. Spec:
//! `docs/reference/weapons.md`, Captain's antique laser.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, Dense, DisplayName, GridPos, ObjectSpec, TilePos};

use crate::actions::Outbox;
use crate::combat::VisibleEvents;
use crate::hands::Item;
use crate::rng::GameRng;
use crate::sound::{GLASS_HIT, SHATTER, SoundEvents};

pub const DISPLAY_CASE: &str = "display-case";
/// The glass's strength (Luna `health`).
pub const CASE_HEALTH: f32 = 30.0;
/// What a kick takes off it.
pub const KICK: f32 = 2.0;

/// A display case: what's left of the glass, whether the gun is still in it, and whether the
/// glass is broken.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct DisplayCase {
    pub health: f32,
    pub occupied: bool,
    pub destroyed: bool,
}

impl Default for DisplayCase {
    fn default() -> Self {
        Self {
            health: CASE_HEALTH,
            occupied: true,
            destroyed: false,
        }
    }
}

/// Give a new display case its glass and gun.
pub fn equip_item(world: &mut World, thing: Entity, proto: &str) {
    if proto == DISPLAY_CASE {
        world.entity_mut(thing).insert(DisplayCase::default());
        update_look(world, thing);
    }
}

fn at(world: &World, e: Entity) -> Option<TilePos> {
    world.get::<GridPos>(e).map(|p| p.0)
}

fn name_of(world: &World, who: Entity) -> String {
    world
        .get::<DisplayName>(who)
        .map(|n| n.0.clone())
        .unwrap_or_default()
}

fn update_look(world: &mut World, case: Entity) {
    let Some(c) = world.get::<DisplayCase>(case).copied() else {
        return;
    };
    let state = format!(
        "glassbox{}{}",
        if c.destroyed { "b" } else { "" },
        u8::from(c.occupied)
    );
    if let Some(mut look) = world.get_mut::<Appearance>(case)
        && look.state != state
    {
        look.state = state;
    }
}

/// The gun drops out onto the case's tile.
fn drop_gun(world: &mut World, case: Entity) {
    let Some(pos) = at(world, case) else { return };
    if let Some(mut c) = world.get_mut::<DisplayCase>(case) {
        if !c.occupied {
            return;
        }
        c.occupied = false;
    }
    let _ = crate::objects::spawn_object(world, &ObjectSpec::new(crate::weapons::CAPTAIN_LASER), pos);
    update_look(world, case);
}

/// Damage to the glass (Luna `healthcheck`): at nothing it shatters and can be walked through;
/// otherwise it rings.
pub fn damage(world: &mut World, case: Entity, amount: f32) {
    let Some(pos) = at(world, case) else { return };
    let Some(mut c) = world.get_mut::<DisplayCase>(case) else {
        return;
    };
    c.health -= amount;
    if c.health > 0.0 {
        world.resource_mut::<SoundEvents>().play(GLASS_HIT, pos, 75.0);
        return;
    }
    if c.destroyed {
        return;
    }
    c.destroyed = true;
    world.entity_mut(case).remove::<Dense>();
    world.resource_mut::<SoundEvents>().play(SHATTER, pos, 70.0);
    update_look(world, case);
}

/// Luna `displaycase/attackby` and `attack_hand`: an item hits the glass for its force; an
/// empty hand takes the gun from a broken case, or kicks it.
pub fn use_on(world: &mut World, mob: Entity, held: Option<Entity>, case: Entity) {
    let Some(state) = world.get::<DisplayCase>(case).copied() else {
        return;
    };
    if let Some(item) = held {
        let force = world.get::<Item>(item).map_or(0.0, |i| i.force);
        damage(world, case, force);
        return;
    }
    if state.destroyed && state.occupied {
        drop_gun(world, case);
        if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
            outbox.0.push(LocMsg::new("display-case-field-off"));
        }
        return;
    }
    if let Some(pos) = at(world, mob) {
        let msg = LocMsg::new("display-case-kick").arg("name", name_of(world, mob));
        world.resource_mut::<VisibleEvents>().0.push((pos, msg, None));
    }
    damage(world, case, KICK);
}

/// A shot stopped by a case (Luna `bullet_act`): the glass takes its damage.
pub fn shot_into(world: &mut World, tile: TilePos, amount: f32) {
    let mut q = world.query::<(Entity, &GridPos, &DisplayCase)>();
    let cases: Vec<Entity> = q
        .iter(world)
        .filter(|(_, p, c)| p.0 == tile && !c.destroyed)
        .map(|(e, ..)| e)
        .collect();
    for case in cases {
        damage(world, case, amount);
    }
}

/// An explosion reaching a case (Luna `displaycase/ex_act`): the heaviest drops the gun and
/// takes the case; the others crack the glass half the time. Whether it is gone.
pub fn blasted(world: &mut World, case: Entity, severity: u8) -> bool {
    match severity {
        1 => {
            drop_gun(world, case);
            true
        }
        _ => {
            if world.resource_mut::<GameRng>().prob(50.0) {
                damage(world, case, if severity == 2 { 15.0 } else { 5.0 });
            }
            false
        }
    }
}
