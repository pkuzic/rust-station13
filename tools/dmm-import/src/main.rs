//! Import a region of a BYOND `.dmm` map into the RON map format.
//!
//! Usage: `rs13-dmm-import <map.dmm> <rules.ron> <out.ron>`
//!
//! Prints what was skipped (types with no prototype yet) and any problems, such as access
//! numbers the rules don't know.

use std::fs;
use std::path::Path;

use rs13_dmm_import::{Dmm, Rules, convert};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [map, rules, out] = args.as_slice() else {
        eprintln!("usage: rs13-dmm-import <map.dmm> <rules.ron> <out.ron>");
        std::process::exit(2);
    };
    if let Err(e) = run(Path::new(map), Path::new(rules), Path::new(out)) {
        eprintln!("rs13-dmm-import: {e}");
        std::process::exit(1);
    }
}

fn run(map: &Path, rules_path: &Path, out: &Path) -> Result<(), String> {
    // BYOND maps are Latin-1 at worst; keep any odd bytes rather than failing.
    let bytes = fs::read(map).map_err(|e| format!("{}: {e}", map.display()))?;
    let dmm = Dmm::parse(&String::from_utf8_lossy(&bytes)).map_err(|e| format!("{}: {e}", map.display()))?;
    let text = fs::read_to_string(rules_path).map_err(|e| format!("{}: {e}", rules_path.display()))?;
    let rules = Rules::from_ron(&text).map_err(|e| format!("{}: {e}", rules_path.display()))?;
    let imported = convert(&dmm, &rules).map_err(|e| e.to_string())?;
    // Check the result loads before writing it.
    rs13_world::TileMap::from_ron(&imported.ron, 0)
        .map_err(|e| format!("generated map does not load: {e}"))?;
    fs::write(out, &imported.ron).map_err(|e| format!("{}: {e}", out.display()))?;

    let skipped: usize = imported.skipped.values().sum();
    println!(
        "{}: {} kinds of thing skipped ({skipped} in all)",
        out.display(),
        imported.skipped.len()
    );
    for problem in &imported.problems {
        println!("problem: {problem}");
    }
    Ok(())
}
