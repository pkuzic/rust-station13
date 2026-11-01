//! The station's decks, for game rules: one map layer per deck (`MapLayer`), looked up by a
//! position's `z`. Rules ask through [`Decks`] what a tile is, whether it is solid or
//! opaque and which area it is in, wherever it is.

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_world::{MapLayer, TileDef, TileMap, TilePos};

/// Every deck of the station.
#[derive(SystemParam)]
pub struct Decks<'w, 's> {
    maps: Query<'w, 's, &'static MapLayer>,
}

/// What [`Decks::at`] gives off every deck: nothing but solid.
fn no_deck() -> &'static TileMap {
    static EMPTY: std::sync::OnceLock<TileMap> = std::sync::OnceLock::new();
    EMPTY.get_or_init(|| {
        TileMap::from_ron(
            r##"(legend: {'#': (tile: (id: "wall", solid: true))}, rows: ["#"])"##,
            i32::MIN,
        )
        .expect("a valid map")
    })
}

impl Decks<'_, '_> {
    /// The deck at height `z`, or a map of nothing at all, for rules that work on one deck.
    pub fn at(&self, z: i32) -> &TileMap {
        self.deck(z).unwrap_or_else(|| no_deck())
    }

    /// The deck at height `z`.
    pub fn deck(&self, z: i32) -> Option<&TileMap> {
        self.maps.iter().map(|m| &m.0).find(|m| m.z == z)
    }

    /// Every deck.
    pub fn iter(&self) -> impl Iterator<Item = &TileMap> {
        self.maps.iter().map(|m| &m.0)
    }

    pub fn tile(&self, pos: TilePos) -> Option<&TileDef> {
        self.deck(pos.z)?.tile(pos)
    }

    /// Whether a tile blocks movement. Off every deck counts as solid.
    pub fn is_solid(&self, pos: TilePos) -> bool {
        self.deck(pos.z).is_none_or(|m| m.is_solid(pos))
    }

    pub fn is_opaque(&self, pos: TilePos) -> bool {
        self.deck(pos.z).is_some_and(|m| m.is_opaque(pos))
    }

    /// Every deck's start points.
    pub fn spawns(&self) -> impl Iterator<Item = TilePos> + '_ {
        self.iter().flat_map(|m| m.spawns.iter().copied())
    }

    /// The area a tile is in, if any.
    pub fn area(&self, pos: TilePos) -> Option<&str> {
        self.deck(pos.z)?.area(pos)
    }
}
