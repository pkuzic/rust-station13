//! Firedoors: firelocks that seal one edge of their tile against air and people.
//! Spec: `docs/reference/firedoors.md`.

use std::collections::HashSet;

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_world::{Appearance, Dir, GridPos, MapLayer, ObjectSpec, TilePos};

use crate::actions::ClickOutcome;
use crate::alarms::AreaSeals;
use crate::time::WorldTime;

/// How long a crowbar takes to open a firedoor, and how long its closing animation runs
/// (Luna `sleep(15)`).
pub const CROWBAR_DS: f32 = 15.0;
/// How long a firedoor the alarm opens takes, and its closing animation (Luna
/// `door/open`, `sleep(6)`).
pub const ALARM_DS: f32 = 6.0;

/// Which way a firedoor is moving.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    Opening,
    Closing,
}

/// A border firedoor. It seals the edge between its tile and the tile in `dir` while
/// `closed`. A `blocked` (welded) one can't be moved, by hand or by the alarm.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Firedoor {
    pub closed: bool,
    pub blocked: bool,
    pub dir: Dir,
    /// Moving, until this world time (deciseconds).
    pub anim: Option<(Motion, f32)>,
    /// Whether an air alarm seal covers it.
    pub in_seal: bool,
    /// What the alarm wants it to do once it can: `Some(true)` close, `Some(false)` open.
    pub pending: Option<bool>,
}

impl Firedoor {
    /// Start opening or closing. Closing seals at once; opening seals until it is done.
    fn start(&mut self, close: bool, now: f32, length: f32) {
        if close {
            self.closed = true;
            self.anim = Some((Motion::Closing, now + length));
        } else {
            self.anim = Some((Motion::Opening, now + length));
        }
    }

    /// The edge it seals, if it faces a cardinal direction.
    pub fn edge(&self, at: TilePos) -> Option<(TilePos, Dir)> {
        matches!(self.dir, Dir::North | Dir::South | Dir::East | Dir::West).then_some((at, self.dir))
    }
}

/// Spawn a firedoor, open, facing the edge it seals (Luna's default is south).
pub fn spawn_firedoor(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: rs13_world::NetId) -> Entity {
    let door = Firedoor {
        closed: false,
        blocked: false,
        dir: spec.dir.unwrap_or(Dir::South),
        anim: None,
        in_seal: false,
        pending: None,
    };
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new("firedoor", firedoor_look(&door)),
            door,
        ))
        .id()
}

/// A firedoor's sprite state: `door0-south` open, `door1-south` closed, `doorc0` while
/// opening and `doorc1` while closing (Luna `update_icon`, `do_animate`).
fn firedoor_look(door: &Firedoor) -> String {
    let state = match door.anim {
        Some((Motion::Opening, _)) => "doorc0",
        Some((Motion::Closing, _)) => "doorc1",
        None if door.closed => "door1",
        None => "door0",
    };
    let facing = match door.dir {
        Dir::North => "north",
        Dir::East => "east",
        Dir::West => "west",
        _ => "south",
    };
    format!("{state}-{facing}")
}

/// The closed firedoor edges, both ways round, for movement and wind.
pub fn closed_edges<'a>(
    doors: impl Iterator<Item = (&'a GridPos, &'a Firedoor)>,
) -> HashSet<(TilePos, TilePos)> {
    let mut edges = HashSet::new();
    for (pos, door) in doors {
        if let (true, Some((at, dir))) = (door.closed, door.edge(pos.0)) {
            edges.insert((at, at.step(dir)));
            edges.insert((at.step(dir), at));
        }
    }
    edges
}

/// Whether a step from `from` to the neighbouring `to` crosses a closed edge. A diagonal
/// step is refused only when both ways round it are.
pub fn crosses_closed_edge(edges: &HashSet<(TilePos, TilePos)>, from: TilePos, to: TilePos) -> bool {
    if edges.is_empty() {
        return false;
    }
    let blocked = |a: TilePos, b: TilePos| edges.contains(&(a, b));
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    if dx == 0 || dy == 0 {
        return blocked(from, to);
    }
    let via_x = TilePos::new(from.x + dx, from.y, from.z);
    let via_y = TilePos::new(from.x, from.y + dy, from.z);
    (blocked(from, via_x) || blocked(via_x, to)) && (blocked(from, via_y) || blocked(via_y, to))
}

