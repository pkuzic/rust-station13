//! A single-deck tile map and its text format.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::TilePos;

/// One of the eight movement directions. `North` is +y.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Dir {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

impl Dir {
    pub fn offset(self) -> (i32, i32) {
        match self {
            Dir::North => (0, 1),
            Dir::NorthEast => (1, 1),
            Dir::East => (1, 0),
            Dir::SouthEast => (1, -1),
            Dir::South => (0, -1),
            Dir::SouthWest => (-1, -1),
            Dir::West => (-1, 0),
            Dir::NorthWest => (-1, 1),
        }
    }

    /// Direction from a held-keys vector, or `None` when it is zero.
    pub fn from_offset(dx: i32, dy: i32) -> Option<Self> {
        Some(match (dx.signum(), dy.signum()) {
            (0, 1) => Dir::North,
            (1, 1) => Dir::NorthEast,
            (1, 0) => Dir::East,
            (1, -1) => Dir::SouthEast,
            (0, -1) => Dir::South,
            (-1, -1) => Dir::SouthWest,
            (-1, 0) => Dir::West,
            (-1, 1) => Dir::NorthWest,
            _ => return None,
        })
    }

    /// For a diagonal, its two cardinal components (horizontal first).
    pub fn cardinals(self) -> Option<(Dir, Dir)> {
        match self {
            Dir::NorthEast => Some((Dir::East, Dir::North)),
            Dir::SouthEast => Some((Dir::East, Dir::South)),
            Dir::SouthWest => Some((Dir::West, Dir::South)),
            Dir::NorthWest => Some((Dir::West, Dir::North)),
            _ => None,
        }
    }
}

/// A kind of tile in a map's palette.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileDef {
    /// Prototype id, e.g. `"wall"`, `"floor"`, `"space"`.
    pub id: String,
    /// Whether the tile blocks movement.
    #[serde(default)]
    pub solid: bool,
}

/// One deck of tiles. Tiles outside the map count as solid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileMap {
    pub width: u32,
    pub height: u32,
    pub z: i32,
    pub palette: Vec<TileDef>,
    /// Row-major palette indices, row 0 is y = 0 (the bottom row).
    pub tiles: Vec<u16>,
    /// Where players may appear.
    pub spawns: Vec<TilePos>,
}

#[derive(Debug, thiserror::Error)]
pub enum MapError {
    #[error("map text: {0}")]
    Parse(String),
    #[error("row {row} is {len} characters wide, expected {width}")]
    RaggedRow { row: usize, len: usize, width: usize },
    #[error("character {0:?} is not in the legend")]
    UnknownSymbol(char),
    #[error("map has no rows")]
    Empty,
}

/// The on-disk map text format (RON): a legend of characters and an ASCII grid.
#[derive(Deserialize)]
struct MapText {
    legend: BTreeMap<char, LegendEntry>,
    /// Top row first, as drawn.
    rows: Vec<String>,
}

#[derive(Deserialize)]
struct LegendEntry {
    tile: TileDef,
    #[serde(default)]
    spawn: bool,
}

