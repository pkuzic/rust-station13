//! The light on each tile, on the server, so that game rules can ask where it is dark.
//! Spec: docs/reference/lighting.md.
//!
//! Worked out with the same engine code and inputs as the client draws (lit fixtures,
//! closed airlocks, the map), again only when one of them changes.

use bevy_ecs::prelude::*;
use rs13_simulation::lighting::{Emitter, LightField, MapDecks, Rgb};
use rs13_world::{GridPos, LightSource, MapLayer, Opaque, TilePos};

/// The light on every tile of the map.
#[derive(Resource, Default)]
pub struct TileLight {
    field: LightField,
    emitters: Vec<Emitter>,
    opaque: Vec<TilePos>,
    /// Whether `field` has been worked out at least once.
    ready: bool,
}

impl TileLight {
    /// Red, green and blue light on a tile, each from 0 (dark) to 1 (fully lit).
    pub fn level(&self, pos: TilePos) -> Rgb {
        self.field.level(pos)
    }

    /// How bright a tile is, from 0 to 1 (its brightest colour).
    pub fn brightness(&self, pos: TilePos) -> f32 {
        self.field.brightness(pos)
    }

    /// Whether a tile is too dark to see in.
    pub fn is_dark(&self, pos: TilePos) -> bool {
        self.field.is_dark(pos)
    }
}

/// Work the light out again when the map, a lit fixture or a closed airlock changed.
pub fn update_tile_light(
    mut light: ResMut<TileLight>,
    maps: Query<Ref<MapLayer>>,
    sources: Query<(&GridPos, &LightSource)>,
    doors: Query<&GridPos, With<Opaque>>,
) {
    let mut emitters: Vec<Emitter> = sources
        .iter()
        .map(|(pos, LightSource(rgb))| Emitter {
            pos: pos.0,
            rgb: rgb.map(f32::from),
        })
        .collect();
    emitters.sort_by_key(|e| e.pos);
    let mut opaque: Vec<TilePos> = doors.iter().map(|p| p.0).collect();
    opaque.sort();
    let map_changed = maps.iter().any(|m| m.is_changed());
    if light.ready && !map_changed && light.emitters == emitters && light.opaque == opaque {
        return;
    }
    light.field =
        MapDecks::new(maps.iter().map(|m| &m.into_inner().0), opaque.iter().copied()).light(&emitters);
    light.emitters = emitters;
    light.opaque = opaque;
    light.ready = true;
}
