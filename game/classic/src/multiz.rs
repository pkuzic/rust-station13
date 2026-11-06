//! Ladders and stairs between decks (Luna `obj/multiz`). Spec: `docs/reference/multiz.md`.
//!
//! - A ladder goes one deck down from its top end and one deck up from its bottom end, to
//!   the same spot: click it.
//! - Stairs are two tiles on each deck: an entry tile and an active tile. Walking into the
//!   active tile takes you to the same spot one deck down (top stairs) or up (bottom
//!   stairs, only from their entry tile). Each deck's entry lies over the other's active
//!   tile, so you arrive on the entry tile.

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_world::{Appearance, Dense, Dir, GridPos, NetId, ObjectSpec, TilePos};

use crate::decks::Decks;
use crate::movement::Mob;

/// The ladder prototype.
pub const LADDER: &str = "ladder";
/// Stair prototypes: Luna's `stairs/active`, `stairs/active/bottom`, `stairs/enter` and
/// `stairs/enter/bottom`.
pub const STAIRS_DOWN: &str = "stairs-down";
pub const STAIRS_UP: &str = "stairs-up";
pub const STAIRS_ENTRY: &str = "stairs-entry";
pub const STAIRS_ENTRY_BOTTOM: &str = "stairs-entry-bottom";

/// A ladder end: `down` from the top end (Luna `istop`), else up.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ladder {
    pub down: bool,
}

/// A stair tile: the deck's top or bottom stairs (Luna `istop`), the active tile or the
/// entry, and the way it faces.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stairs {
    pub top: bool,
    pub active: bool,
    pub dir: Dir,
}

/// The same spot one deck down (`down`) or up. Decks are numbered bottom first.
pub fn other_deck(at: TilePos, down: bool) -> TilePos {
    TilePos::new(at.x, at.y, if down { at.z - 1 } else { at.z + 1 })
}

pub fn is_multiz(proto: &str) -> bool {
    matches!(
        proto,
        LADDER | STAIRS_DOWN | STAIRS_UP | STAIRS_ENTRY | STAIRS_ENTRY_BOTTOM
    )
}

fn dir_name(dir: Dir) -> &'static str {
    match dir {
        Dir::North => "north",
        Dir::East => "east",
        Dir::West => "west",
        _ => "south",
    }
}

/// A stair tile's sprite (Luna `stairs/New`: `ramptop` when it is the top stairs' entry or
/// the bottom stairs' active tile, else `rampbottom`), facing its way.
pub fn stairs_look(stairs: &Stairs) -> String {
    let ramp = if stairs.top ^ stairs.active {
        "top"
    } else {
        "bottom"
    };
    format!("{ramp}-{}", dir_name(stairs.dir))
}

/// Spawn a ladder end or a stair tile.
pub fn spawn_multiz(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    let proto = spec.id.as_str();
    if proto == LADDER {
        // Luna's default is the top end.
        let down = spec.vars.get("istop").is_none_or(|v| v.trim() != "0");
        return world
            .spawn((
                GridPos(pos),
                id,
                Appearance::new(LADDER, if down { "ladderdown" } else { "ladderup" }),
                Ladder { down },
            ))
            .id();
    }
    let stairs = Stairs {
        top: matches!(proto, STAIRS_DOWN | STAIRS_ENTRY),
        active: matches!(proto, STAIRS_DOWN | STAIRS_UP),
        dir: match spec.dir {
            Some(d @ (Dir::North | Dir::South | Dir::East | Dir::West)) => d,
            _ => Dir::South,
        },
    };
    let mut entity = world.spawn((
        GridPos(pos),
        id,
        Appearance::new("stairs", stairs_look(&stairs)),
        stairs,
    ));
    // Active stair tiles are solid: you go through them, not onto them (Luna `density = 1`).
    if stairs.active {
        entity.insert(Dense);
    }
    entity.id()
}

/// Whether someone could stand on `at`: a deck there, no wall, nothing dense, no one.
fn free(decks: &Decks, blockers: &Query<&GridPos, Or<(With<Dense>, With<Mob>)>>, at: TilePos) -> bool {
    decks.deck(at.z).is_some() && !decks.is_solid(at) && !blockers.iter().any(|p| p.0 == at)
}

