//! Window doors: glass doors on one edge of a tile that open for whoever has the access,
//! by walking into them or clicking them, and shut again by themselves. Spec:
//! `docs/reference/windoors.md`.

use std::collections::HashSet;

use bevy_ecs::prelude::*;
use rs13_world::{Appearance, Dir, GridPos, NetId, ObjectSpec, TilePos};

use crate::access::{IdCard, Shown};
use crate::actions::in_reach;
use crate::equipment::Equipment;
use crate::hands::Hands;
use crate::movement::MoveIntent;
use crate::sound::SoundEvents;
use crate::time::WorldTime;

pub const WINDOOR: &str = "windoor";
/// The brig's window doors (Luna `door/window/brigdoor`, "Brig Door").
pub const BRIG_WINDOOR: &str = "brig-windoor";
/// How long opening and closing take, ds (Luna `sleep(10)`, the 7-frame flicks).
pub const SWING_DS: f32 = 10.0;
/// How long it stays open, ds: a door anyone may use, and one that wants access (Luna
/// `sleep(50 * tick_multiplier)` and `sleep(20 * tick_multiplier)`).
pub const STAY_OPEN_DS: f32 = 100.0;
pub const STAY_OPEN_SECURE_DS: f32 = 40.0;
/// How long the deny animation shows, ds (3 frames).
pub const DENY_DS: f32 = 3.0;

/// Where a window door is in its swing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Swing {
    Closed,
    /// Opening; passable once `at` comes. Walked into, it shuts again by itself (`auto`).
    Opening {
        at: f32,
        auto: bool,
    },
    /// Open; shuts by itself at `close_at`, if it was walked into.
    Open {
        close_at: Option<f32>,
    },
    /// Closing (it seals at once); done at `at`.
    Closing {
        at: f32,
    },
}

/// A window door: the edge of its tile it stands on, its look (`left`, `right`,
/// `leftsecure`, `rightsecure`), the access it wants, and its swing.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Windoor {
    pub dir: Dir,
    pub base: String,
    pub access: Vec<String>,
    pub swing: Swing,
    /// Showing its deny animation until then, ds.
    pub deny_until: Option<f32>,
}

impl Windoor {
    /// Whether it blocks its edge (Luna `density`): shut, closing, or not yet open.
    pub fn sealed(&self) -> bool {
        !matches!(self.swing, Swing::Open { .. })
    }

    pub fn edge(&self, at: TilePos) -> (TilePos, Dir) {
        (at, self.dir)
    }
}

/// The edges shut window doors seal, both ways round, as movement checks them
/// (`firedoors::crosses_closed_edge`).
#[derive(Resource, Debug, Default)]
pub struct WindoorEdges(pub HashSet<(TilePos, TilePos)>);

/// Everything that seals one edge of a tile and doesn't move: window panes and shut window
/// doors (firedoors are read from their own query).
#[derive(bevy_ecs::system::SystemParam)]
pub struct EdgeBarriers<'w> {
    panes: Res<'w, crate::windows::PaneEdges>,
    windoors: Res<'w, WindoorEdges>,
}

impl EdgeBarriers<'_> {
    pub fn iter(&self) -> impl Iterator<Item = (TilePos, TilePos)> + '_ {
        self.panes.0.iter().chain(self.windoors.0.iter()).copied()
    }
}

pub fn spawn_windoor(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    let dir = match spec.dir {
        Some(d @ (Dir::North | Dir::East | Dir::South | Dir::West)) => d,
        _ => Dir::South,
    };
    let (sprite, bases) = if spec.id == BRIG_WINDOOR {
        (BRIG_WINDOOR, ["leftsecure", "rightsecure"])
    } else {
        (WINDOOR, ["left", "right"])
    };
    // Luna `door/window/New`: a door that wants access takes its look from its icon state.
    let look_var = if spec.access.is_empty() {
        "base_state"
    } else {
        "icon_state"
    };
    let base = spec
        .vars
        .get(look_var)
        .or_else(|| spec.vars.get("base_state"))
        .map(|b| b.trim().trim_matches('"').to_owned())
        .filter(|b| bases.contains(&b.as_str()))
        .unwrap_or_else(|| bases[0].to_owned());
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(sprite, format!("{base}-{}", dir_name(dir))),
            Windoor {
                dir,
                base,
                access: spec.access.clone(),
                swing: Swing::Closed,
                deny_until: None,
            },
        ))
        .id()
}

fn dir_name(dir: Dir) -> &'static str {
    match dir {
        Dir::North => "north",
        Dir::East => "east",
        Dir::West => "west",
        _ => "south",
    }
}

