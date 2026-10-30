//! Reading BYOND `.dmi` icon files.
//!
//! A `.dmi` is a PNG holding a grid of equally sized icons, with a `zTXt` chunk named
//! `Description` that lists the states:
//!
//! ```text
//! # BEGIN DMI
//! version = 4.0
//!     width = 32
//!     height = 32
//! state = "door_opening"
//!     dirs = 1
//!     frames = 6
//!     delay = 2,2,2,2,2,2
//! # END DMI
//! ```
//!
//! Icons are stored left to right, top to bottom, state after state; within a state,
//! frame after frame, and within a frame, one icon per direction in the order
//! south, north, east, west, south-east, south-west, north-east, north-west.

use std::io::Cursor;

pub mod atlas;

/// One decoded image in 8-bit RGBA.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgba {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Rgba {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; (width * height * 4) as usize],
        }
    }

    /// Copy a `w`×`h` block from `src` at (`sx`, `sy`) to (`dx`, `dy`) here.
    pub fn blit(&mut self, src: &Rgba, sx: u32, sy: u32, w: u32, h: u32, dx: u32, dy: u32) {
        for row in 0..h {
            let s = (((sy + row) * src.width + sx) * 4) as usize;
            let d = (((dy + row) * self.width + dx) * 4) as usize;
            let len = (w * 4) as usize;
            self.pixels[d..d + len].copy_from_slice(&src.pixels[s..s + len]);
        }
    }
}

/// One icon state.
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    pub name: String,
    pub dirs: u32,
    pub frames: u32,
    /// Per-frame delay in deciseconds (empty for single frames).
    pub delays: Vec<f32>,
    /// Index of this state's first icon in the sheet.
    pub first_icon: u32,
}

impl State {
    /// Sheet icon index for a frame and a direction index (0 = south, 1 = north, ...).
    pub fn icon(&self, frame: u32, dir: u32) -> u32 {
        self.first_icon + frame * self.dirs + dir
    }
}

#[derive(Debug, Clone)]
pub struct Dmi {
    pub icon_width: u32,
    pub icon_height: u32,
    pub states: Vec<State>,
    pub sheet: Rgba,
}

#[derive(Debug, thiserror::Error)]
pub enum DmiError {
    #[error("PNG: {0}")]
    Png(String),
    #[error("no DMI description in the PNG")]
    NoDescription,
    #[error("bad DMI description: {0}")]
    Description(String),
}

impl Dmi {
    pub fn read(bytes: &[u8]) -> Result<Self, DmiError> {
        let mut decoder = png::Decoder::new(Cursor::new(bytes));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::ALPHA);
        let mut reader = decoder.read_info().map_err(|e| DmiError::Png(e.to_string()))?;
        let text = description(reader.info()).ok_or(DmiError::NoDescription)?;
        let size = reader
            .output_buffer_size()
            .ok_or_else(|| DmiError::Png("image too large".into()))?;
        let mut buf = vec![0; size];
        let out = reader
            .next_frame(&mut buf)
            .map_err(|e| DmiError::Png(e.to_string()))?;
        let sheet = to_rgba(&buf, &out)?;
        let (icon_width, icon_height, states) = parse_description(&text)?;
        Ok(Dmi {
            icon_width,
            icon_height,
            states,
            sheet,
        })
    }

    pub fn state(&self, name: &str) -> Option<&State> {
        self.states.iter().find(|s| s.name == name)
    }

    /// Icons per row in the sheet.
    pub fn columns(&self) -> u32 {
        (self.sheet.width / self.icon_width).max(1)
    }

    /// The icon at sheet `index`, as its own image.
    pub fn icon(&self, index: u32) -> Rgba {
        let (w, h) = (self.icon_width, self.icon_height);
        let mut out = Rgba::new(w, h);
        let (col, row) = (index % self.columns(), index / self.columns());
        if (row + 1) * h <= self.sheet.height {
            out.blit(&self.sheet, col * w, row * h, w, h, 0, 0);
        }
        out
    }
}

