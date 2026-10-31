//! Chairs: sit and buckle in, turn them, take them apart. Spec: `docs/reference/chairs.md`.

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, Dir, DisplayName, GridPos, NetId, ObjectSpec, TilePos};

use crate::actions::ClickOutcome;

/// Chair prototypes: Luna's metal chair and its brown comfy chair.
pub fn is_chair(proto: &str) -> bool {
    matches!(proto, "chair" | "comfy-chair-brown")
}

/// A chair, facing `dir`, with whoever is buckled into it.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chair {
    pub dir: Dir,
    pub buckled: Option<Entity>,
}

/// On a mob buckled into a chair: which one. It can't walk away (Luna `canmove = 0`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Buckled(pub Entity);

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
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(spec.id.as_str(), chair_look(dir)),
            Chair { dir, buckled: None },
        ))
        .id()
}

/// Everything clicking chairs needs.
#[derive(SystemParam)]
pub struct ChairWork<'w, 's> {
    chairs: Query<'w, 's, (&'static GridPos, &'static mut Chair)>,
    buckled: Query<'w, 's, &'static Buckled>,
    names: Query<'w, 's, &'static DisplayName>,
}

impl ChairWork<'_, '_> {
    pub fn contains(&self, target: Entity) -> bool {
        self.chairs.contains(target)
    }

    /// Luna's chair: an empty hand unbuckles whoever is in it (`attack_hand`), or buckles
    /// in a mob standing on it (dragging yourself onto it, in Luna), or, from beside it,
    /// turns an empty chair (the "Rotate Chair" verb). A wrench takes it apart into a
    /// metal sheet. The caller checks reach; `seen` messages want the clicker's `name`.
    pub fn use_on(
        &mut self,
        mob: Entity,
        mob_pos: TilePos,
        target: Entity,
        held: Option<&str>,
        commands: &mut Commands,
    ) -> Option<ClickOutcome> {
        let (pos, mut chair) = self.chairs.get_mut(target).ok()?;
        let at = pos.0;
        match held {
            Some("wrench") => {
                if let Some(sitter) = chair.buckled.take() {
                    commands.entity(sitter).remove::<Buckled>();
                }
                commands.entity(target).despawn();
                commands.queue(move |world: &mut World| {
                    let _ = crate::objects::spawn_object(world, &ObjectSpec::new("metal-sheet"), at);
                });
                Some(ClickOutcome::default())
            }
            None => Some(if let Some(sitter) = chair.buckled.take() {
                commands.entity(sitter).remove::<Buckled>();
                if sitter == mob {
                    ClickOutcome {
                        to_mob: Some(LocMsg::new("chair-unbuckled-self")),
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
                ClickOutcome {
                    to_mob: Some(LocMsg::new("chair-buckled-self")),
                    seen: Some(LocMsg::new("chair-buckles")),
                }
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
            commands.entity(sitter).remove::<Buckled>();
        }
        let state = chair_look(chair.dir);
        if look.state != state {
            look.state = state.to_owned();
        }
    }
}
