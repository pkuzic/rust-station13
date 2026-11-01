//! Tile lighting: coloured light from point sources.
//!
//! Each light has a red, a green and a blue strength, in tiles of reach. A tile gets
//! what each light it can see has left after the distance to it, summed, capped by the
//! strongest light reaching it and scaled so that [`FULL_LIGHT`] is fully lit. Distance
//! is not rounded to whole tiles, so light fades smoothly. Light passes between decks
//! through open floors, losing one tile of reach per deck. A tile is dark when even its
//! brightest colour is below [`DARK_BELOW`].
//!
//! Spec: `docs/reference/lighting.md` (luna:code/modules/power/UltraLight.dm).

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use rs13_world::{TileMap, TilePos};

/// The light level at which a tile is fully lit (Luna `ul_Steps`).
pub const FULL_LIGHT: f32 = 7.0;

/// A tile whose brightest colour is below this is dark. A tube keeps tiles out of the dark
/// up to six and a half tiles away.
pub const DARK_BELOW: f32 = 0.2;

/// A tile with no lighting of its own, always fully lit (Luna's space area).
pub const SPACE: &str = "space";

/// A tile with no floor: light passes through it to the deck below.
pub const OPEN_SPACE: &str = "open-space";

/// Red, green and blue: strengths in tiles of reach, or light levels from 0 to 1.
pub type Rgb = [f32; 3];

/// A light: where it is and how strong each colour is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Emitter {
    pub pos: TilePos,
    pub rgb: Rgb,
}

/// What the map tells lighting.
pub trait LightWorld {
    /// Light doesn't pass through this tile (walls, closed doors). The tile itself is
    /// still lit from the sides that see the light.
    fn is_opaque(&self, pos: TilePos) -> bool;
    /// This tile has no floor: light passes between it and the tile below.
    fn is_open_below(&self, pos: TilePos) -> bool;
}

/// Which way light that has left its own deck keeps going.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Vertical {
    Any,
    Down,
    Up,
}

#[derive(Debug, Clone, Copy, Default)]
struct Accum {
    sum: Rgb,
    cap: Rgb,
}

/// Light on every lit tile.
#[derive(Debug, Clone, Default)]
pub struct LightField {
    tiles: HashMap<TilePos, Accum>,
}

impl LightField {
    pub fn compute(world: &impl LightWorld, emitters: &[Emitter]) -> Self {
        let mut field = Self::default();
        for emitter in emitters {
            field.add(world, *emitter);
        }
        field
    }

    /// Add one light's contribution.
    pub fn add(&mut self, world: &impl LightWorld, emitter: Emitter) {
        self.spread(world, emitter, Vertical::Any);
    }

    /// Light a tile fully whatever is around it, as Luna does for areas without
    /// lighting, such as space.
    pub fn light_fully(&mut self, pos: TilePos) {
        self.tiles.insert(
            pos,
            Accum {
                sum: [FULL_LIGHT; 3],
                cap: [FULL_LIGHT; 3],
            },
        );
    }

    /// Light at a tile, each colour from 0 (dark) to 1 (fully lit).
    pub fn level(&self, pos: TilePos) -> Rgb {
        let Some(acc) = self.tiles.get(&pos) else {
            return [0.0; 3];
        };
        std::array::from_fn(|c| (acc.sum[c].min(acc.cap[c]) / FULL_LIGHT).clamp(0.0, 1.0))
    }

    /// How bright a tile is, from 0 to 1: its brightest colour, so that a tile lit only by
    /// a red bulb is as bright as the red light on it.
    pub fn brightness(&self, pos: TilePos) -> f32 {
        self.level(pos).into_iter().fold(0.0, f32::max)
    }

    /// Whether a tile is too dark to see in: its brightness is below [`DARK_BELOW`].
    pub fn is_dark(&self, pos: TilePos) -> bool {
        self.brightness(pos) < DARK_BELOW
    }

