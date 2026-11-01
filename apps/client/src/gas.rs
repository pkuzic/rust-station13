//! Gas you can see (`docs/reference/lighting.md`, Gas): plasma and sleeping gas drawn over
//! the tiles that hold enough of them, as Luna does (`tile_effects.dmi`), and fainter the
//! less there is, so a leak shows as a thin haze and a flooded room as a thick one.

use std::collections::HashMap;

use bevy::prelude::*;
use rs13_world::{AtmosView, MapLayer, TilePos};

use crate::shadows::Sight;
use crate::sprites::{Look, SpriteAtlas};
use crate::world_pos;

/// Over people and what stands on the floor (3.0–3.2), under wall lights (3.5), and lit
/// by the light overlay like everything else.
const GAS_Z: f32 = 3.4;
/// How opaque the thinnest visible gas is.
const FAINTEST: f32 = 0.3;
/// Moles on a tile at which a gas is drawn fully opaque.
const THICK: f32 = 10.0;

/// A gas you can see.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VisibleGas {
    Plasma,
    SleepingAgent,
}

impl VisibleGas {
    const ALL: [VisibleGas; 2] = [VisibleGas::Plasma, VisibleGas::SleepingAgent];

    /// Moles on a tile above which the gas shows (Luna `MOLES_PLASMA_VISIBLE`, and the
    /// sleeping agent's 1 mol).
    fn visible_above(self) -> f32 {
        match self {
            VisibleGas::Plasma => 0.5,
            VisibleGas::SleepingAgent => 1.0,
        }
    }

    fn state(self) -> &'static str {
        match self {
            VisibleGas::Plasma => "plasma",
            VisibleGas::SleepingAgent => "sleeping-agent",
        }
    }

    /// Tenths of a mole of this gas on every tile.
    fn amounts(self, view: &AtmosView) -> &[u8] {
        match self {
            VisibleGas::Plasma => &view.plasma_dmol,
            VisibleGas::SleepingAgent => &view.n2o_dmol,
        }
    }
}

/// How opaque `moles` of `gas` is drawn on a tile, or `None` if too little to see.
pub fn opacity(gas: VisibleGas, moles: f32) -> Option<f32> {
    let from = gas.visible_above();
    if moles <= from {
        return None;
    }
    let t = ((moles - from) / (THICK - from)).clamp(0.0, 1.0);
    Some(FAINTEST + (1.0 - FAINTEST) * t)
}

/// One tile's cloud of one gas.
#[derive(Component)]
struct Cloud;

/// The clouds drawn now, by tile and gas.
#[derive(Resource, Default)]
struct Clouds(HashMap<(TilePos, VisibleGas), Entity>);

pub struct GasPlugin;

impl Plugin for GasPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Clouds>()
            .add_systems(Update, (update_clouds, hide_unseen).chain());
    }
}

/// Spawn, fade and remove clouds as the air changes.
fn update_clouds(
    mut commands: Commands,
    atlas: Option<Res<SpriteAtlas>>,
    time: Res<Time>,
    maps: Query<(&MapLayer, &AtmosView), Changed<AtmosView>>,
    mut clouds: ResMut<Clouds>,
    mut sprites: Query<&mut Sprite, With<Cloud>>,
) {
    let Some(atlas) = atlas else { return };
    for (MapLayer(map), view) in &maps {
        let mut wanted = HashMap::new();
        for gas in VisibleGas::ALL {
            for (i, amount) in gas.amounts(view).iter().enumerate() {
                let moles = f32::from(*amount) / 10.0;
                if let Some(alpha) = opacity(gas, moles) {
                    let (x, y) = (i as u32 % map.width, i as u32 / map.width);
                    wanted.insert((TilePos::new(x as i32, y as i32, map.z), gas), alpha);
                }
            }
        }
        clouds.0.retain(|key, entity| {
            let keep = key.0.z != map.z || wanted.contains_key(key);
            if !keep {
                commands.entity(*entity).despawn();
            }
            keep
        });
        for ((tile, gas), alpha) in wanted {
            let colour = Color::srgba(1.0, 1.0, 1.0, alpha);
            if let Some(entity) = clouds.0.get(&(tile, gas)) {
                if let Ok(mut sprite) = sprites.get_mut(*entity)
                    && sprite.color != colour
                {
                    sprite.color = colour;
                }
                continue;
            }
            let Some(mut sprite) = atlas.sprite("gas", gas.state(), 0) else {
                continue;
            };
            sprite.color = colour;
            let entity = commands
                .spawn((
                    Cloud,
                    sprite,
                    Transform::from_translation(world_pos(tile, GAS_Z)),
                    Look {
                        sprite: "gas".into(),
                        state: gas.state().into(),
                        facing: 0,
                        since: time.elapsed_secs(),
                    },
                ))
                .id();
            clouds.0.insert((tile, gas), entity);
        }
    }
}

/// Gas on tiles out of sight isn't drawn.
fn hide_unseen(sight: Res<Sight>, clouds: Res<Clouds>, mut visibility: Query<&mut Visibility, With<Cloud>>) {
    for ((tile, _), entity) in &clouds.0 {
        if let Ok(mut v) = visibility.get_mut(*entity) {
            v.set_if_neq(if sight.sees(*tile) {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gas_shows_above_its_threshold_and_thickens_with_amount() {
        assert_eq!(opacity(VisibleGas::Plasma, 0.4), None);
        assert_eq!(
            opacity(VisibleGas::SleepingAgent, 0.9),
            None,
            "needs a whole mole"
        );
        let thin = opacity(VisibleGas::Plasma, 0.6).unwrap();
        let thick = opacity(VisibleGas::Plasma, 5.0).unwrap();
        assert!(thin >= FAINTEST && thin < thick);
        assert_eq!(opacity(VisibleGas::Plasma, 50.0), Some(1.0));
    }
}
