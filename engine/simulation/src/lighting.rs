//! Tile lighting: coloured light from point sources.
//!
//! Each light has a red, a green and a blue strength, in tiles of reach. A tile gets
//! what each light it can see has left after the distance to it, summed, capped by the
//! strongest light reaching it and scaled so that [`FULL_LIGHT`] is fully lit. Distance
//! is not rounded to whole tiles, so light fades smoothly. Light passes between decks
//! through open floors, losing one tile of reach per deck.
//!
//! Spec: `docs/reference/lighting.md` (luna:code/modules/power/UltraLight.dm).

use std::collections::HashMap;

use rs13_world::TilePos;

/// The light level at which a tile is fully lit (Luna `ul_Steps`).
pub const FULL_LIGHT: f32 = 7.0;

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
    AIM.iter().any(|&(ox, oy)| clear_line(world, from, to, ox, oy))
}

fn clear_line(world: &impl LightWorld, from: TilePos, to: TilePos, ox: f32, oy: f32) -> bool {
    let (x0, y0) = (from.x as f32, from.y as f32);
    let (x1, y1) = (to.x as f32 + ox, to.y as f32 + oy);
    let steps = ((x1 - x0).abs().max((y1 - y0).abs()) * 4.0).ceil() as i32;
    for i in 1..steps {
        let t = i as f32 / steps as f32;
        let pos = TilePos::new(
            (x0 + (x1 - x0) * t).round() as i32,
            (y0 + (y1 - y0) * t).round() as i32,
            from.z,
        );
        if pos == from || pos == to {
            continue;
        }
        if world.is_opaque(pos) {
            return false;
        }
    }
    true
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
    fn fully_lit_tiles_stay_lit() {
        let mut field = LightField::default();
        field.light_fully(TilePos::new(5, 5, 0));
        assert_eq!(field.level(TilePos::new(5, 5, 0)), [1.0; 3]);
    }
}
