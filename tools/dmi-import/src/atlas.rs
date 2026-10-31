//! The sprite atlas the client draws from: one PNG of equally sized cells plus this
//! description. Written by `rs13-dmi-import`, read by the client.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Atlas {
    /// Cell size in pixels (square).
    pub tile: u32,
    /// Cells per row in the PNG.
    pub columns: u32,
    /// Sprite id → state → animation.
    pub sprites: BTreeMap<String, BTreeMap<String, Anim>>,
}

/// The cells of one state: `frames` × `dirs`, frame-major (all directions of frame 0,
/// then frame 1, ...), directions in DMI order (S, N, E, W, SE, SW, NE, NW).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Anim {
    pub dirs: u32,
    pub frames: u32,
    /// Deciseconds per frame; empty for a still.
    pub delays: Vec<f32>,
    pub cells: Vec<u32>,
}

impl Anim {
    /// Cell for a frame and a DMI direction index. A diagonal on a four-direction state
    /// shows its east or west side, as BYOND does; other missing directions fall back to
    /// south.
    pub fn cell(&self, frame: u32, dir: u32) -> u32 {
        let dir = match dir {
            d if d < self.dirs => d,
            4 | 6 if self.dirs >= 4 => 2,
            5 | 7 if self.dirs >= 4 => 3,
            _ => 0,
        };
        let frame = frame % self.frames.max(1);
        self.cells[(frame * self.dirs + dir) as usize]
    }

    /// Which frame shows `elapsed_ds` deciseconds into a looping animation.
    pub fn frame_at(&self, elapsed_ds: f32) -> u32 {
        if self.frames <= 1 || self.delays.is_empty() {
            return 0;
        }
        let total: f32 = self.delays.iter().sum();
        let mut t = elapsed_ds.rem_euclid(total.max(0.1));
        for (i, d) in self.delays.iter().enumerate() {
            if t < *d {
                return i as u32;
            }
            t -= d;
        }
        self.frames - 1
    }
}

impl Atlas {
    pub fn get(&self, sprite: &str, state: &str) -> Option<&Anim> {
        let states = self.sprites.get(sprite)?;
        states.get(state).or_else(|| states.get(""))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_follow_their_delays_and_loop() {
        let anim = Anim {
            dirs: 1,
            frames: 3,
            delays: vec![1.0, 2.0, 1.0],
            cells: vec![10, 11, 12],
        };
        assert_eq!(anim.frame_at(0.5), 0);
        assert_eq!(anim.frame_at(1.5), 1);
        assert_eq!(anim.frame_at(3.5), 2);
        assert_eq!(anim.frame_at(4.5), 0, "loops");
        assert_eq!(anim.cell(anim.frame_at(1.5), 0), 11);
    }

    #[test]
    fn missing_directions_fall_back_to_south() {
        let anim = Anim {
            dirs: 1,
            frames: 1,
            delays: vec![],
            cells: vec![7],
        };
        assert_eq!(anim.cell(0, 3), 7);
    }
}
