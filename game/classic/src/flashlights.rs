//! Flashlights, penlights, lanterns and desk lamps. Spec: `docs/reference/flashlights.md`.

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_world::{Appearance, GridPos, LightSource};

use crate::actions::ClickOutcome;
use crate::closets::InCloset;
use crate::equipment::WornBy;
use crate::hands::HeldBy;
use crate::movement::Mob;

/// A hand-held light: how strong it is when lit, and whether it is (Luna `on`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flashlight {
    pub rgb: [u8; 3],
    pub on: bool,
}

/// The light a prototype gives off when lit, and whether it starts lit (Luna
/// `brightness_on_r`, `_g`, `_b` and `on`).
pub fn kind(proto: &str) -> Option<Flashlight> {
    let (rgb, on) = match proto {
        "flashlight" => ([4, 4, 4], false),
        "penlight" => ([2, 2, 2], false),
        "lantern" => ([6, 6, 5], false),
        "desk-lamp" | "desk-lamp-green" => ([6, 6, 6], true),
        _ => return None,
    };
    Some(Flashlight { rgb, on })
}

/// Everything using a hand-held light needs.
#[derive(SystemParam)]
pub struct FlashlightWork<'w, 's> {
    lights: Query<'w, 's, &'static mut Flashlight>,
}

impl FlashlightWork<'_, '_> {
    pub fn contains(&self, item: Entity) -> bool {
        self.lights.contains(item)
    }

    /// Clicking a hand-held light in the active hand switches it, silently (Luna
    /// `attack_self`). `None` when `item` is no hand-held light.
    pub fn use_self(&mut self, item: Entity) -> Option<ClickOutcome> {
        let mut light = self.lights.get_mut(item).ok()?;
        light.on = !light.on;
        Some(ClickOutcome::default())
    }
}

/// A lit hand-held light shows it.
pub fn update_flashlight_looks(mut lights: Query<(&Flashlight, &mut Appearance), Changed<Flashlight>>) {
    for (light, mut look) in &mut lights {
        let state = if light.on { "on" } else { "" };
        if look.state != state {
            look.state = state.to_owned();
        }
    }
}

/// A lit light on the floor lights its own tile; one in a hand or worn lights its carrier's,
/// adding to the carrier's other lights, as in Luna. Nothing gets out of a closet or a bag.
#[allow(clippy::type_complexity)]
pub fn update_flashlight_sources(
    mut commands: Commands,
    lights: Query<(
        Entity,
        &Flashlight,
        Has<GridPos>,
        Option<&HeldBy>,
        Option<&WornBy>,
    )>,
    sources: Query<&LightSource>,
    mobs: Query<(Entity, Has<InCloset>), With<Mob>>,
) {
    let mut carried: HashMap<Entity, [u8; 3]> = HashMap::new();
    for (item, light, on_floor, held, worn) in &lights {
        let carrier = held.map(|h| h.0).or(worn.map(|w| w.0));
        let own = (light.on && on_floor && carrier.is_none()).then_some(LightSource(light.rgb));
        set_source(&mut commands, &sources, item, own);
        if let Some(carrier) = carrier
            && light.on
        {
            let sum = carried.entry(carrier).or_default();
            *sum = std::array::from_fn(|c| sum[c].saturating_add(light.rgb[c]));
        }
    }
    for (mob, in_closet) in &mobs {
        let own = carried
            .get(&mob)
            .filter(|_| !in_closet)
            .map(|rgb| LightSource(*rgb));
        set_source(&mut commands, &sources, mob, own);
    }
}

fn set_source(
    commands: &mut Commands,
    sources: &Query<&LightSource>,
    entity: Entity,
    want: Option<LightSource>,
) {
    let have = sources.get(entity).ok().copied();
    if have == want {
        return;
    }
    match want {
        Some(source) => commands.entity(entity).insert(source),
        None => commands.entity(entity).remove::<LightSource>(),
    };
}