/// Finish animations, then do what the alarm asked (Luna `firedoor/process`, `nextstate`).
/// A seal or a fire alarm over the area closes its firedoors; when neither covers one any
/// more, it opens (Luna `air_doors_close` / `air_doors_open`, `firealert` / `firereset`).
/// Power plays no part.
pub fn update_firedoors(
    time: Res<WorldTime>,
    seals: Res<AreaSeals>,
    fires: Res<crate::firealarms::AreaFires>,
    map: Single<&MapLayer>,
    mut doors: Query<(&GridPos, &mut Firedoor)>,
) {
    let now = time.deciseconds();
    for (pos, mut door) in &mut doors {
        if let Some((motion, until)) = door.anim
            && now >= until
        {
            if motion == Motion::Opening {
                door.closed = false;
            }
            door.anim = None;
        }
        let covered = crate::alarms::covered_by_seal(&seals, &map.0, pos.0)
            || crate::alarms::covered_by(&fires.0, &map.0, pos.0);
        if covered != door.in_seal {
            door.in_seal = covered;
            door.pending = Some(covered);
        }
        if door.blocked {
            door.pending = None;
            continue;
        }
        if door.anim.is_some() {
            continue;
        }
        match door.pending.take() {
            Some(true) if !door.closed => door.start(true, now, ALARM_DS),
            Some(false) if door.closed => door.start(false, now, ALARM_DS),
            _ => {}
        }
    }
}

/// Keep firedoor sprites in step with them.
pub fn update_firedoor_looks(mut doors: Query<(&Firedoor, &mut Appearance), Changed<Firedoor>>) {
    for (door, mut look) in &mut doors {
        let state = firedoor_look(door);
        if look.state != state {
            look.state = state;
        }
    }
}

/// Everything clicking firedoors needs.
#[derive(SystemParam)]
pub struct FiredoorWork<'w, 's> {
    doors: Query<'w, 's, &'static mut Firedoor>,
    time: Res<'w, WorldTime>,
}

impl FiredoorWork<'_, '_> {
    pub fn contains(&self, target: Entity) -> bool {
        self.doors.contains(target)
    }

    /// Clicking a firedoor (Luna `firedoor/attackby`). A crowbar opens or closes it unless it
    /// is welded or moving; a lit welding tool welds or unwelds a closed, still one. Anything
    /// else does nothing. The caller checks reach.
    pub fn use_on(&mut self, target: Entity, crowbar: bool, lit_welder: bool) -> Option<ClickOutcome> {
        let now = self.time.deciseconds();
        let mut door = self.doors.get_mut(target).ok()?;
        let idle = door.anim.is_none();
        if lit_welder && idle && door.closed {
            door.blocked = !door.blocked;
        } else if crowbar && idle && !door.blocked {
            let close = !door.closed;
            door.start(close, now, CROWBAR_DS);
        }
        Some(ClickOutcome::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn door(dir: Dir) -> Firedoor {
        Firedoor {
            closed: true,
            blocked: false,
            dir,
            anim: None,
            in_seal: false,
            pending: None,
        }
    }

    #[test]
    fn looks_follow_luna_states() {
        let mut d = door(Dir::North);
        assert_eq!(firedoor_look(&d), "door1-north");
        d.closed = false;
        d.dir = Dir::West;
        assert_eq!(firedoor_look(&d), "door0-west");
        d.anim = Some((Motion::Closing, 0.0));
        assert_eq!(firedoor_look(&d), "doorc1-west");
    }

    #[test]
    fn a_closed_edge_blocks_straight_steps_and_diagonals_only_both_ways_round() {
        let at = TilePos::new(5, 5, 0);
        let pos = GridPos(at);
        let d = door(Dir::North);
        let edges = closed_edges([(&pos, &d)].into_iter());
        let north = TilePos::new(5, 6, 0);
        assert!(crosses_closed_edge(&edges, at, north));
        assert!(crosses_closed_edge(&edges, north, at));
        assert!(!crosses_closed_edge(&edges, at, TilePos::new(6, 5, 0)));
        // North-east goes round by the east tile.
        assert!(!crosses_closed_edge(&edges, at, TilePos::new(6, 6, 0)));
    }
}
