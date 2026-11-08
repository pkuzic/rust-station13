//! How well someone sees (Luna's `eye_blind`, `eye_blurry`, nearsightedness, `sdisabilities`, the
//! blindfold and X-ray): worked out into the mob's [`Vision`] and [`EyeTrouble`], which the
//! server acts on and sends. Spec: `docs/reference/eyes.md`.

use bevy_ecs::prelude::*;
use rs13_world::{Appearance, EyeTrouble, Slot, Vision};

use crate::equipment::Equipment;
use crate::genetics::{Gene, Mutations};
use crate::movement::Mob;
use crate::time::WorldTime;

/// One of Luna's life ticks, ds: `eye_blind` and `eye_blurry` go down by one each.
pub const COUNT_DS: f32 = 20.0;

/// Blind until then, ds (Luna `eye_blind`).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct EyeBlind(pub f32);

/// Blurred until then, ds (Luna `eye_blurry`).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct EyeBlurry(pub f32);

/// Nearsighted until then, ds (Luna's `disabilities` bit 1, from a sting or a spell).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Nearsighted(pub f32);

fn until(world: &World, counts: f32) -> f32 {
    world.resource::<WorldTime>().deciseconds() + counts * COUNT_DS
}

/// Blind for `counts` life ticks, or longer if they already were.
pub fn blind(world: &mut World, mob: Entity, counts: f32) {
    let t = until(world, counts);
    let t = world.get::<EyeBlind>(mob).map_or(t, |b| b.0.max(t));
    world.entity_mut(mob).insert(EyeBlind(t));
    refresh(world, mob);
}

/// Blurred for `counts` life ticks, or longer if they already were.
pub fn blur(world: &mut World, mob: Entity, counts: f32) {
    let t = until(world, counts);
    let t = world.get::<EyeBlurry>(mob).map_or(t, |b| b.0.max(t));
    world.entity_mut(mob).insert(EyeBlurry(t));
    refresh(world, mob);
}

/// Nearsighted for `ds`, or longer if they already were.
pub fn nearsight(world: &mut World, mob: Entity, ds: f32) {
    let t = world.resource::<WorldTime>().deciseconds() + ds;
    let t = world.get::<Nearsighted>(mob).map_or(t, |b| b.0.max(t));
    world.entity_mut(mob).insert(Nearsighted(t));
    refresh(world, mob);
}

fn wearing(world: &World, mob: Entity, sprite: &str) -> bool {
    world
        .get::<Equipment>(mob)
        .and_then(|e| e.get(Slot::Glasses))
        .and_then(|item| world.get::<Appearance>(item))
        .is_some_and(|a| a.sprite == sprite)
}

/// Work out how someone sees now, and drop what has worn off.
pub fn refresh(world: &mut World, mob: Entity) {
    let now = world.resource::<WorldTime>().deciseconds();
    let gene = |g: Gene| world.get::<Mutations>(mob).is_some_and(|m| m.0.contains(&g));
    let (blind_gene, xray) = (gene(Gene::Blind), gene(Gene::Xray));
    let blinded = world.get::<EyeBlind>(mob).is_some_and(|b| b.0 > now);
    let blurry = world.get::<EyeBlurry>(mob).is_some_and(|b| b.0 > now);
    // Glasses put nearsightedness right (Luna's `glasses/regular`).
    let short = world.get::<Nearsighted>(mob).is_some_and(|b| b.0 > now);
    let nearsighted = short && !wearing(world, mob, "glasses");
    // Luna's blindness covers the screen whatever else they could see.
    let vision = if blind_gene || blinded || wearing(world, mob, "blindfold") {
        Vision::Blind
    } else if xray {
        Vision::Xray
    } else {
        Vision::Normal
    };
    let eyes = EyeTrouble { blurry, nearsighted };
    let mut e = world.entity_mut(mob);
    if !blinded {
        e.remove::<EyeBlind>();
    }
    if !blurry {
        e.remove::<EyeBlurry>();
    }
    if !short {
        e.remove::<Nearsighted>();
    }
    if vision == Vision::Normal {
        if e.contains::<Vision>() {
            e.remove::<Vision>();
        }
    } else if e.get::<Vision>() != Some(&vision) {
        e.insert(vision);
    }
    if eyes == EyeTrouble::default() {
        if e.contains::<EyeTrouble>() {
            e.remove::<EyeTrouble>();
        }
    } else if e.get::<EyeTrouble>() != Some(&eyes) {
        e.insert(eyes);
    }
}

/// Every tick: how each mob sees.
pub fn run_eyes(world: &mut World) {
    let mut q = world.query_filtered::<Entity, Or<(
        With<Mutations>,
        With<EyeBlind>,
        With<EyeBlurry>,
        With<Nearsighted>,
        With<Vision>,
        With<EyeTrouble>,
        (With<Mob>, With<Equipment>),
    )>>();
    let mobs: Vec<Entity> = q.iter(world).collect();
    for mob in mobs {
        refresh(world, mob);
    }
}
