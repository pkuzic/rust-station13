//! Read BYOND `.dmm` maps and convert a region of one into the RON map format
//! (`rs13_world::TileMap::from_ron`). What each Luna type becomes is decided by a rules
//! file; anything the rules don't map is skipped and counted, so the report shows what
//! content is still missing.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;

use rs13_world::TileDef;
use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("line {line}: {message}")]
    Syntax { line: usize, message: String },
    #[error("the map has no z level {0}")]
    NoLevel(i32),
    #[error("region {0}")]
    Region(String),
    #[error("rules: {0}")]
    Rules(String),
}

/// One atom on a tile: its type path and the variables the map overrides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Atom {
    pub path: String,
    pub vars: BTreeMap<String, String>,
}

/// A parsed `.dmm` file.
#[derive(Debug, Default)]
pub struct Dmm {
    /// Tile key → the atoms on such a tile, in map order.
    pub keys: HashMap<String, Vec<Atom>>,
    /// z level → rows of keys, top row first.
    pub levels: BTreeMap<i32, Vec<Vec<String>>>,
}

impl Dmm {
    /// Parse the classic `.dmm` format: a dictionary of `"key" = (atoms)` lines, then one
    /// `(1,1,z) = {"..."}` block of key rows per z level.
    pub fn parse(text: &str) -> Result<Self, ImportError> {
        let mut dmm = Dmm::default();
        let mut key_len = 0;
        let mut lines = text.lines().enumerate();
        while let Some((index, line)) = lines.next() {
            let line_no = index + 1;
            let syntax = |message: &str| ImportError::Syntax {
                line: line_no,
                message: message.to_owned(),
            };
            if let Some(rest) = line.strip_prefix('"') {
                let (key, body) = rest.split_once("\" = (").ok_or_else(|| syntax("bad key line"))?;
                let body = body
                    .strip_suffix(')')
                    .ok_or_else(|| syntax("key line must end with ')'"))?;
                key_len = key.len();
                let atoms = split_top_level(body, ',')
                    .into_iter()
                    .map(|a| parse_atom(a).ok_or_else(|| syntax("bad atom")))
                    .collect::<Result<Vec<_>, _>>()?;
                dmm.keys.insert(key.to_owned(), atoms);
            } else if line.starts_with('(') && line.ends_with("{\"") {
                let coords = line.trim_start_matches('(').split(')').next().unwrap_or_default();
                let z: i32 = coords
                    .split(',')
                    .nth(2)
                    .and_then(|z| z.trim().parse().ok())
                    .ok_or_else(|| syntax("bad block coordinates"))?;
                if key_len == 0 {
                    return Err(syntax("a level before any keys"));
                }
                let mut rows = Vec::new();
                for (_, row) in lines.by_ref() {
                    if row.starts_with("\"}") {
                        break;
                    }
                    let keys: Vec<String> = row
                        .as_bytes()
                        .chunks(key_len)
                        .map(|k| String::from_utf8_lossy(k).into_owned())
                        .collect();
                    rows.push(keys);
                }
                dmm.levels.insert(z, rows);
            }
        }
        Ok(dmm)
    }
}

/// Split on `sep` outside quotes and braces.
fn split_top_level(text: &str, sep: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let (mut depth, mut quoted, mut start) = (0i32, false, 0);
    for (i, c) in text.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '{' if !quoted => depth += 1,
            '}' if !quoted => depth -= 1,
            c if c == sep && !quoted && depth == 0 => {
                parts.push(&text[start..i]);
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&text[start..]);
    parts
        .into_iter()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect()
}

/// `/obj/thing{a = 1; b = "two"}`
fn parse_atom(text: &str) -> Option<Atom> {
    let (path, vars) = match text.split_once('{') {
        Some((path, rest)) => (path, rest.strip_suffix('}')?),
        None => (text, ""),
    };
    let vars = split_top_level(vars, ';')
        .into_iter()
        .filter_map(|v| v.split_once('='))
        .map(|(name, value)| {
            let value = value.trim();
            let value = value
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .unwrap_or(value);
            (name.trim().to_owned(), value.to_owned())
        })
        .collect();
    Some(Atom {
        path: path.trim().to_owned(),
        vars,
    })
}

