//! Fire in the station: what lights it, and how burning air looks. The burning itself is
//! in `rs13_simulation::atmos::fire`. Spec: `docs/reference/fire.md`.

use std::collections::{BTreeSet, HashMap};

use bevy_ecs::prelude::*;
use rs13_world::{Appearance, GridPos, LightSource, NetId, TilePos};

use crate::atmos::StationAtmos;
use crate::hands::HeldBy;
use crate::machines::MACHINE_TICKS;
use crate::objects::NetIds;
use crate::time::WorldTime;
use crate::welding::Welder;

/// How hot a welding tool's flame is, K (Luna `SPARK_TEMP`).
pub const FLAME_TEMPERATURE: f64 = 3500.0;
/// The fire sprite redraws this often, ticks (half a second).
pub const FIRE_VIEW_TICKS: u64 = 10;
/// Light a burning tile gives off (Luna's hotspot `ul_SetLuminosity(3)`), warm.
pub const FIRE_LIGHT: [u8; 3] = [4, 2, 1];
/// Above this many moles burnt per second per tile the fire looks fiercer.
const FIERCE: f64 = 0.01;

/// A burning tile's fire, drawn over it.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fire;

/// The fire drawn on each burning tile.
#[derive(Resource, Debug, Default)]
pub struct FireSprites(pub HashMap<TilePos, Entity>);

/// Every machine cycle, a lit welding tool touches the air where it is with its flame
/// (Luna `weldingtool/process`: `hotspot_expose(SPARK_TEMP, 5)`), in hand or on the floor.
pub fn expose_flames(
    time: Res<WorldTime>,
    atmos: Option<ResMut<StationAtmos>>,
    welders: Query<(&Welder, Option<&GridPos>, Option<&HeldBy>)>,
    places: Query<&GridPos>,
) {
    let Some(mut atmos) = atmos else { return };
    if !time.tick.0.is_multiple_of(MACHINE_TICKS) {
        return;
    }
    for (welder, pos, held) in &welders {
        if !welder.lit {
            continue;
        }
        let at = pos
            .map(|p| p.0)
            .or_else(|| held.and_then(|h| places.get(h.0).ok()).map(|p| p.0));
        if let Some(at) = at {
            atmos.0.expose(at, FLAME_TEMPERATURE);
        }
    }
}

/// Put fire on every tile of a burning zone, and take it off when the zone stops burning.
pub fn update_fire_sprites(
    time: Res<WorldTime>,
    atmos: Option<Res<StationAtmos>>,
    mut sprites: ResMut<FireSprites>,
    mut looks: Query<&mut Appearance, With<Fire>>,
    mut ids: ResMut<NetIds>,
    mut commands: Commands,
) {
    let Some(atmos) = atmos else { return };
    if !time.tick.0.is_multiple_of(FIRE_VIEW_TICKS) {
        return;
    }
    let mut burning: HashMap<TilePos, &'static str> = HashMap::new();
    for zone in atmos.0.zones() {
        if zone.burn_rate <= 0.0 {
            continue;
        }
        let per_tile = zone.burn_rate / zone.tiles.len().max(1) as f64;
        let state = if per_tile > FIERCE { "3" } else { "2" };
        for tile in &zone.tiles {
            burning.insert(*tile, state);
        }
    }
    let gone: BTreeSet<TilePos> = sprites
        .0
        .keys()
        .filter(|t| !burning.contains_key(t))
        .copied()
        .collect();
    for tile in gone {
        if let Some(entity) = sprites.0.remove(&tile) {
            commands.entity(entity).despawn();
        }
    }
    for (tile, state) in burning {
        match sprites.0.get(&tile) {
            Some(entity) => {
                if let Ok(mut look) = looks.get_mut(*entity)
                    && look.state != state
                {
                    look.state = state.to_owned();
                }
            }
            None => {
                let id: NetId = ids.alloc();
                let entity = commands
                    .spawn((
                        GridPos(tile),
                        id,
                        Appearance::new("fire", state),
                        LightSource(FIRE_LIGHT),
                        Fire,
                    ))
                    .id();
                sprites.0.insert(tile, entity);
            }
        }
    }
}