    fn spread(&mut self, world: &impl LightWorld, emitter: Emitter, vertical: Vertical) {
        let reach = emitter.rgb.iter().copied().fold(0.0_f32, f32::max);
        if reach <= 0.0 {
            return;
        }
        let r = reach.ceil() as i32;
        let origin = emitter.pos;
        let mut next = Vec::new();
        for dy in -r..=r {
            for dx in -r..=r {
                let dist = ((dx * dx + dy * dy) as f32).sqrt();
                let left: Rgb = std::array::from_fn(|c| emitter.rgb[c] - dist);
                if left.iter().all(|v| *v <= 0.0) {
                    continue;
                }
                let pos = TilePos::new(origin.x + dx, origin.y + dy, origin.z);
                if !can_see(world, origin, pos) {
                    continue;
                }
                let acc = self.tiles.entry(pos).or_default();
                acc.sum = std::array::from_fn(|c| acc.sum[c] + left[c].max(0.0));
                acc.cap = std::array::from_fn(|c| acc.cap[c].max(emitter.rgb[c]));
                if world.is_opaque(pos) {
                    continue;
                }
                // Through a hole in this floor, or in the ceiling above.
                let onward: Rgb = std::array::from_fn(|c| left[c] - 1.0);
                if onward.iter().all(|v| *v <= 0.0) {
                    continue;
                }
                if vertical != Vertical::Up && world.is_open_below(pos) {
                    let below = TilePos::new(pos.x, pos.y, pos.z - 1);
                    next.push((below, onward, Vertical::Down));
                }
                let above = TilePos::new(pos.x, pos.y, pos.z + 1);
                if vertical != Vertical::Down && world.is_open_below(above) {
                    next.push((above, onward, Vertical::Up));
                }
            }
        }
        for (pos, rgb, way) in next {
            self.spread(world, Emitter { pos, rgb }, way);
        }
    }
}

/// The decks of a map and the closed doors on them, as light sees them. Server and client
/// both light the map through this, so they agree on where it is dark.
pub struct MapDecks<'a> {
    decks: HashMap<i32, Deck<'a>>,
}

/// One deck, with what light asks of each tile worked out once.
struct Deck<'a> {
    map: &'a TileMap,
    /// Row-major: walls and closed doors.
    opaque: Vec<bool>,
    /// Row-major: no floor.
    open_below: Vec<bool>,
}

impl Deck<'_> {
    fn index(&self, pos: TilePos) -> Option<usize> {
        let (w, h) = (self.map.width as i32, self.map.height as i32);
        (pos.x >= 0 && pos.y >= 0 && pos.x < w && pos.y < h).then(|| (pos.y * w + pos.x) as usize)
    }
}

impl<'a> MapDecks<'a> {
    /// `opaque`: where things block light besides the map's own tiles, such as closed
    /// airlocks.
    pub fn new(
        decks: impl IntoIterator<Item = &'a TileMap>,
        opaque: impl IntoIterator<Item = TilePos>,
    ) -> Self {
        let extra: HashSet<TilePos> = opaque.into_iter().collect();
        let decks = decks
            .into_iter()
            .map(|map| {
                let positions = (0..map.height as i32)
                    .flat_map(|y| (0..map.width as i32).map(move |x| TilePos::new(x, y, map.z)));
                let opaque = positions
                    .clone()
                    .map(|p| map.is_opaque(p) || extra.contains(&p))
                    .collect();
                let open_below = positions
                    .map(|p| map.tile(p).is_some_and(|t| t.id == OPEN_SPACE))
                    .collect();
                (
                    map.z,
                    Deck {
                        map,
                        opaque,
                        open_below,
                    },
                )
            })
            .collect();
        Self { decks }
    }

    /// The light on every tile from these lights, with space fully lit.
    pub fn light(&self, emitters: &[Emitter]) -> LightField {
        let mut field = LightField::compute(self, emitters);
        for deck in self.decks.values() {
            let deck = deck.map;
            for y in 0..deck.height as i32 {
                for x in 0..deck.width as i32 {
                    let pos = TilePos::new(x, y, deck.z);
                    if deck.tile(pos).is_some_and(|t| t.id == SPACE) {
                        field.light_fully(pos);
                    }
                }
            }
        }
        field
    }
}