impl TileMap {
    /// Parse the RON map format:
    ///
    /// ```ron
    /// (
    ///     legend: {
    ///         '#': (tile: (id: "wall", solid: true)),
    ///         '.': (tile: (id: "floor")),
    ///         'S': (tile: (id: "floor"), spawn: true),
    ///     },
    ///     rows: ["###", "#S#", "###"],
    /// )
    /// ```
    pub fn from_ron(text: &str, z: i32) -> Result<Self, MapError> {
        let parsed: MapText = ron::from_str(text).map_err(|e| MapError::Parse(e.to_string()))?;
        let width = parsed.rows.first().ok_or(MapError::Empty)?.chars().count();
        let height = parsed.rows.len();

        let mut palette: Vec<TileDef> = Vec::new();
        let mut tiles = vec![0u16; width * height];
        let mut spawns = Vec::new();
        // The text lists the top row first; the map stores y = 0 at the bottom.
        for (row_index, row) in parsed.rows.iter().enumerate() {
            let len = row.chars().count();
            if len != width {
                return Err(MapError::RaggedRow {
                    row: row_index,
                    len,
                    width,
                });
            }
            let y = height - 1 - row_index;
            for (x, symbol) in row.chars().enumerate() {
                let entry = parsed
                    .legend
                    .get(&symbol)
                    .ok_or(MapError::UnknownSymbol(symbol))?;
                let index = match palette.iter().position(|d| *d == entry.tile) {
                    Some(i) => i,
                    None => {
                        palette.push(entry.tile.clone());
                        palette.len() - 1
                    }
                };
                tiles[y * width + x] = index as u16;
                if entry.spawn {
                    spawns.push(TilePos::new(x as i32, y as i32, z));
                }
            }
        }
        spawns.sort();

        Ok(Self {
            width: width as u32,
            height: height as u32,
            z,
            palette,
            tiles,
            spawns,
        })
    }

    pub fn tile(&self, pos: TilePos) -> Option<&TileDef> {
        if pos.z != self.z || pos.x < 0 || pos.y < 0 {
            return None;
        }
        let (x, y) = (pos.x as u32, pos.y as u32);
        if x >= self.width || y >= self.height {
            return None;
        }
        let index = self.tiles[(y * self.width + x) as usize];
        self.palette.get(index as usize)
    }

    /// Whether movement into `pos` is blocked by the map itself.
    pub fn is_solid(&self, pos: TilePos) -> bool {
        self.tile(pos).is_none_or(|t| t.solid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOM: &str = r#"(
        legend: {
            'W': (tile: (id: "wall", solid: true)),
            '.': (tile: (id: "floor")),
            'S': (tile: (id: "floor"), spawn: true),
        },
        rows: [
            "WWWW",
            "WS.W",
            "W..W",
            "WWWW",
        ],
    )"#;

    #[test]
    fn parses_grid_bottom_up() {
        let map = TileMap::from_ron(ROOM, 0).unwrap();
        assert_eq!((map.width, map.height), (4, 4));
        assert_eq!(map.palette.len(), 2);
        // 'S' is drawn on the second row from the top, so y = 2.
        assert_eq!(map.spawns, vec![TilePos::new(1, 2, 0)]);
        assert!(map.is_solid(TilePos::new(0, 0, 0)));
        assert!(!map.is_solid(TilePos::new(2, 1, 0)));
    }

    #[test]
    fn outside_the_map_is_solid() {
        let map = TileMap::from_ron(ROOM, 0).unwrap();
        assert!(map.is_solid(TilePos::new(-1, 1, 0)));
        assert!(map.is_solid(TilePos::new(1, 1, 1)));
        assert!(map.is_solid(TilePos::new(4, 1, 0)));
    }

    #[test]
    fn rejects_ragged_rows_and_unknown_symbols() {
        let ragged = ROOM.replace("\"W..W\"", "\"W..\"");
        assert!(matches!(
            TileMap::from_ron(&ragged, 0),
            Err(MapError::RaggedRow { .. })
        ));
        let unknown = ROOM.replace("\"W..W\"", "\"W.?W\"");
        assert!(matches!(
            TileMap::from_ron(&unknown, 0),
            Err(MapError::UnknownSymbol('?'))
        ));
    }

    #[test]
    fn direction_offsets_round_trip() {
        for dir in [
            Dir::North,
            Dir::NorthEast,
            Dir::East,
            Dir::SouthEast,
            Dir::South,
            Dir::SouthWest,
            Dir::West,
            Dir::NorthWest,
        ] {
            let (dx, dy) = dir.offset();
            assert_eq!(Dir::from_offset(dx, dy), Some(dir));
        }
        assert_eq!(Dir::from_offset(0, 0), None);
    }
}
