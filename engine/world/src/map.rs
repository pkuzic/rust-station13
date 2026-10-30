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

    pub fn opposite(self) -> Self {
        match self {
            Dir::North => Dir::South,
            Dir::NorthEast => Dir::SouthWest,
            Dir::East => Dir::West,
            Dir::SouthEast => Dir::NorthWest,
            Dir::South => Dir::North,
            Dir::SouthWest => Dir::NorthEast,
            Dir::West => Dir::East,
            Dir::NorthWest => Dir::SouthEast,
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

/// An object placed by a map: a prototype id plus optional per-instance settings.
/// In map files it is written either as a bare id, `"crowbar"`, or in full,
/// `(id: "airlock", access: ["engine_equip"])` or `(id: "cable", ends: [North, South])`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "ObjectEntry")]
pub struct ObjectSpec {
    pub id: String,
    /// Access ids: required to open (doors) or granted (ID cards).
    pub access: Vec<String>,
    /// Facing, for directional machines such as pumps.
    pub dir: Option<Dir>,
    /// Where a cable runs: two directions, or one for a cable that ends in a knot on its
    /// own tile.
    pub ends: Vec<Dir>,
}

impl ObjectSpec {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            access: Vec::new(),
            dir: None,
            ends: Vec::new(),
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ObjectEntry {
    Id(String),
    Full {
        id: String,
        #[serde(default)]
        access: Vec<String>,
        #[serde(default)]
        dir: Option<Dir>,
        #[serde(default)]
        ends: Vec<Dir>,
    },
}

impl From<ObjectEntry> for ObjectSpec {
    fn from(entry: ObjectEntry) -> Self {
        match entry {
            ObjectEntry::Id(id) => ObjectSpec::new(id),
            ObjectEntry::Full {
                id,
                access,
                dir,
                ends,
            } => ObjectSpec {
                id,
                access,
                dir,
                ends,
            },
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
    /// Whether the tile blocks sight. Defaults to `solid` (walls block both; windows
    /// will set `opaque: Some(false)`).
    #[serde(default)]
    pub opaque: Option<bool>,
}

impl TileDef {
    pub fn is_opaque(&self) -> bool {
        self.opaque.unwrap_or(self.solid)
    }
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
    /// Objects placed by the map, in reading order. Server-side only: clients receive
    /// the spawned objects as entities, so this is not serialized (and `ObjectSpec`'s
    /// map-file shorthand only reads from self-describing formats like RON).
    #[serde(skip)]
    pub objects: Vec<(TilePos, ObjectSpec)>,
    /// Area names; `areas` refers to them by index + 1.
    pub area_names: Vec<String>,
    /// Row-major area of every tile: 0 for none, otherwise index + 1 into `area_names`.
    pub areas: Vec<u16>,
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
    /// Optional second layer, the same size as `rows`: which area each tile is in.
    /// A space character means no area.
    #[serde(default)]
    area_legend: BTreeMap<char, String>,
    #[serde(default)]
    area_rows: Vec<String>,
}

#[derive(Deserialize)]
struct LegendEntry {
    tile: TileDef,
    #[serde(default)]
    spawn: bool,
    /// Prototype ids of objects to place on this tile.
    #[serde(default)]
    objects: Vec<ObjectSpec>,
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
    ///         'c': (tile: (id: "floor"), objects: ["crowbar"]),
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
        let mut objects = Vec::new();
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
                let pos = TilePos::new(x as i32, y as i32, z);
                if entry.spawn {
                    spawns.push(pos);
                }
                objects.extend(entry.objects.iter().map(|spec| (pos, spec.clone())));
            }
        }
        spawns.sort();

        let area_names: Vec<String> = parsed.area_legend.values().cloned().collect();
        let mut areas = vec![0u16; width * height];
        if !parsed.area_rows.is_empty() {
            if parsed.area_rows.len() != height {
                return Err(MapError::Parse(format!(
                    "area_rows has {} rows, rows has {height}",
                    parsed.area_rows.len()
                )));
            }
            for (row_index, row) in parsed.area_rows.iter().enumerate() {
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
                    if symbol == ' ' {
                        continue;
                    }
                    let name = parsed
                        .area_legend
                        .get(&symbol)
                        .ok_or(MapError::UnknownSymbol(symbol))?;
                    let index = area_names
                        .iter()
                        .position(|n| n == name)
                        .expect("from the legend");
                    areas[y * width + x] = index as u16 + 1;
                }
            }
        }

        Ok(Self {
            width: width as u32,
            height: height as u32,
            z,
            palette,
            tiles,
            spawns,
            objects,
            area_names,
            areas,
        })
    }

    /// The area a tile belongs to, if any.
    pub fn area(&self, pos: TilePos) -> Option<&str> {
        if pos.z != self.z
            || pos.x < 0
            || pos.y < 0
            || pos.x >= self.width as i32
            || pos.y >= self.height as i32
        {
            return None;
        }
        let index = self
            .areas
            .get((pos.y as u32 * self.width + pos.x as u32) as usize)?;
        if *index == 0 {
            return None;
        }
        self.area_names.get(*index as usize - 1).map(String::as_str)
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

    /// Whether sight through `pos` is blocked by the map itself. Outside the map: no.
    pub fn is_opaque(&self, pos: TilePos) -> bool {
        self.tile(pos).is_some_and(|t| t.is_opaque())
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
            'c': (tile: (id: "floor"), objects: ["crowbar"]),
            'D': (tile: (id: "floor"), objects: [(id: "airlock", access: ["engine"])]),
        },
        rows: [
            "WWWW",
            "WSDW",
            "Wc.W",
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
        assert_eq!(
            map.objects,
            vec![
                (
                    TilePos::new(2, 2, 0),
                    ObjectSpec {
                        id: "airlock".into(),
                        access: vec!["engine".into()],
                        dir: None,
                        ends: Vec::new(),
                    }
                ),
                (TilePos::new(1, 1, 0), ObjectSpec::new("crowbar")),
            ]
        );
    }

    #[test]
    fn areas_come_from_the_second_layer() {
        let text = ROOM.replace(
            "rows: [",
            r#"area_legend: { 'a': "office", 'b': "closet" },
        area_rows: ["    ", " aa ", " ab ", "    "],
        rows: ["#,
        );
        let map = TileMap::from_ron(&text, 0).unwrap();
        assert_eq!(map.area(TilePos::new(1, 2, 0)), Some("office"));
        assert_eq!(map.area(TilePos::new(2, 1, 0)), Some("closet"));
        assert_eq!(map.area(TilePos::new(0, 0, 0)), None);
        assert_eq!(map.area(TilePos::new(9, 9, 0)), None);
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
        let ragged = ROOM.replace("\"Wc.W\"", "\"Wc.\"");
        assert!(matches!(
            TileMap::from_ron(&ragged, 0),
            Err(MapError::RaggedRow { .. })
        ));
        let unknown = ROOM.replace("\"Wc.W\"", "\"W.?W\"");
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
