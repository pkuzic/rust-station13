//! Import the sprites listed in a manifest from NSV Luna's `.dmi` files into one atlas.
//!
//! Usage: `rs13-dmi-import <luna root> <manifest.ron> <out dir>`
//!
//! Writes `<out dir>/atlas.png`, `<out dir>/atlas.ron` and `<out dir>/ATTRIBUTION.md`.
//! Only the frames named in the manifest are copied.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use rs13_dmi_import::atlas::{Anim, Atlas};
use rs13_dmi_import::{Dmi, Rgba};
use serde::Deserialize;

/// Cells per row in the atlas.
const COLUMNS: u32 = 16;

#[derive(Deserialize)]
struct Manifest {
    /// Shown in the attribution file, e.g. "NSV Luna, github.com/SS13/luna, commit e13cf12".
    source: String,
    licence: String,
    /// Sprite id → our state → where it comes from.
    sprites: BTreeMap<String, BTreeMap<String, Entry>>,
}

#[derive(Deserialize)]
struct Entry {
    /// Path of the `.dmi` relative to the Luna root.
    dmi: String,
    state: String,
    /// Take only this DMI direction index (0 S, 1 N, 2 E, 3 W, 4 SE, 5 SW, 6 NE, 7 NW).
    #[serde(default)]
    dir: Option<u32>,
    /// Added to every colour. Luna tints its near-black hair by adding to it, which the client can't,
    /// so these are brightened here for the client to multiply instead.
    #[serde(default)]
    lift: Option<u8>,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [root, manifest, out] = args.as_slice() else {
        eprintln!("usage: rs13-dmi-import <luna root> <manifest.ron> <out dir>");
        std::process::exit(2);
    };
    if let Err(e) = run(Path::new(root), Path::new(manifest), Path::new(out)) {
        eprintln!("rs13-dmi-import: {e}");
        std::process::exit(1);
    }
}

fn run(root: &Path, manifest_path: &Path, out: &Path) -> Result<(), String> {
    let text = fs::read_to_string(manifest_path).map_err(|e| format!("{}: {e}", manifest_path.display()))?;
    let manifest: Manifest = ron::Options::default()
        .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
        .from_str(&text)
        .map_err(|e| format!("{}: {e}", manifest_path.display()))?;

    let mut dmis: HashMap<String, Dmi> = HashMap::new();
    let mut cells: Vec<Rgba> = Vec::new();
    let mut atlas = Atlas {
        tile: 32,
        columns: COLUMNS,
        sprites: BTreeMap::new(),
    };
    let mut attribution: Vec<(String, String, String)> = Vec::new();

    for (sprite, states) in &manifest.sprites {
        for (our_state, entry) in states {
            if !dmis.contains_key(&entry.dmi) {
                let path: PathBuf = root.join(&entry.dmi);
                let bytes = fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                let dmi = Dmi::read(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
                if dmi.icon_width > 32 || dmi.icon_height > 32 {
                    return Err(format!("{}: icons over 32×32 aren't supported", entry.dmi));
                }
                dmis.insert(entry.dmi.clone(), dmi);
            }
            let dmi = &dmis[&entry.dmi];
            let state = dmi.state(&entry.state).ok_or_else(|| {
                format!(
                    "{}: no state {:?} (for {sprite}:{our_state})",
                    entry.dmi, entry.state
                )
            })?;
            let dirs: Vec<u32> = match entry.dir {
                Some(d) if d < state.dirs => vec![d],
                Some(d) => {
                    return Err(format!(
                        "{}:{} has {} dirs, asked for {d}",
                        entry.dmi, entry.state, state.dirs
                    ));
                }
                None => (0..state.dirs).collect(),
            };
            let mut anim = Anim {
                dirs: dirs.len() as u32,
                frames: state.frames,
                delays: state.delays.clone(),
                cells: Vec::new(),
            };
            for frame in 0..state.frames {
                for dir in &dirs {
                    anim.cells.push(cells.len() as u32);
                    let mut cell = dmi.icon(state.icon(frame, *dir));
                    if let Some(lift) = entry.lift {
                        for pixel in cell.pixels.as_chunks_mut::<4>().0 {
                            for colour in &mut pixel[..3] {
                                *colour = colour.saturating_add(lift);
                            }
                        }
                    }
                    cells.push(cell);
                }
            }
            atlas
                .sprites
                .entry(sprite.clone())
                .or_default()
                .insert(our_state.clone(), anim);
            attribution.push((
                format!("{sprite}:{our_state}"),
                format!("{} : \"{}\"", entry.dmi, entry.state),
                manifest.licence.clone(),
            ));
        }
    }

    // Pack the cells.
    let rows = (cells.len() as u32).div_ceil(COLUMNS).max(1);
    let mut sheet = Rgba::new(COLUMNS * 32, rows * 32);
    for (i, cell) in cells.iter().enumerate() {
        let i = i as u32;
        // A smaller icon sits in the cell's bottom-left corner, as BYOND draws it on a tile.
        let (x, y) = ((i % COLUMNS) * 32, (i / COLUMNS) * 32 + (32 - cell.height));
        sheet.blit(cell, 0, 0, cell.width, cell.height, x, y);
    }

    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    write_png(&out.join("atlas.png"), &sheet)?;
    let ron =
        ron::ser::to_string_pretty(&atlas, ron::ser::PrettyConfig::default()).map_err(|e| e.to_string())?;
    fs::write(out.join("atlas.ron"), ron).map_err(|e| e.to_string())?;

    let mut md = format!(
        "# Asset attribution\n\nGenerated by `rs13-dmi-import` from `client-assets/sprites.ron`. Do not edit by hand.\n\n\
         Every sprite below is taken from {} and is licensed under {}. The atlas\n\
         (`sprites/atlas.png`) is a derivative work under the same licence.\n\n\
         | Sprite:state | Source file : state | Licence |\n|---|---|---|\n",
        manifest.source, manifest.licence
    );
    for (ours, theirs, licence) in &attribution {
        md.push_str(&format!("| `{ours}` | `{theirs}` | {licence} |\n"));
    }
    fs::write(out.join("ATTRIBUTION.md"), md).map_err(|e| e.to_string())?;

    println!(
        "{} cells from {} files into {}",
        cells.len(),
        dmis.len(),
        out.display()
    );
    Ok(())
}

fn write_png(path: &Path, img: &Rgba) -> Result<(), String> {
    let file = fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), img.width, img.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(&img.pixels).map_err(|e| e.to_string())
}
