//! Light cast geometrically (`docs/reference/lighting.md`, The light on a tile): each light
//! lights the area it can see past walls and closed doors, worked out as a polygon from
//! the edges in its way, not tile by tile, so its shadows have true edges. Inside, a light
//! of strength `s` gives `s − distance` (in tiles) per colour; lights add up, capped by the
//! strongest light reaching a point, and `FULL_LIGHT` is fully lit, as
//! `rs13_simulation::lighting` works it out per tile for game rules.
//!
//! Positions are in tiles, with tile centres on whole numbers.

use rs13_simulation::lighting::FULL_LIGHT;

/// How finely the light is worked out: texels along a tile.
pub const TEXELS_PER_TILE: usize = 16;
/// How far shadow edges are softened, in texels each way.
pub const SOFTEN: usize = 2;

/// The area a light at `origin` lights within `reach` tiles: a polygon around it, its
/// corners in order of angle. Empty if the light is inside something opaque.
pub fn visibility_polygon(origin: [f32; 2], reach: f32, opaque: &impl Fn(i32, i32) -> bool) -> Vec<[f32; 2]> {
    let tile = |v: f32| v.round() as i32;
    if opaque(tile(origin[0]), tile(origin[1])) {
        return Vec::new();
    }
    let r = reach.ceil() as i32 + 1;
    let (ox, oy) = (tile(origin[0]), tile(origin[1]));
    let mut segments: Vec<([f32; 2], [f32; 2])> = Vec::new();
    for y in oy - r..=oy + r {
        for x in ox - r..=ox + r {
            if !opaque(x, y) {
                continue;
            }
            let (l, rt, b, t) = (x as f32 - 0.5, x as f32 + 0.5, y as f32 - 0.5, y as f32 + 0.5);
            // Only the faces with open space beyond them can stop light.
            if !opaque(x - 1, y) {
                segments.push(([l, b], [l, t]));
            }
            if !opaque(x + 1, y) {
                segments.push(([rt, b], [rt, t]));
            }
            if !opaque(x, y - 1) {
                segments.push(([l, b], [rt, b]));
            }
            if !opaque(x, y + 1) {
                segments.push(([l, t], [rt, t]));
            }
        }
    }
    // A box just past its reach closes the polygon.
    let e = reach + 1.0;
    let corners = [
        [origin[0] - e, origin[1] - e],
        [origin[0] + e, origin[1] - e],
        [origin[0] + e, origin[1] + e],
        [origin[0] - e, origin[1] + e],
    ];
    for i in 0..4 {
        segments.push((corners[i], corners[(i + 1) % 4]));
    }

    let mut angles: Vec<f32> = Vec::with_capacity(segments.len() * 6);
    for (a, b) in &segments {
        for p in [a, b] {
            let angle = (p[1] - origin[1]).atan2(p[0] - origin[0]);
            angles.extend([angle - 1e-4, angle, angle + 1e-4]);
        }
    }
    angles.sort_by(f32::total_cmp);
    angles.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    angles
        .into_iter()
        .map(|angle| {
            let dir = [angle.cos(), angle.sin()];
            let t = segments
                .iter()
                .filter_map(|(a, b)| ray_hits(origin, dir, *a, *b))
                .fold(f32::INFINITY, f32::min);
            [origin[0] + dir[0] * t, origin[1] + dir[1] * t]
        })
        .collect()
}

/// How far along the ray from `o` in direction `d` it meets the segment `a`–`b`, ends
/// included.
fn ray_hits(o: [f32; 2], d: [f32; 2], a: [f32; 2], b: [f32; 2]) -> Option<f32> {
    let s = [b[0] - a[0], b[1] - a[1]];
    let denom = d[0] * s[1] - d[1] * s[0];
    if denom.abs() < 1e-9 {
        return None;
    }
    let w = [a[0] - o[0], a[1] - o[1]];
    let t = (w[0] * s[1] - w[1] * s[0]) / denom;
    let u = (w[0] * d[1] - w[1] * d[0]) / denom;
    (t >= 0.0 && (-1e-5..=1.0 + 1e-5).contains(&u)).then_some(t)
}

/// The light over a deck, worked out finely: per texel, what the lights add up to and the
/// strongest light reaching it.
#[derive(Debug, Clone, Default)]
pub struct LightRaster {
    /// Size in tiles.
    pub width: usize,
    pub height: usize,
    sum: Vec<[f32; 3]>,
    cap: Vec<[f32; 3]>,
}

impl LightRaster {
    pub fn new(width: usize, height: usize) -> Self {
        let n = width * height * TEXELS_PER_TILE * TEXELS_PER_TILE;
        Self {
            width,
            height,
            sum: vec![[0.0; 3]; n],
            cap: vec![[0.0; 3]; n],
        }
    }

    fn texels(&self) -> (usize, usize) {
        (self.width * TEXELS_PER_TILE, self.height * TEXELS_PER_TILE)
    }

