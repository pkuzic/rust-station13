//! Chunked tile grid with x, y and z (station deck) coordinates.

mod components;
mod map;

pub use components::{
    AlarmPanelView, AlarmReading, Appearance, AtmosView, Dense, DisplayName, EnvAlert, GridPos, Hand,
    HandsView, Intent, MapLayer, MobStat, NetId, Opaque, Slot, StorageView, WirePanelView, WornView,
};
pub use map::{Dir, MapError, ObjectSpec, TileDef, TileMap};

use serde::{Deserialize, Serialize};

/// Width and height of a chunk, in tiles.
pub const CHUNK_SIZE: i32 = 16;

/// A tile on the map. `z` is the deck (floor) index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TilePos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// A chunk of `CHUNK_SIZE` x `CHUNK_SIZE` tiles on one deck.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ChunkPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl TilePos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub fn chunk(self) -> ChunkPos {
        ChunkPos {
            x: self.x.div_euclid(CHUNK_SIZE),
            y: self.y.div_euclid(CHUNK_SIZE),
            z: self.z,
        }
    }

    /// Position inside the chunk, each component in `0..CHUNK_SIZE`.
    pub fn local(self) -> (i32, i32) {
        (self.x.rem_euclid(CHUNK_SIZE), self.y.rem_euclid(CHUNK_SIZE))
    }

    /// The neighbouring tile in `dir` on the same deck.
    pub fn step(self, dir: Dir) -> Self {
        let (dx, dy) = dir.offset();
        Self::new(self.x + dx, self.y + dy, self.z)
    }
}

impl ChunkPos {
    /// The tile at the chunk's lowest x/y corner.
    pub fn origin(self) -> TilePos {
        TilePos::new(self.x * CHUNK_SIZE, self.y * CHUNK_SIZE, self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_tiles_map_to_negative_chunks() {
        let t = TilePos::new(-1, -16, 2);
        assert_eq!(t.chunk(), ChunkPos { x: -1, y: -1, z: 2 });
        assert_eq!(t.local(), (15, 0));
    }

    #[test]
    fn chunk_origin_round_trips() {
        for x in -40..40 {
            for y in -40..40 {
                let t = TilePos::new(x, y, 0);
                let (lx, ly) = t.local();
                let o = t.chunk().origin();
                assert_eq!(TilePos::new(o.x + lx, o.y + ly, 0), t);
            }
        }
    }
}