/// What to import and what each type becomes. A rules file may name a `base` file whose
/// rules it adds to (`Rules::extend_from`): the type rules shared by every region of a map.
#[derive(Debug, Deserialize)]
pub struct Rules {
    /// Another rules file, relative to this one, whose rules this one adds to.
    #[serde(default)]
    pub base: Option<String>,
    /// Shown in the generated file's header.
    pub source: String,
    pub z: i32,
    /// Columns and rows of the `.dmm` text (0-based, top row first), inclusive.
    pub columns: (usize, usize),
    pub rows: (usize, usize),
    /// Turf type prefixes → tiles. The longest matching prefix wins.
    #[serde(default)]
    pub turfs: Vec<TurfRule>,
    /// Any of these on a tile makes it a window tile.
    #[serde(default)]
    pub windows: Vec<String>,
    #[serde(default)]
    pub window_tile: Option<TileDef>,
    /// Any of these on a tile without a window makes it a grille tile: solid, but air
    /// passes. Without `grille_tile` they are skipped.
    #[serde(default)]
    pub grilles: Vec<String>,
    #[serde(default)]
    pub grille_tile: Option<TileDef>,
    /// Object type prefixes → our prototypes. The longest matching prefix wins.
    #[serde(default)]
    pub objects: Vec<ObjectRule>,
    /// Landmarks where players may appear. The first that matches wins.
    #[serde(default)]
    pub spawns: Vec<SpawnRule>,
    /// More start points, by column and row of the `.dmm` text, for regions without
    /// landmarks.
    #[serde(default)]
    pub spawn_at: Vec<(usize, usize)>,
    /// Types skipped without being reported (decals, landmarks we don't use).
    #[serde(default)]
    pub ignore: Vec<String>,
    /// Area paths → our area ids. Other areas are left out and reported, unless
    /// `auto_areas`: then each gets an id made from its path (`/area/toxins/lab` →
    /// `toxins-lab`).
    #[serde(default)]
    pub areas: BTreeMap<String, String>,
    #[serde(default)]
    pub auto_areas: bool,
    /// Luna access numbers → our access ids.
    #[serde(default)]
    pub access: BTreeMap<u32, String>,
}

impl Rules {
    /// Parse a rules file. `Some(..)` may be left out around optional values.
    pub fn from_ron(text: &str) -> Result<Self, ImportError> {
        ron::Options::default()
            .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
            .from_str(text)
            .map_err(|e| ImportError::Rules(e.to_string()))
    }