    /// The centre of texel `(i, j)`, in tiles (`j` counts up from the bottom).
    fn texel_centre(i: usize, j: usize) -> [f32; 2] {
        let k = TEXELS_PER_TILE as f32;
        [-0.5 + (i as f32 + 0.5) / k, -0.5 + (j as f32 + 0.5) / k]
    }

    /// Add a light at the centre of tile `(x, y)` with strengths `rgb`, in tiles of reach.
    pub fn add_light(&mut self, x: i32, y: i32, rgb: [f32; 3], opaque: &impl Fn(i32, i32) -> bool) {
        let reach = rgb.into_iter().fold(0.0_f32, f32::max);
        if reach <= 0.0 {
            return;
        }
        let origin = [x as f32, y as f32];
        let polygon = visibility_polygon(origin, reach, opaque);
        if polygon.len() < 3 {
            return;
        }
        // The polygon's corners go round the light: a point is lit if it lies on the light's
        // side of the edge spanning its direction. Directions are compared by a cheap
        // stand-in for the angle, so the corners are put in its order.
        let mut polygon = polygon;
        polygon.sort_by(|a, b| {
            pseudo_angle(a[0] - origin[0], a[1] - origin[1])
                .total_cmp(&pseudo_angle(b[0] - origin[0], b[1] - origin[1]))
        });
        let angles: Vec<f32> = polygon
            .iter()
            .map(|p| pseudo_angle(p[0] - origin[0], p[1] - origin[1]))
            .collect();
        let lit = |p: [f32; 2]| {
            let a = pseudo_angle(p[0] - origin[0], p[1] - origin[1]);
            let n = polygon.len();
            // The last corner at or before this angle, wrapping round.
            let i = angles
                .partition_point(|x| *x <= a)
                .checked_sub(1)
                .unwrap_or(n - 1);
            let tri = [origin, polygon[i], polygon[(i + 1) % n]];
            in_triangle(p, tri)
        };
        let (tw, th) = self.texels();
        let k = TEXELS_PER_TILE as f32;
        let to_texel = |v: f32| ((v + 0.5) * k).floor().max(0.0) as usize;
        let (i0, i1) = (
            to_texel(origin[0] - reach),
            to_texel(origin[0] + reach).min(tw - 1),
        );
        let (j0, j1) = (
            to_texel(origin[1] - reach),
            to_texel(origin[1] + reach).min(th - 1),
        );
        for j in j0..=j1 {
            for ti in i0..=i1 {
                let p = Self::texel_centre(ti, j);
                let dist = ((p[0] - origin[0]).powi(2) + (p[1] - origin[1]).powi(2)).sqrt();
                let left: [f32; 3] = std::array::from_fn(|c| rgb[c] - dist);
                if left.iter().all(|v| *v <= 0.0) || !lit(p) {
                    continue;
                }
                let idx = j * tw + ti;
                for c in 0..3 {
                    self.sum[idx][c] += left[c].max(0.0);
                    self.cap[idx][c] = self.cap[idx][c].max(rgb[c]);
                }
            }
        }
    }

    /// Light tile `(x, y)` fully, as space is.
    pub fn light_fully(&mut self, x: i32, y: i32) {
        let (tw, th) = self.texels();
        for dj in 0..TEXELS_PER_TILE {
            for di in 0..TEXELS_PER_TILE {
                let (i, j) = (
                    x as usize * TEXELS_PER_TILE + di,
                    y as usize * TEXELS_PER_TILE + dj,
                );
                if i < tw && j < th {
                    let idx = j * tw + i;
                    self.sum[idx] = [FULL_LIGHT; 3];
                    self.cap[idx] = [FULL_LIGHT; 3];
                }
            }
        }
    }

    /// The light in texel `(i, j)`, each colour from 0 to 1.
    pub fn texel(&self, i: usize, j: usize) -> [f32; 3] {
        let idx = j * self.texels().0 + i;
        std::array::from_fn(|c| (self.sum[idx][c].min(self.cap[idx][c]) / FULL_LIGHT).clamp(0.0, 1.0))
    }

    /// The light in every texel, rows from the bottom up, with shadow edges softened over
    /// [`SOFTEN`] texels each way, as real shadows have a soft edge.
    pub fn softened(&self) -> Vec<[f32; 3]> {
        let (tw, th) = self.texels();
        let mut levels: Vec<[f32; 3]> = (0..th)
            .flat_map(|j| (0..tw).map(move |i| (i, j)))
            .map(|(i, j)| self.texel(i, j))
            .collect();
        // A box blur, across then up.
        for (step, len, lines) in [(1, tw, th), (tw, th, tw)] {
            let first = |line: usize| if step == 1 { line * tw } else { line };
            let mut out = levels.clone();
            for line in 0..lines {
                let at = |m: usize| levels[first(line) + m * step];
                // A running sum over the window, moved along one texel at a time.
                let mut acc = [0.0_f32; 3];
                for m in 0..SOFTEN.min(len) {
                    let v = at(m);
                    for c in 0..3 {
                        acc[c] += v[c];
                    }
                }
                for n in 0..len {
                    if n + SOFTEN < len {
                        let v = at(n + SOFTEN);
                        for c in 0..3 {
                            acc[c] += v[c];
                        }
                    }
                    if n > SOFTEN {
                        let v = at(n - SOFTEN - 1);
                        for c in 0..3 {
                            acc[c] -= v[c];
                        }
                    }
                    let count = ((n + SOFTEN).min(len - 1) - n.saturating_sub(SOFTEN) + 1) as f32;
                    out[first(line) + n * step] = acc.map(|a| a / count);
                }
            }
            levels = out;
        }
        levels
    }