impl LightWorld for MapDecks<'_> {
    fn is_opaque(&self, pos: TilePos) -> bool {
        self.decks
            .get(&pos.z)
            .is_some_and(|d| d.index(pos).is_some_and(|i| d.opaque[i]))
    }

    fn is_open_below(&self, pos: TilePos) -> bool {
        self.decks
            .get(&pos.z)
            .is_some_and(|d| d.index(pos).is_some_and(|i| d.open_below[i]))
    }
}

/// Offsets from a tile's centre that sight lines aim at: a tile is seen when the line to
/// its centre or near one of its corners is clear, as BYOND's `view` is generous.
const AIM: [(f32, f32); 5] = [
    (0.0, 0.0),
    (-0.45, -0.45),
    (0.45, -0.45),
    (-0.45, 0.45),
    (0.45, 0.45),
];

/// Whether light from `from` reaches `to` on the same deck: no opaque tile in between.
pub fn can_see(world: &impl LightWorld, from: TilePos, to: TilePos) -> bool {
    if from == to {
        return true;
    }
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    let clear = |path: &[(i32, i32)]| {
        path.iter()
            .all(|&(x, y)| !world.is_opaque(TilePos::new(from.x + x, from.y + y, from.z)))
    };
    match sight_lines().get(dx, dy) {
        Some(paths) => paths.iter().any(|p| clear(p)),
        None => AIM.iter().any(|&(ox, oy)| clear(&line_path(dx, dy, ox, oy))),
    }
}

/// The tiles between a tile and the one `dx`, `dy` from it that the sight line aimed at
/// (`ox`, `oy`) from the far tile's centre crosses, in order, without either end.
fn line_path(dx: i32, dy: i32, ox: f32, oy: f32) -> Vec<(i32, i32)> {
    let (x1, y1) = (dx as f32 + ox, dy as f32 + oy);
    let steps = (x1.abs().max(y1.abs()) * 4.0).ceil() as i32;
    let mut path: Vec<(i32, i32)> = Vec::new();
    for i in 1..steps {
        let t = i as f32 / steps as f32;
        // Halves round up, as rounding the tile's own (positive) coordinates does.
        let p = ((x1 * t + 0.5).floor() as i32, (y1 * t + 0.5).floor() as i32);
        if p == (0, 0) || p == (dx, dy) || path.last() == Some(&p) {
            continue;
        }
        path.push(p);
    }
    path
}

/// Sight lines out to this many tiles each way are worked out once and kept; longer ones,
/// rarer, are traced each time.
const LINES_RADIUS: i32 = 24;

/// The sight-line paths for every offset within [`LINES_RADIUS`]: they depend only on the
/// offset, and lighting asks for them for every lit tile of every light.
struct SightLines(Vec<[Vec<(i32, i32)>; 5]>);

impl SightLines {
    fn get(&self, dx: i32, dy: i32) -> Option<&[Vec<(i32, i32)>; 5]> {
        if dx.abs() > LINES_RADIUS || dy.abs() > LINES_RADIUS {
            return None;
        }
        let side = 2 * LINES_RADIUS + 1;
        self.0
            .get(((dy + LINES_RADIUS) * side + dx + LINES_RADIUS) as usize)
    }
}