/// Luna's icon state after the base: `open`, the `opening` and `closing` flicks, `deny`.
fn swing_suffix(door: &Windoor) -> &'static str {
    match door.swing {
        _ if door.deny_until.is_some() => "deny",
        Swing::Closed => "",
        Swing::Opening { .. } => "opening",
        Swing::Open { .. } => "open",
        Swing::Closing { .. } => "closing",
    }
}

/// Start opening, with Luna's sound.
fn open(door: &mut Windoor, now: f32, at: TilePos, auto: bool, sounds: &mut SoundEvents) {
    door.swing = Swing::Opening {
        at: now + SWING_DS,
        auto,
    };
    sounds.play(crate::sound::WINDOOR, at, 100.0);
}

/// Clicking a window door within reach (Luna `attackby`, an empty hand too): with the
/// access it opens or shuts, and stays that way; without, a shut one shows its deny
/// animation. Nothing mid-swing.
pub fn click(
    door: &mut Windoor,
    at: TilePos,
    mob_pos: TilePos,
    shown: Shown,
    now: f32,
    sounds: &mut SoundEvents,
) {
    if !in_reach(mob_pos, at) {
        return;
    }
    match door.swing {
        _ if !shown.opens(&door.access) => {
            if door.sealed() {
                door.deny_until = Some(now + DENY_DS);
            }
        }
        Swing::Closed => open(door, now, at, false, sounds),
        Swing::Open { .. } => {
            door.swing = Swing::Closing { at: now + SWING_DS };
            sounds.play(crate::sound::WINDOOR, at, 100.0);
        }
        _ => {}
    }
}

/// Whether a step from `from` to `to` runs into this door's edge.
fn crosses(door_at: TilePos, dir: Dir, from: TilePos, to: TilePos) -> bool {
    let other = door_at.step(dir);
    (from == door_at && to == other) || (from == other && to == door_at)
}

/// Luna `Bumped`: someone walking into a shut window door with its access opens it, and it
/// shuts again by itself.
#[allow(clippy::type_complexity)]
pub fn bump_windoors(
    time: Res<WorldTime>,
    mobs: Query<(&GridPos, &MoveIntent, &Hands, Option<&Equipment>)>,
    cards: Query<&IdCard>,
    mut doors: Query<(&GridPos, &mut Windoor)>,
    mut sounds: ResMut<SoundEvents>,
) {
    let now = time.deciseconds();
    for (pos, intent, hands, equipment) in &mobs {
        let Some(dir) = intent.dir else { continue };
        let to = pos.0.step(dir);
        for (at, mut door) in &mut doors {
            if door.swing != Swing::Closed || !crosses(at.0, door.dir, pos.0, to) {
                continue;
            }
            if Shown::of(Some(hands), equipment, &cards).opens(&door.access) {
                open(&mut door, now, at.0, true, &mut sounds);
            }
        }
    }
}

/// Move every door along its swing, shut the ones whose time is up, and keep the sealed
/// edges and sprites in step.
pub fn run_windoors(
    time: Res<WorldTime>,
    mut doors: Query<(&GridPos, &mut Windoor, &mut Appearance)>,
    mut edges: ResMut<WindoorEdges>,
    mut sounds: ResMut<SoundEvents>,
) {
    let now = time.deciseconds();
    let mut sealed = HashSet::new();
    for (at, mut door, mut look) in &mut doors {
        match door.swing {
            Swing::Opening { at: done, auto } if now >= done => {
                let stay = if door.access.is_empty() {
                    STAY_OPEN_DS
                } else {
                    STAY_OPEN_SECURE_DS
                };
                door.swing = Swing::Open {
                    close_at: auto.then_some(now + stay),
                };
            }
            Swing::Open {
                close_at: Some(close_at),
            } if now >= close_at => {
                door.swing = Swing::Closing { at: now + SWING_DS };
                sounds.play(crate::sound::WINDOOR, at.0, 100.0);
            }
            Swing::Closing { at: done } if now >= done => door.swing = Swing::Closed,
            _ => {}
        }
        if door.deny_until.is_some_and(|t| now >= t) {
            door.deny_until = None;
        }
        if door.sealed() {
            let (tile, dir) = door.edge(at.0);
            sealed.insert((tile, tile.step(dir)));
            sealed.insert((tile.step(dir), tile));
        }
        let state = format!("{}{}-{}", door.base, swing_suffix(&door), dir_name(door.dir));
        if look.state != state {
            look.state = state;
        }
    }
    if edges.0 != sealed {
        edges.0 = sealed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_step_crosses_the_door_edge_either_way() {
        let door = TilePos::new(2, 2, 0);
        assert!(crosses(door, Dir::North, door, TilePos::new(2, 3, 0)));
        assert!(crosses(door, Dir::North, TilePos::new(2, 3, 0), door));
        assert!(!crosses(door, Dir::North, door, TilePos::new(3, 2, 0)));
    }
}
