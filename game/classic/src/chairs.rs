//! Chairs, stools and beds (in Luna one family: a chair is a bed is a stool): sit and
//! buckle in, lie in a bed, turn chairs, take them apart. Spec: `docs/reference/chairs.md`.

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, Dir, DisplayName, GridPos, NetId, ObjectSpec, TilePos};

use crate::actions::ClickOutcome;

/// Chair prototypes: Luna's metal chair and its brown comfy chair.
pub fn is_chair(proto: &str) -> bool {
    matches!(proto, "chair" | "comfy-chair-brown")
}

/// Which member of the family a seat is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seat {
    /// Faces a way, turns, buckles you in sitting.
    Chair,
    /// Just somewhere to sit: no buckling.
    Stool,
    /// Buckles you in lying down.
    Bed,
}

/// The kind of seat a prototype is, if it is one.
pub fn seat(proto: &str) -> Option<Seat> {
    match proto {
        p if is_chair(p) => Some(Seat::Chair),
        "stool" | "barstool" => Some(Seat::Stool),
        "bed" => Some(Seat::Bed),
        _ => None,
    }
}

/// A seat, facing `dir`, with whoever is buckled into it.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chair {
    pub seat: Seat,
    pub dir: Dir,
    pub buckled: Option<Entity>,
}

/// On a mob buckled into a chair or bed: which one. It can't walk away (Luna
/// `canmove = 0`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Buckled(pub Entity);

/// On a mob lying in a bed (Luna `lying = 1`): drawn lying down.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resting;

/// The sprite state for a chair facing `dir`.
pub fn chair_look(dir: Dir) -> &'static str {
    match dir {
        Dir::North => "north",
        Dir::East => "east",
        Dir::West => "west",
        _ => "south",
    }
}

/// BYOND `turn(dir, 90)`: a quarter turn anticlockwise.
fn quarter_turn(dir: Dir) -> Dir {
    match dir {
        Dir::North => Dir::West,
        Dir::West => Dir::South,
        Dir::East => Dir::North,
        _ => Dir::East,
    }
}

pub fn spawn_chair(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    let dir = spec.dir.unwrap_or(Dir::South);
    let seat = seat(&spec.id).unwrap_or(Seat::Chair);
    let state = if seat == Seat::Chair { chair_look(dir) } else { "" };
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(spec.id.as_str(), state),
            Chair {
                seat,
                dir,
                buckled: None,
            },
        ))
        .id()
}

/// Let `sitter` out of their seat (if they are still there: a player who leaves the game
/// is gone).
fn unbuckle(commands: &mut Commands, sitter: Entity) {
    commands.entity(sitter).try_remove::<(Buckled, Resting)>();
}

/// Everything clicking chairs needs.
#[derive(SystemParam)]
pub struct ChairWork<'w, 's> {
    chairs: Query<'w, 's, (&'static GridPos, &'static mut Chair, &'static Appearance)>,
    buckled: Query<'w, 's, &'static Buckled>,
    names: Query<'w, 's, &'static DisplayName>,
}

impl ChairWork<'_, '_> {
    pub fn contains(&self, target: Entity) -> bool {
        self.chairs.contains(target)
    }

    /// Luna's chair and bed: an empty hand unbuckles whoever is in it (`attack_hand`), or
    /// buckles in a mob standing on it (dragging yourself onto it, in Luna), lying down in
    /// a bed, or, from beside it, turns an empty chair (the "Rotate Chair" verb). A stool
    /// does nothing for an empty hand. A wrench takes any of them apart into a metal
    /// sheet. The caller checks reach; `seen` messages want the clicker's `name`.
    pub fn use_on(
        &mut self,
        mob: Entity,
        mob_pos: TilePos,
        target: Entity,
        held: Option<&str>,
        commands: &mut Commands,
    ) -> Option<ClickOutcome> {
        let (pos, mut chair, look) = self.chairs.get_mut(target).ok()?;
        let at = pos.0;
        let what = look.sprite.clone();
        match held {
            Some("wrench") => {
                if let Some(sitter) = chair.buckled.take() {
                    unbuckle(commands, sitter);
                }
                commands.entity(target).despawn();
                commands.queue(move |world: &mut World| {
                    let _ = crate::objects::spawn_object(world, &ObjectSpec::new("metal-sheet"), at);
                });
                Some(ClickOutcome::default())
            }
            None if chair.seat == Seat::Stool => Some(ClickOutcome::default()),
            None => Some(if let Some(sitter) = chair.buckled.take() {
                unbuckle(commands, sitter);
                if sitter == mob {
                    ClickOutcome {
                        to_mob: Some(LocMsg::new("chair-unbuckled-self").arg("seat", what)),
                        seen: Some(LocMsg::new("chair-unbuckles")),
                    }
                } else {
                    ClickOutcome {
                        to_mob: None,
                        seen: Some(LocMsg::new("chair-unbuckled-by").arg(
                            "target",
                            self.names.get(sitter).map(|n| n.0.clone()).unwrap_or_default(),
                        )),
                    }
                }
            } else if mob_pos == at {
                if self.buckled.contains(mob) {
                    return Some(ClickOutcome::default());
                }
                chair.buckled = Some(mob);
                commands.entity(mob).insert(Buckled(target));
                if chair.seat == Seat::Bed {
                    commands.entity(mob).insert(Resting);
                }
                ClickOutcome {
                    to_mob: Some(LocMsg::new("chair-buckled-self").arg("seat", what)),
                    seen: Some(LocMsg::new("chair-buckles")),
                }
            } else if chair.seat == Seat::Bed {
                ClickOutcome::default()
            } else {
                chair.dir = quarter_turn(chair.dir);
                ClickOutcome::default()
            }),
            _ => None,
        }
    }
}

/// A chair shows which way it faces; whoever leaves a chair's tile (blown away, say) is no
/// longer buckled into it.
pub fn update_chairs(
    mut chairs: Query<(&GridPos, &mut Chair, &mut Appearance)>,
    mobs: Query<&GridPos, Without<Chair>>,
    mut commands: Commands,
) {
    for (pos, mut chair, mut look) in &mut chairs {
        if let Some(sitter) = chair.buckled
            && mobs.get(sitter).ok().is_none_or(|p| p.0 != pos.0)
        {
            chair.buckled = None;
            unbuckle(&mut commands, sitter);
        }
        if chair.seat != Seat::Chair {
            continue;
        }
        let state = chair_look(chair.dir);
        if look.state != state {
            look.state = state.to_owned();
        }
    }
}