/// Luna `stairs/active/Bumped`: where walking from `from` into `ahead` takes you, if
/// `ahead` is an active stair tile (`stairs` lists every stair tile near). Bottom stairs only
/// take you up from an entry tile. The caller checks the spot is free.
pub fn stairs_target(stairs: &[(TilePos, Stairs)], from: TilePos, ahead: TilePos) -> Option<TilePos> {
    let (_, active) = stairs.iter().find(|(p, s)| *p == ahead && s.active)?;
    if !active.top && !stairs.iter().any(|(p, s)| *p == from && !s.active) {
        return None;
    }
    Some(other_deck(ahead, active.top))
}

/// Everything climbing a ladder needs.
#[derive(SystemParam)]
pub struct ClimbWork<'w, 's> {
    ladders: Query<'w, 's, (&'static GridPos, &'static Ladder)>,
    blockers: Query<'w, 's, &'static GridPos, Or<(With<Dense>, With<Mob>)>>,
    decks: Decks<'w, 's>,
}

impl ClimbWork<'_, '_> {
    pub fn is_ladder(&self, target: Entity) -> bool {
        self.ladders.contains(target)
    }

    /// Luna `ladder/attack_hand`, with or without something in the hand: where clicking
    /// this ladder takes you, if the spot at its other end is free. The caller checks reach.
    pub fn climb(&self, target: Entity) -> Option<TilePos> {
        let (pos, ladder) = self.ladders.get(target).ok()?;
        let to = other_deck(pos.0, ladder.down);
        free(&self.decks, &self.blockers, to).then_some(to)
    }
}

/// How much brute a person takes landing on the deck below (Luna `adjustBruteLoss(10)`).
pub const FALL_BRUTE: f32 = 10.0;
/// How long a fall takes a person off their feet, in Luna's counts (its `weakened = 5`).
const FALL_COUNTS: f32 = 5.0;

/// Luna `floor/open/Enter`: anything that isn't anchored and stands on open floor drops to the
/// same spot one deck down, if it can go there. Something solid (a person, a closet) needs the
/// spot free of anything solid; an item needs only a floor. A person who lands takes
/// [`FALL_BRUTE`] and is knocked down. A hole over a hole keeps them falling, a deck each tick.
pub fn fall_through_holes(
    holes: Res<crate::decks::Holes>,
    decks: Decks,
    time: Res<crate::time::WorldTime>,
    mut set: ParamSet<(
        (
            crate::pulling::Pullable,
            Query<(Entity, &GridPos, Has<Dense>, Has<Mob>), Without<rs13_world::Ghost>>,
        ),
        Query<(&mut GridPos, Option<&mut crate::health::Body>)>,
    )>,
    mut commands: Commands,
) {
    if holes.0.is_empty() {
        return;
    }
    let falls: Vec<(Entity, TilePos, bool)> = {
        let (pullable, things) = set.p0();
        let solid_at: std::collections::HashSet<TilePos> = things
            .iter()
            .filter(|(_, _, dense, mob)| *dense || *mob)
            .map(|(_, p, ..)| p.0)
            .collect();
        things
            .iter()
            .filter(|(_, at, ..)| holes.0.binary_search(&at.0).is_ok())
            .filter(|(e, ..)| pullable.check(*e))
            .filter_map(|(e, at, dense, mob)| {
                let to = other_deck(at.0, true);
                let lands = decks.deck(to.z).is_some()
                    && !decks.is_solid(to)
                    && !((dense || mob) && solid_at.contains(&to));
                lands.then_some((e, to, mob))
            })
            .collect()
    };
    let until = time.deciseconds() + FALL_COUNTS * crate::combat::COUNT_DS;
    let mut moved = set.p1();
    for (thing, to, mob) in falls {
        let Ok((mut pos, body)) = moved.get_mut(thing) else {
            continue;
        };
        pos.0 = to;
        if mob {
            if let Some(mut body) = body {
                body.take_damage_spread(FALL_BRUTE, 0.0);
            }
            commands
                .entity(thing)
                .remove::<crate::pulling::Pulling>()
                .insert(crate::flashes::Weakened(until));
        }
    }
}