fn sight_lines() -> &'static SightLines {
    static LINES: OnceLock<SightLines> = OnceLock::new();
    LINES.get_or_init(|| {
        let r = LINES_RADIUS;
        SightLines(
            (-r..=r)
                .flat_map(|dy| (-r..=r).map(move |dx| (dx, dy)))
                .map(|(dx, dy)| AIM.map(|(ox, oy)| line_path(dx, dy, ox, oy)))
                .collect(),
        )
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[derive(Default)]
    struct Grid {
        walls: HashSet<TilePos>,
        holes: HashSet<TilePos>,
    }

    impl LightWorld for Grid {
        fn is_opaque(&self, pos: TilePos) -> bool {
            self.walls.contains(&pos)
        }
        fn is_open_below(&self, pos: TilePos) -> bool {
            self.holes.contains(&pos)
        }
    }

    fn tube(x: i32, y: i32, z: i32) -> Emitter {
        Emitter {
            pos: TilePos::new(x, y, z),
            rgb: [8.0, 8.0, 8.0],
        }
    }

    #[test]
    fn light_fades_with_distance() {
        let field = LightField::compute(&Grid::default(), &[tube(0, 0, 0)]);
        assert_eq!(field.level(TilePos::new(0, 0, 0)), [1.0; 3]);
        assert_eq!(field.level(TilePos::new(1, 0, 0)), [1.0; 3]);
        let near = field.level(TilePos::new(3, 0, 0))[0];
        let far = field.level(TilePos::new(6, 0, 0))[0];
        assert!((near - 5.0 / 7.0).abs() < 1e-5);
        assert!((far - 2.0 / 7.0).abs() < 1e-5);
        assert_eq!(field.level(TilePos::new(8, 0, 0)), [0.0; 3]);
    }

    #[test]
    fn kept_sight_lines_match_traced_ones_and_long_ones_still_work() {
        // A wall two tiles from a light, off its axis: the kept paths and the traced ones
        // agree on what it hides, and a light reaching past the kept radius still lights
        // and shadows tiles.
        let mut grid = Grid::default();
        grid.walls.insert(TilePos::new(2, 1, 0));
        let from = TilePos::new(0, 0, 0);
        for dy in -6..=6 {
            for dx in -6..=6 {
                let to = TilePos::new(dx, dy, 0);
                let traced = to == from
                    || AIM.iter().any(|&(ox, oy)| {
                        line_path(dx, dy, ox, oy)
                            .iter()
                            .all(|&(x, y)| !grid.is_opaque(TilePos::new(x, y, 0)))
                    });
                assert_eq!(can_see(&grid, from, to), traced, "{dx},{dy}");
            }
        }
        let far = Emitter {
            pos: from,
            rgb: [40.0; 3],
        };
        let field = LightField::compute(&grid, &[far]);
        assert!(
            field.brightness(TilePos::new(30, 0, 0)) > 0.0,
            "lit past the kept radius"
        );
        assert_eq!(
            field.level(TilePos::new(30, 15, 0)),
            [0.0; 3],
            "in the wall's shadow"
        );
    }

    #[test]
    fn colours_are_separate() {
        // Luna's red small light: red 5, no green or blue.
        let red = Emitter {
            pos: TilePos::new(0, 0, 0),
            rgb: [5.0, 0.0, 0.0],
        };
        let [r, g, b] = LightField::compute(&Grid::default(), &[red]).level(TilePos::new(1, 0, 0));
        assert!((r - 4.0 / 7.0).abs() < 1e-5);
        assert_eq!((g, b), (0.0, 0.0));
    }

    #[test]
    fn lights_add_up_to_the_strongest() {
        let dim = |x| Emitter {
            pos: TilePos::new(x, 0, 0),
            rgb: [3.0; 3],
        };
        let field = LightField::compute(&Grid::default(), &[dim(-1), dim(1)]);
        // Each leaves 2 at the middle; together 4, capped at the strongest light, 3.
        assert!((field.level(TilePos::new(0, 0, 0))[0] - 3.0 / 7.0).abs() < 1e-5);
    }

    #[test]
    fn walls_cast_shadows_but_are_lit() {
        let mut grid = Grid::default();
        for y in -5..=5 {
            grid.walls.insert(TilePos::new(2, y, 0));
        }
        let field = LightField::compute(&grid, &[tube(0, 0, 0)]);
        assert!(
            field.level(TilePos::new(2, 0, 0))[0] > 0.5,
            "the wall's face is lit"
        );
        assert_eq!(field.level(TilePos::new(3, 0, 0)), [0.0; 3], "behind it is dark");
        assert_eq!(field.level(TilePos::new(4, 1, 0)), [0.0; 3]);
    }

    #[test]
    fn light_goes_round_open_doorways() {
        let mut grid = Grid::default();
        for y in -5..=5 {
            if y != 0 {
                grid.walls.insert(TilePos::new(2, y, 0));
            }
        }
        let field = LightField::compute(&grid, &[tube(0, 0, 0)]);
        assert!(field.level(TilePos::new(4, 0, 0))[0] > 0.0);
    }

    #[test]
    fn light_falls_through_open_floors_and_rises_through_holes() {
        let mut grid = Grid::default();
        grid.holes.insert(TilePos::new(2, 0, 1));
        // Upstairs, by the hole: lights the deck below, one tile weaker.
        let field = LightField::compute(&grid, &[tube(0, 0, 1)]);
        let above = field.level(TilePos::new(2, 0, 1))[0];
        let below = field.level(TilePos::new(2, 0, 0))[0];
        assert!(below > 0.0 && below < above);
        assert_eq!(
            field.level(TilePos::new(-3, 0, 0)),
            [0.0; 3],
            "only near the hole"
        );
        // Downstairs, under the hole: lights the deck above.
        let field = LightField::compute(&grid, &[tube(0, 0, 0)]);
        assert!(field.level(TilePos::new(2, 0, 1))[0] > 0.0);
        assert_eq!(field.level(TilePos::new(2, 0, 2)), [0.0; 3]);
    }

    #[test]
    fn darkness_starts_past_six_tiles_from_a_tube() {
        let field = LightField::compute(&Grid::default(), &[tube(0, 0, 0)]);
        assert!(!field.is_dark(TilePos::new(6, 0, 0)));
        assert!(field.is_dark(TilePos::new(7, 0, 0)));
        assert!(field.is_dark(TilePos::new(30, 0, 0)), "no light at all");
        // A red bulb lights its neighbours red, and that is light enough.
        let red = Emitter {
            pos: TilePos::new(0, 0, 0),
            rgb: [5.0, 0.0, 0.0],
        };
        let field = LightField::compute(&Grid::default(), &[red]);
        assert!(!field.is_dark(TilePos::new(1, 0, 0)));
        assert!((field.brightness(TilePos::new(1, 0, 0)) - 4.0 / 7.0).abs() < 1e-5);
    }

    const ROOM: &str = r#"(
        legend: {
            ' ': (tile: (id: "space")),
            '#': (tile: (id: "wall", solid: true)),
            '.': (tile: (id: "floor")),
        },
        rows: [
            "           ",
            " ######### ",
            " #.......# ",
            " ######### ",
            "           ",
        ],
    )"#;

    #[test]
    fn map_decks_light_space_and_respect_doors() {
        let map = TileMap::from_ron(ROOM, 0).unwrap();
        // The room's floor is y = 2, x = 2..=8.
        let lamp = Emitter {
            pos: TilePos::new(2, 2, 0),
            rgb: [8.0; 3],
        };
        let open = MapDecks::new([&map], []).light(&[lamp]);
        assert!(!open.is_dark(TilePos::new(7, 2, 0)));
        assert!(!open.is_dark(TilePos::new(0, 0, 0)), "space is always lit");
        // A closed door in the room shades the far end.
        let shut = MapDecks::new([&map], [TilePos::new(4, 2, 0)]).light(&[lamp]);
        assert!(!shut.is_dark(TilePos::new(4, 2, 0)), "the door's face is lit");
        assert!(shut.is_dark(TilePos::new(7, 2, 0)));
    }

    #[test]
    fn fully_lit_tiles_stay_lit() {
        let mut field = LightField::default();
        field.light_fully(TilePos::new(5, 5, 0));
        assert_eq!(field.level(TilePos::new(5, 5, 0)), [1.0; 3]);
    }
}