    /// The light at a point, in tiles; dark off the deck.
    pub fn at(&self, p: [f32; 2]) -> [f32; 3] {
        let k = TEXELS_PER_TILE as f32;
        let (tw, th) = self.texels();
        let (i, j) = (((p[0] + 0.5) * k).floor(), ((p[1] + 0.5) * k).floor());
        if i < 0.0 || j < 0.0 || i as usize >= tw || j as usize >= th {
            return [0.0; 3];
        }
        self.texel(i as usize, j as usize)
    }
}

/// A stand-in for the angle of `(dx, dy)`, from 0 to 4 anticlockwise from the x axis: it
/// rises with the angle, without trigonometry.
fn pseudo_angle(dx: f32, dy: f32) -> f32 {
    let p = dy / (dx.abs() + dy.abs()).max(f32::MIN_POSITIVE);
    match (dx < 0.0, dy < 0.0) {
        (false, false) => p,
        (true, _) => 2.0 - p,
        (false, true) => 4.0 + p,
    }
}

/// Whether `p` is inside the triangle (edges included, either winding).
fn in_triangle(p: [f32; 2], t: [[f32; 2]; 3]) -> bool {
    let side = |a: [f32; 2], b: [f32; 2]| (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
    let (d0, d1, d2) = (side(t[0], t[1]), side(t[1], t[2]), side(t[2], t[0]));
    let neg = d0 < -1e-6 || d1 < -1e-6 || d2 < -1e-6;
    let pos = d0 > 1e-6 || d1 > 1e-6 || d2 > 1e-6;
    !(neg && pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn walls(tiles: &'static [(i32, i32)]) -> impl Fn(i32, i32) -> bool {
        move |x, y| tiles.contains(&(x, y))
    }

    #[test]
    fn light_falls_off_with_distance_like_the_tile_rules() {
        let mut raster = LightRaster::new(20, 3);
        raster.add_light(2, 1, [8.0; 3], &walls(&[]));
        let at = |x: f32| raster.at([x, 1.0])[0];
        assert!((at(2.0) - 1.0).abs() < 0.03, "full at the light: {}", at(2.0));
        assert!((at(5.0) - 5.0 / 7.0).abs() < 0.03, "8 − 3 = 5 of 7: {}", at(5.0));
        assert!(at(11.0) < 0.01, "out of reach");
    }

    #[test]
    fn a_wall_casts_a_shadow_with_a_true_edge() {
        // A wall at (4, 1) with the light at (2, 1): straight behind it is dark, but beside
        // the shadow, rows away, it is lit, not cut off tile by tile.
        let mut raster = LightRaster::new(12, 7);
        raster.add_light(2, 3, [8.0; 3], &walls(&[(4, 3)]));
        assert!(raster.at([6.0, 3.0])[0] < 0.01, "behind the wall");
        assert!(raster.at([6.0, 5.0])[0] > 0.2, "past the shadow's edge");
        // The shadow widens with distance: just behind the wall a point half a tile off its
        // axis is lit, further back the same offset is in shadow.
        assert!(raster.at([4.6, 3.45])[0] < 0.01);
        assert!(raster.at([8.0, 3.9])[0] < 0.01);
        assert!(raster.at([8.0, 5.5])[0] > 0.1);
    }

    #[test]
    fn light_does_not_slip_between_walls_that_meet_at_a_corner() {
        let mut raster = LightRaster::new(8, 8);
        raster.add_light(2, 2, [8.0; 3], &walls(&[(3, 2), (2, 3)]));
        assert!(raster.at([3.0, 3.0])[0] < 0.01);
        assert!(raster.at([4.0, 4.0])[0] < 0.01);
    }

    #[test]
    fn lights_add_up_but_no_brighter_than_the_strongest_reaching() {
        let mut raster = LightRaster::new(10, 3);
        raster.add_light(2, 1, [2.0; 3], &walls(&[]));
        raster.add_light(3, 1, [2.0; 3], &walls(&[]));
        // 2 − 0.5 twice would be 3; capped at 2 of 7.
        assert!((raster.at([2.5, 1.0])[0] - 2.0 / 7.0).abs() < 0.02);
    }

    #[test]
    fn a_light_inside_a_wall_lights_nothing() {
        let mut raster = LightRaster::new(5, 5);
        raster.add_light(2, 2, [8.0; 3], &walls(&[(2, 2)]));
        assert_eq!(raster.at([1.0, 2.0]), [0.0; 3]);
    }
}