    /// Add `base`'s rules under these: its type rules, ignored types, areas and access
    /// numbers come first (these win where both name the same area or number), its window
    /// and grille tiles are used where these have none, and its spawn rules are tried
    /// after these.
    pub fn extend_from(&mut self, base: Rules) {
        fn under<T>(mine: &mut Vec<T>, mut theirs: Vec<T>) {
            theirs.append(mine);
            *mine = theirs;
        }
        under(&mut self.turfs, base.turfs);
        under(&mut self.windows, base.windows);
        under(&mut self.grilles, base.grilles);
        under(&mut self.objects, base.objects);
        under(&mut self.ignore, base.ignore);
        self.spawns.extend(base.spawns);
        self.window_tile = self.window_tile.take().or(base.window_tile);
        self.grille_tile = self.grille_tile.take().or(base.grille_tile);
        for (k, v) in base.areas {
            self.areas.entry(k).or_insert(v);
        }
        for (k, v) in base.access {
            self.access.entry(k).or_insert(v);
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct TurfRule {
    pub path: String,
    pub tile: TileDef,
}

#[derive(Debug, Deserialize)]
pub struct ObjectRule {
    pub path: String,
    /// Our prototype, if the thing itself is imported.
    #[serde(default)]
    pub id: Option<String>,
    /// Read the access list from this variable (Luna `req_access_txt`, numbers split by
    /// `;`).
    #[serde(default)]
    pub access_var: Option<String>,
    /// What it holds, placed loose on its tile (for containers we can't model yet).
    #[serde(default)]
    pub contents: Vec<Placed>,
    /// Keep the atom's direction (BYOND `dir`, south when the map doesn't set it).
    #[serde(default)]
    pub dir: bool,
    /// Read a cable's ends from its `icon_state` (`"1-2"`, with 0 for a knot). The value
    /// is the type's own icon state, used when the map doesn't set one.
    #[serde(default)]
    pub ends_from_state: Option<String>,
    /// Keep the atom's `icon_state`, for things whose look the mapper chose (table edges).
    /// The value is the type's own icon state, used when the map doesn't set one.
    #[serde(default)]
    pub state: Option<String>,
    /// Map variables to keep as the object's settings (`vars`), when the map sets them.
    #[serde(default)]
    pub vars: Vec<String>,
    /// Variables the type itself sets, which the map doesn't repeat (Luna's wall lockers
    /// set their `dir` and pixel offsets in each subtype). The map's own values win.
    #[serde(default)]
    pub type_vars: BTreeMap<String, String>,
}

/// BYOND direction numbers → our direction names.
fn dir_name(byond: &str) -> Option<&'static str> {
    Some(match byond.trim() {
        "1" => "North",
        "2" => "South",
        "4" => "East",
        "8" => "West",
        "5" => "NorthEast",
        "6" => "SouthEast",
        "9" => "NorthWest",
        "10" => "SouthWest",
        _ => return None,
    })
}

#[derive(Debug, Deserialize)]
pub struct SpawnRule {
    pub path: String,
    /// Only landmarks with this `name` (Luna's job start points: "Scientist").
    #[serde(default)]
    pub name: Option<String>,
    /// Placed on the spawn tile (a stand-in for job equipment).
    #[serde(default)]
    pub with: Vec<Placed>,
}

/// An object placed by a rule.
#[derive(Debug, Deserialize)]
pub struct Placed {
    pub id: String,
    #[serde(default)]
    pub access: Vec<String>,
    /// For an ID card waiting at a start point: the job it's for.
    #[serde(default)]
    pub job: Option<String>,
}

impl Placed {
    fn ron(&self) -> String {
        if self.access.is_empty() {
            format!("{:?}", self.id)
        } else {
            let access: Vec<String> = self.access.iter().map(|a| format!("{a:?}")).collect();
            let vars = self
                .job
                .as_ref()
                .map_or(String::new(), |job| format!(", vars: {{\"job\": {job:?}}}"));
            format!("(id: {:?}, access: [{}]{vars})", self.id, access.join(", "))
        }
    }
}

/// The converted map, and what was left out.
#[derive(Debug)]
pub struct Imported {
    pub ron: String,
    /// Type path → how many were skipped.
    pub skipped: BTreeMap<String, usize>,
    pub problems: Vec<String>,
}

/// Whether `path` is `prefix` or a subtype of it.
fn is_a(path: &str, prefix: &str) -> bool {
    path.strip_prefix(prefix)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

fn longest<'a, T>(rules: &'a [T], path: &str, prefix: impl Fn(&T) -> &str) -> Option<&'a T> {
    rules
        .iter()
        .filter(|r| is_a(path, prefix(r)))
        .max_by_key(|r| prefix(r).len())
}

/// Characters for the legend: memorable ones for plain tiles, then a pool.
fn pool() -> impl Iterator<Item = char> {
    let ascii = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWYZ0123456789!$%&*+-/:;<>?@^_`|~";
    ascii
        .chars()
        .chain('\u{0391}'..='\u{03A9}')
        .chain('\u{03B1}'..='\u{03C9}')
        .chain('\u{0410}'..='\u{044F}')
        .chain('\u{00C0}'..='\u{00FF}')
        .chain('\u{0100}'..='\u{017F}')
        // Whole decks have thousands of kinds of tile.
        .chain('\u{4E00}'..='\u{9FFF}')
        .filter(|c| c.is_alphanumeric() || c.is_ascii_punctuation())
}

fn tile_ron(tile: &TileDef) -> String {
    let mut s = format!("(id: {:?}", tile.id);
    if tile.solid {
        s.push_str(", solid: true");
    }
    if let Some(opaque) = tile.opaque {
        let _ = write!(s, ", opaque: Some({opaque})");
    }
    if let Some(airtight) = tile.airtight {
        let _ = write!(s, ", airtight: Some({airtight})");
    }
    s.push(')');
    s
}

/// Convert the rules' region of `dmm` into the RON map format.
pub fn convert(dmm: &Dmm, rules: &Rules) -> Result<Imported, ImportError> {
    let level = dmm.levels.get(&rules.z).ok_or(ImportError::NoLevel(rules.z))?;
    let (c0, c1) = rules.columns;
    let (r0, r1) = rules.rows;
    if c0 > c1 || r0 > r1 || r1 >= level.len() || level.iter().any(|row| c1 >= row.len()) {
        return Err(ImportError::Region(format!(
            "columns {c0}..={c1}, rows {r0}..={r1} do not fit the level"
        )));
    }

    let mut skipped: BTreeMap<String, usize> = BTreeMap::new();
    let mut problems = Vec::new();
    // Legend entry text → its character; the grid as entry texts.
    let mut entries: Vec<String> = Vec::new();
    let mut area_ids: Vec<String> = Vec::new();
    let mut grid: Vec<Vec<(usize, Option<usize>)>> = Vec::new();

    for (r, row) in level.iter().enumerate().take(r1 + 1).skip(r0) {
        let mut out_row = Vec::new();
        for (c, key) in row.iter().enumerate().take(c1 + 1).skip(c0) {
            let atoms = dmm.keys.get(key).ok_or_else(|| ImportError::Syntax {
                line: 0,
                message: format!("unknown key {key:?} at column {c}, row {r}"),
            })?;
            let mut tile: Option<TileDef> = None;
            let mut window = false;
            let mut grille = false;
            let mut spawn = false;
            let mut objects: Vec<String> = Vec::new();
            let mut area = None;
            for atom in atoms {
                let path = atom.path.as_str();
                if is_a(path, "/area") {
                    let auto = rules.auto_areas.then(|| auto_area_id(path)).flatten();
                    match rules.areas.get(path).or(auto.as_ref()) {
                        Some(id) => {
                            let index = area_ids.iter().position(|a| a == id).unwrap_or_else(|| {
                                area_ids.push(id.clone());
                                area_ids.len() - 1
                            });
                            area = Some(index);
                        }
                        None if path == "/area" => {}
                        None => *skipped.entry(path.to_owned()).or_default() += 1,
                    }
                } else if is_a(path, "/turf") {
                    match longest(&rules.turfs, path, |t| &t.path) {
                        Some(rule) => tile = Some(rule.tile.clone()),
                        None => *skipped.entry(path.to_owned()).or_default() += 1,
                    }
                } else if rules.windows.iter().any(|w| is_a(path, w)) {
                    window = true;
                } else if rules.grilles.iter().any(|g| is_a(path, g)) {
                    grille = true;
                } else if let Some(rule) = rules.spawns.iter().find(|s| {
                    is_a(path, &s.path)
                        && s.name
                            .as_deref()
                            .is_none_or(|n| atom.vars.get("name").map(|v| v.trim_matches('"')) == Some(n))
                }) {
                    spawn = true;
                    objects.extend(rule.with.iter().map(Placed::ron));
                } else if let Some(rule) = longest(&rules.objects, path, |o| &o.path) {
                    if let Some(id) = &rule.id {
                        objects.push(object_ron(id, rule, atom, &rules.access, &mut problems, (c, r)));
                    }
                    objects.extend(rule.contents.iter().map(Placed::ron));
                } else if !rules.ignore.iter().any(|i| is_a(path, i)) {
                    *skipped.entry(path.to_owned()).or_default() += 1;
                }
            }
            let mut tile =
                tile.ok_or_else(|| ImportError::Region(format!("no turf at column {c}, row {r}")))?;
            if let Some(window_tile) = rules.window_tile.as_ref().filter(|_| window) {
                tile = window_tile.clone();
            } else if let Some(grille_tile) = rules.grille_tile.as_ref().filter(|_| grille) {
                tile = grille_tile.clone();
            }
            spawn |= rules.spawn_at.contains(&(c, r));
            let mut entry = format!("(tile: {}", tile_ron(&tile));
            if spawn {
                entry.push_str(", spawn: true");
            }
            if !objects.is_empty() {
                let _ = write!(entry, ", objects: [{}]", objects.join(", "));
            }
            entry.push(')');
            let index = entries.iter().position(|e| *e == entry).unwrap_or_else(|| {
                entries.push(entry);
                entries.len() - 1
            });
            out_row.push((index, area));
        }
        grid.push(out_row);
    }

    // Plain tiles get memorable characters; everything else comes from the pool.
    let memorable: [(&str, char); 6] = [
        ("(tile: (id: \"space\"))", ' '),
        ("(tile: (id: \"floor\"))", '.'),
        ("(tile: (id: \"plating\"))", ','),
        ("(tile: (id: \"wall\", solid: true))", '#'),
        ("(tile: (id: \"r-wall\", solid: true))", 'X'),
        ("(tile: (id: \"window\", solid: true, opaque: Some(false)))", '='),
    ];
    let mut chars: Vec<Option<char>> = entries
        .iter()
        .map(|e| memorable.iter().find(|(m, _)| m == e).map(|(_, c)| *c))
        .collect();
    let mut free = pool().filter(|c| !memorable.iter().any(|(_, m)| m == c));
    for slot in chars.iter_mut().filter(|c| c.is_none()) {
        *slot = Some(
            free.next()
                .ok_or_else(|| ImportError::Region("too many kinds of tile for the legend".into()))?,
        );
    }
    let chars: Vec<char> = chars.into_iter().flatten().collect();
    let area_chars: Vec<char> = pool().take(area_ids.len()).collect();
    if area_chars.len() < area_ids.len() {
        return Err(ImportError::Region("too many areas for the legend".into()));
    }

    let mut ron = String::new();
    let _ = writeln!(ron, "// Generated by rs13-dmm-import from {}.", rules.source);
    let _ = writeln!(
        ron,
        "// z {}, columns {c0}..={c1}, rows {r0}..={r1}. Do not edit by hand: change the rules and",
        rules.z
    );
    let _ = writeln!(ron, "// import again.");
    if !skipped.is_empty() {
        let _ = writeln!(ron, "//\n// Skipped (no prototype yet):");
        for (path, n) in &skipped {
            let _ = writeln!(ron, "//   {n:4} {path}");
        }
    }
    ron.push_str("(\n    legend: {\n");
    let mut order: Vec<usize> = (0..entries.len()).collect();
    order.sort_by_key(|i| chars[*i]);
    for i in order {
        let _ = writeln!(ron, "        {:?}: {},", chars[i], entries[i]);
    }
    ron.push_str("    },\n    area_legend: {\n");
    for (id, c) in area_ids.iter().zip(&area_chars) {
        let _ = writeln!(ron, "        {c:?}: {id:?},");
    }
    ron.push_str("    },\n    area_rows: [\n");
    for row in &grid {
        let line: String = row
            .iter()
            .map(|(_, a)| a.map_or(' ', |a| area_chars[a]))
            .collect();
        let _ = writeln!(ron, "        {line:?},");
    }
    ron.push_str("    ],\n    rows: [\n");
    for row in &grid {
        let line: String = row.iter().map(|(e, _)| chars[*e]).collect();
        let _ = writeln!(ron, "        {line:?},");
    }
    ron.push_str("    ],\n)\n");
    Ok(Imported {
        ron,
        skipped,
        problems,
    })
}

/// An area id made from its Luna path: `/area/toxins/lab` → `toxins-lab`.
fn auto_area_id(path: &str) -> Option<String> {
    let rest = path.strip_prefix("/area/")?;
    Some(rest.replace(['/', '_'], "-").to_lowercase())
}

/// One access entry of a Luna door: a number (stray control characters, which some of
/// Luna's map entries carry, are ignored) or a constant's name (`access_engine`).
fn access_id<'a>(entry: &str, access: &'a BTreeMap<u32, String>) -> Option<&'a String> {
    let entry: String = entry.chars().filter(|c| !c.is_control()).collect();
    let entry = entry.trim();
    if let Ok(n) = entry.parse::<u32>() {
        return access.get(&n);
    }
    let name = entry.strip_prefix("access_")?.to_lowercase();
    access.values().find(|id| **id == name)
}

fn object_ron(
    id: &str,
    rule: &ObjectRule,
    atom: &Atom,
    access: &BTreeMap<u32, String>,
    problems: &mut Vec<String>,
    (c, r): (usize, usize),
) -> String {
    let mut fields = Vec::new();
    if let Some(text) = rule.access_var.as_ref().and_then(|v| atom.vars.get(v)) {
        let ids: Vec<String> = text
            .split(';')
            // Luna's 0 is no access at all.
            .filter(|n| !n.trim().is_empty() && n.trim() != "0")
            .filter_map(|n| match access_id(n, access) {
                Some(id) => Some(format!("{id:?}")),
                None => {
                    problems.push(format!(
                        "column {c}, row {r}: unknown access {n:?} on {}",
                        atom.path
                    ));
                    None
                }
            })
            .collect();
        if !ids.is_empty() {
            fields.push(format!("access: [{}]", ids.join(", ")));
        }
    }
    let var = |name: &str| atom.vars.get(name).or_else(|| rule.type_vars.get(name));
    if rule.dir {
        let byond = var("dir").map_or("2", String::as_str);
        match dir_name(byond) {
            Some(name) => fields.push(format!("dir: {name}")),
            None => problems.push(format!("column {c}, row {r}: odd dir {byond:?} on {}", atom.path)),
        }
    }
    if let Some(default) = &rule.ends_from_state {
        let state = atom
            .vars
            .get("icon_state")
            .map_or(default.as_str(), String::as_str);
        let mut ends = Vec::new();
        for end in state.split('-') {
            match end.trim() {
                "0" => {}
                // Up and down (BYOND 16 and 32) lead to other decks: not imported.
                "16" | "32" => problems.push(format!(
                    "column {c}, row {r}: cable {state:?} leads off the deck; that end is left out"
                )),
                n => match dir_name(n) {
                    Some(name) => ends.push(name),
                    None => problems.push(format!("column {c}, row {r}: odd cable {state:?}")),
                },
            }
        }
        fields.push(format!("ends: [{}]", ends.join(", ")));
    }
    if let Some(default) = &rule.state {
        let state = atom
            .vars
            .get("icon_state")
            .map_or(default.as_str(), String::as_str);
        fields.push(format!("state: Some({state:?})"));
    }
    // Every object keeps its pixel offset: it is how maps hang devices on walls.
    let offsets = ["pixel_x", "pixel_y"].into_iter().filter(|name| {
        var(name)
            .and_then(|v| v.trim().parse::<i32>().ok())
            .is_some_and(|v| v != 0)
    });
    let kept: Vec<String> = rule
        .vars
        .iter()
        .map(String::as_str)
        .chain(offsets)
        .filter_map(|name| var(name).map(|value| format!("{name:?}: {:?}", value.trim())))
        .collect();
    if !kept.is_empty() {
        fields.push(format!("vars: {{{}}}", kept.join(", ")));
    }
    // Luna maps lay some pipes over the floor tiles instead of under them.
    if atom.vars.get("level").is_some_and(|l| l.trim() == "2") {
        fields.push("above_floor: true".to_owned());
    }
    if fields.is_empty() {
        format!("{id:?}")
    } else {
        format!("(id: {id:?}, {})", fields.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAP: &str = r#""aa" = (/turf/space,/area)
"ab" = (/turf/simulated/wall/r_wall,/area/engine)
"ac" = (/obj/machinery/door/airlock/glass{name = "Core; Access"; req_access_txt = "10"},/turf/simulated/floor,/area/engine)
"ad" = (/obj/effect/landmark/start{name = "Engineer"},/obj/item/weapon/wrench,/obj/effect/decal/sign,/turf/simulated/floor/plating,/area/engine)
"ae" = (/obj/structure/grille,/obj/structure/window/reinforced{dir = 4},/turf/simulated/floor/plating,/area/engine)
"af" = (/obj/machinery/atmospherics/pipe/simple{dir = 6},/obj/machinery/atmospherics/pipe/simple{level = 2},/turf/simulated/floor,/area/engine)

(1,1,1) = {"
aaabafaa
aaacadae
aaabaaaa
"}
"#;

    fn rules() -> Rules {
        Rules::from_ron(
            r#"(
                source: "test.dmm",
                z: 1,
                columns: (1, 3),
                rows: (0, 2),
                turfs: [
                    (path: "/turf/space", tile: (id: "space")),
                    (path: "/turf/simulated/floor", tile: (id: "floor")),
                    (path: "/turf/simulated/floor/plating", tile: (id: "plating")),
                    (path: "/turf/simulated/wall", tile: (id: "wall", solid: true)),
                    (path: "/turf/simulated/wall/r_wall", tile: (id: "r-wall", solid: true)),
                ],
                windows: ["/obj/structure/window"],
                window_tile: (id: "window", solid: true, opaque: Some(false)),
                grilles: ["/obj/structure/grille"],
                grille_tile: Some((id: "grille", solid: true, opaque: Some(false), airtight: Some(false))),
                objects: [
                    (path: "/obj/machinery/door/airlock", id: "airlock", access_var: Some("req_access_txt")),
                    (path: "/obj/item/weapon/wrench", id: "wrench"),
                    (path: "/obj/machinery/atmospherics/pipe/simple", id: "pipe", dir: true),
                    (path: "/obj/item/weapon/storage/toolbox", contents: [(id: "crowbar"), (id: "wrench")]),
                ],
                spawns: [(path: "/obj/effect/landmark/start", with: [(id: "id-card", access: ["engine"])])],
                ignore: ["/obj/effect/decal"],
                areas: { "/area/engine": "engine" },
                access: { 10: "engine" },
            )"#,
        )
        .unwrap()
    }

    #[test]
    fn parses_keys_vars_and_levels() {
        let dmm = Dmm::parse(MAP).unwrap();
        let door = &dmm.keys["ac"][0];
        assert_eq!(door.path, "/obj/machinery/door/airlock/glass");
        assert_eq!(door.vars["name"], "Core; Access", "quoted ';' is not a separator");
        assert_eq!(door.vars["req_access_txt"], "10");
        assert_eq!(dmm.levels[&1].len(), 3);
        assert_eq!(dmm.levels[&1][1], ["aa", "ac", "ad", "ae"]);
    }

    #[test]
    fn converts_a_region_into_a_loadable_map() {
        let dmm = Dmm::parse(MAP).unwrap();
        let imported = convert(&dmm, &rules()).unwrap();
        assert!(imported.problems.is_empty(), "{:?}", imported.problems);
        assert!(imported.skipped.is_empty(), "{:?}", imported.skipped);
        let map = rs13_world::TileMap::from_ron(&imported.ron, 0).unwrap();
        assert_eq!((map.width, map.height), (3, 3));
        // The middle row: door, wrench on a spawn, window.
        let at = |x, y| map.tile(rs13_world::TilePos::new(x, y, 0)).unwrap().id.clone();
        assert_eq!(at(0, 2), "r-wall");
        assert_eq!(at(1, 1), "plating");
        assert_eq!(at(2, 1), "window");
        assert_eq!(map.spawns, [rs13_world::TilePos::new(1, 1, 0)]);
        let door = &map.objects.iter().find(|(_, o)| o.id == "airlock").unwrap().1;
        assert_eq!(door.access, ["engine"]);
        assert!(map.objects.iter().any(|(_, o)| o.id == "wrench"));
        let card = &map.objects.iter().find(|(_, o)| o.id == "id-card").unwrap();
        assert_eq!(
            (card.0, card.1.access.clone()),
            (map.spawns[0], vec!["engine".to_owned()])
        );
        assert_eq!(map.area(rs13_world::TilePos::new(0, 1, 0)), Some("engine"));
        // Directions come across; an unset one is BYOND's south.
        // Luna's `level = 2` lays a pipe over the floor tiles.
        let pipes: Vec<(Option<rs13_world::Dir>, bool)> = map
            .objects
            .iter()
            .filter(|(_, o)| o.id == "pipe")
            .map(|(_, o)| (o.dir, o.above_floor))
            .collect();
        assert_eq!(
            pipes,
            [
                (Some(rs13_world::Dir::SouthEast), false),
                (Some(rs13_world::Dir::South), true)
            ]
        );
    }

    #[test]
    fn unmapped_types_are_reported() {
        let dmm = Dmm::parse(MAP).unwrap();
        let mut rules = rules();
        rules.objects.retain(|o| o.path != "/obj/item/weapon/wrench");
        rules.ignore.clear();
        let imported = convert(&dmm, &rules).unwrap();
        assert_eq!(imported.skipped["/obj/item/weapon/wrench"], 1);
        assert_eq!(imported.skipped["/obj/effect/decal/sign"], 1);
    }
}

#[cfg(test)]
mod vars_tests {
    use super::*;

    #[test]
    fn rules_keep_the_map_variables_they_name() {
        let rule: ObjectRule =
            ron::from_str(r#"(path: "/obj/filter", id: Some("gas-filter"), vars: ["filter_type", "on"])"#)
                .unwrap();
        let atom = Atom {
            path: "/obj/filter".into(),
            vars: BTreeMap::from([
                ("filter_type".to_owned(), "2".to_owned()),
                ("name".to_owned(), "Nitrogen Filter".to_owned()),
            ]),
        };
        let ron = object_ron(
            "gas-filter",
            &rule,
            &atom,
            &BTreeMap::new(),
            &mut Vec::new(),
            (0, 0),
        );
        assert_eq!(ron, r#"(id: "gas-filter", vars: {"filter_type": "2"})"#);
        let spec: rs13_world::ObjectSpec = ron::from_str(&ron).unwrap();
        assert_eq!(spec.vars["filter_type"], "2");
    }
}
