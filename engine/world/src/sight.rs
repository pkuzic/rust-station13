//! What a person can see, shared by the server (which sends only that) and the client
//! (which draws it). Spec: `docs/reference/lighting.md`, Line of sight and Field of view.
//!
//! Positions here are in tiles, with tile centres on whole numbers.

use std::collections::HashSet;

use crate::{Dir, TilePos};

/// How far a person sees, in tiles: Luna's 15 × 15 view.
pub const VIEW_RANGE: i32 = 7;
/// How wide the arc behind a person is, either side of straight back, in degrees.
pub const BLIND_HALF_ANGLE_DEG: f32 = 60.0;

/// Whether every square the straight line from `from` to `to` crosses after the first is
/// one `open` lets through. Squares are walked edge by edge, so none is skipped; where the
/// line passes exactly through a corner, it gets past if either square beside the corner
/// is open: grazing one wall's corner is seen past, squeezing between two that touch is not.
pub fn line_clear(from: [f32; 2], to: [f32; 2], open: impl Fn(i32, i32) -> bool) -> bool {
    // Square edges on whole numbers.
    let a = [from[0] + 0.5, from[1] + 0.5];
    let b = [to[0] + 0.5, to[1] + 0.5];
    let d = [b[0] - a[0], b[1] - a[1]];
    let (mut x, mut y) = (a[0].floor() as i32, a[1].floor() as i32);
    let end = (b[0].floor() as i32, b[1].floor() as i32);
    let axis = |start: f32, cell: i32, delta: f32| -> (i32, f32, f32) {
        if delta > 0.0 {
            (1, (cell as f32 + 1.0 - start) / delta, 1.0 / delta)
        } else if delta < 0.0 {
            (-1, (start - cell as f32) / -delta, -1.0 / delta)
        } else {
            (0, f32::INFINITY, f32::INFINITY)
        }
    };
    let (sx, mut next_x, step_x) = axis(a[0], x, d[0]);
    let (sy, mut next_y, step_y) = axis(a[1], y, d[1]);
    // Never more squares than the walk could cross.
    for _ in 0..=(end.0 - x).abs() + (end.1 - y).abs() {
        if (x, y) == end {
            return true;
        }
        if (next_x - next_y).abs() < 1e-4 {
            if !open(x + sx, y) && !open(x, y + sy) {
                return false;
            }
            (x, y) = (x + sx, y + sy);
            next_x += step_x;
            next_y += step_y;
        } else if next_x < next_y {
            x += sx;
            next_x += step_x;
        } else {
            y += sy;
            next_y += step_y;
        }
        if !open(x, y) {
            return false;
        }
    }
    true
}

/// Whether the centre of `tile` can be seen from `eye` (in tiles): the straight line
/// between them crosses no opaque square but the tile's own.
pub fn sees(eye: [f32; 2], tile: TilePos, opaque: impl Fn(TilePos) -> bool) -> bool {
    line_clear(eye, [tile.x as f32, tile.y as f32], |x, y| {
        (x == tile.x && y == tile.y) || !opaque(TilePos::new(x, y, tile.z))
    })
}

/// Whether a wall or closed door at `tile`, whose own centre isn't in sight, still shows: a floor
/// tile beside it is in sight, or one touching it at the corner of a room. `sees` answers for the
/// centres of other tiles.
pub fn borders_seen(
    tile: TilePos,
    blocks: impl Fn(TilePos) -> bool,
    mut sees: impl FnMut(TilePos) -> bool,
) -> bool {
    let n = |dx: i32, dy: i32| TilePos::new(tile.x + dx, tile.y + dy, tile.z);
    let beside = [(0, 1), (0, -1), (1, 0), (-1, 0)]
        .into_iter()
        .any(|(dx, dy)| !blocks(n(dx, dy)) && sees(n(dx, dy)));
    beside
        || [(1, 1), (1, -1), (-1, 1), (-1, -1)]
            .into_iter()
            .any(|(dx, dy)| !blocks(n(dx, dy)) && blocks(n(dx, 0)) && blocks(n(0, dy)) && sees(n(dx, dy)))
}