fn description(info: &png::Info) -> Option<String> {
    for chunk in &info.compressed_latin1_text {
        if chunk.keyword == "Description" {
            let mut chunk = chunk.clone();
            chunk.decompress_text().ok()?;
            return chunk.get_text().ok();
        }
    }
    info.uncompressed_latin1_text
        .iter()
        .find(|c| c.keyword == "Description")
        .map(|c| c.text.clone())
}

fn to_rgba(buf: &[u8], out: &png::OutputInfo) -> Result<Rgba, DmiError> {
    let (w, h) = (out.width, out.height);
    let mut img = Rgba::new(w, h);
    let px = (w * h) as usize;
    match out.color_type {
        png::ColorType::Rgba => img.pixels.copy_from_slice(&buf[..px * 4]),
        png::ColorType::Rgb => {
            for i in 0..px {
                img.pixels[i * 4..i * 4 + 3].copy_from_slice(&buf[i * 3..i * 3 + 3]);
                img.pixels[i * 4 + 3] = 255;
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for i in 0..px {
                let (g, a) = (buf[i * 2], buf[i * 2 + 1]);
                img.pixels[i * 4..i * 4 + 4].copy_from_slice(&[g, g, g, a]);
            }
        }
        png::ColorType::Grayscale => {
            for (out, g) in img.pixels.as_chunks_mut::<4>().0.iter_mut().zip(&buf[..px]) {
                out.copy_from_slice(&[*g, *g, *g, 255]);
            }
        }
        png::ColorType::Indexed => return Err(DmiError::Png("palette not expanded".into())),
    }
    Ok(img)
}

/// Parse the `Description` text: icon size and states with their sheet positions.
pub fn parse_description(text: &str) -> Result<(u32, u32, Vec<State>), DmiError> {
    let mut width = 32;
    let mut height = 32;
    let mut states: Vec<State> = Vec::new();
    let mut next_icon = 0;
    for line in text.lines() {
        let line = line.trim();
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        let number = || {
            value
                .parse::<u32>()
                .map_err(|e| DmiError::Description(format!("{line}: {e}")))
        };
        match key {
            "width" if states.is_empty() => width = number()?,
            "height" if states.is_empty() => height = number()?,
            "state" => {
                if let Some(last) = states.last() {
                    next_icon = last.first_icon + last.dirs * last.frames;
                }
                states.push(State {
                    name: value.trim_matches('"').to_owned(),
                    dirs: 1,
                    frames: 1,
                    delays: Vec::new(),
                    first_icon: next_icon,
                });
            }
            "dirs" => {
                if let Some(s) = states.last_mut() {
                    s.dirs = number()?;
                }
            }
            "frames" => {
                if let Some(s) = states.last_mut() {
                    s.frames = number()?;
                }
            }
            "delay" => {
                if let Some(s) = states.last_mut() {
                    s.delays = value.split(',').filter_map(|d| d.trim().parse().ok()).collect();
                }
            }
            _ => {}
        }
    }
    Ok((width, height, states))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn states_are_laid_out_one_after_another() {
        let text = "# BEGIN DMI\nversion = 4.0\n\twidth = 32\n\theight = 32\nstate = \"a\"\n\tdirs = 4\n\tframes = 1\nstate = \"b\"\n\tdirs = 1\n\tframes = 3\n\tdelay = 2,2,1\nstate = \"c\"\n\tdirs = 1\n\tframes = 1\n# END DMI";
        let (w, h, states) = parse_description(text).unwrap();
        assert_eq!((w, h), (32, 32));
        assert_eq!(states.len(), 3);
        assert_eq!(states[1].first_icon, 4);
        assert_eq!(states[1].delays, vec![2.0, 2.0, 1.0]);
        assert_eq!(states[2].first_icon, 7);
        assert_eq!(states[0].icon(0, 2), 2, "east facing of the first state");
    }
}