/// The tiles seen from `eye` within `range`: those whose centre is in sight, and walls and closed
/// doors (`blocks`) that `borders_seen` lets show. Everything on a seen tile is seen, so the server
/// sends and the client draws by this same test.
pub fn seen_tiles(eye: TilePos, range: i32, blocks: impl Fn(TilePos) -> bool) -> HashSet<TilePos> {
    let centre = [eye.x as f32, eye.y as f32];
    let around: Vec<TilePos> = (-range..=range)
        .flat_map(|dy| (-range..=range).map(move |dx| (dx, dy)))
        .map(|(dx, dy)| TilePos::new(eye.x + dx, eye.y + dy, eye.z))
        .collect();
    let mut seen: HashSet<TilePos> = around
        .iter()
        .copied()
        .filter(|t| sees(centre, *t, &blocks))
        .collect();
    let walls: Vec<TilePos> = around
        .iter()
        .copied()
        .filter(|t| blocks(*t) && !seen.contains(t))
        .filter(|t| borders_seen(*t, &blocks, |p| seen.contains(&p)))
        .collect();
    seen.extend(walls);
    seen
}

/// The unit vector a facing points along.
pub fn facing_vector(dir: Dir) -> [f32; 2] {
    let (x, y) = dir.offset();
    let len = ((x * x + y * y) as f32).sqrt().max(1.0);
    [x as f32 / len, y as f32 / len]
}

/// Whether `p` lies in the blind arc of someone at `eye` facing `facing` (a unit vector).
pub fn behind(eye: [f32; 2], facing: [f32; 2], p: [f32; 2]) -> bool {
    let to = [p[0] - eye[0], p[1] - eye[1]];
    let len = (to[0] * to[0] + to[1] * to[1]).sqrt();
    if len == 0.0 {
        return false;
    }
    let back = -(to[0] * facing[0] + to[1] * facing[1]) / len;
    back > BLIND_HALF_ANGLE_DEG.to_radians().cos()
}

/// Whether someone at `eye` facing `facing` misses a person on `tile` behind their back.
/// Whoever stands within a tile of them is always noticed.
pub fn behind_back(eye: [f32; 2], facing: [f32; 2], tile: TilePos) -> bool {
    let p = [tile.x as f32, tile.y as f32];
    let near = (p[0] - eye[0]).abs() < 1.5 && (p[1] - eye[1]).abs() < 1.5;
    !near && behind(eye, facing, p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn walls(tiles: &[(i32, i32)]) -> impl Fn(TilePos) -> bool + use<> {
        let set: HashSet<(i32, i32)> = tiles.iter().copied().collect();
        move |p| set.contains(&(p.x, p.y))
    }

    #[test]
    fn a_wall_hides_what_is_behind_it() {
        let opaque = walls(&[(2, 0)]);
        assert!(
            sees([0.0, 0.0], TilePos::new(2, 0, 0), &opaque),
            "the wall itself"
        );
        assert!(!sees([0.0, 0.0], TilePos::new(4, 0, 0), &opaque));
        assert!(sees([0.0, 0.0], TilePos::new(0, 3, 0), &opaque));
    }

    #[test]
    fn a_door_diagonally_past_a_wall_corner_is_seen() {
        // The line runs exactly through the corner of the wall beside the eye, on either
        // side of the origin.
        for (ex, ey) in [(0, 0), (32, 18)] {
            let eye = [ex as f32, ey as f32];
            let opaque = walls(&[(ex + 1, ey + 1), (ex + 1, ey), (ex + 1, ey - 1), (ex + 1, ey - 2)]);
            assert!(sees(eye, TilePos::new(ex + 1, ey - 1, 0), &opaque));
            let opaque = walls(&[(ex + 1, ey - 1), (ex + 1, ey), (ex + 1, ey + 1), (ex + 1, ey + 2)]);
            assert!(sees(eye, TilePos::new(ex + 1, ey + 1, 0), &opaque));
        }
    }

    #[test]
    fn sight_does_not_squeeze_between_walls_that_meet_at_a_corner() {
        let opaque = walls(&[(1, 0), (0, 1)]);
        assert!(!sees([0.0, 0.0], TilePos::new(2, 2, 0), &opaque));
        assert!(!sees([0.0, 0.0], TilePos::new(1, 1, 0), &opaque));
    }

    #[test]
    fn the_back_arc_is_hidden_but_not_neighbours() {
        let north = facing_vector(Dir::North);
        assert!(behind([0.0, 0.0], north, [0.0, -3.0]));
        assert!(behind([0.0, 0.0], north, [1.2, -3.0]));
        assert!(!behind([0.0, 0.0], north, [3.0, 0.0]), "beside");
        assert!(!behind([0.0, 0.0], north, [0.0, 3.0]), "ahead");
        assert!(
            !behind_back([0.0, 0.0], north, TilePos::new(0, -1, 0)),
            "right behind"
        );
        assert!(behind_back([0.0, 0.0], north, TilePos::new(0, -3, 0)));
        let ne = facing_vector(Dir::NorthEast);
        assert!(behind([0.0, 0.0], ne, [-3.0, -3.0]));
    }
}
