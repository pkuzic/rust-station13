//! Closets, secure lockers and crates: things to keep items, and people, in.
//! Spec: `docs/reference/closets.md`.

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, Dense, GridPos, Hidden, ObjectSpec, TilePos};
use serde::Deserialize;

use crate::access::{IdCard, Shown};
use crate::actions::{ClickOutcome, Outbox};
use crate::equipment::Equipment;
use crate::hands::{Hands, HeldBy, Item};
use crate::movement::Mob;
use crate::rng::GameRng;

/// Things a closet takes in when it closes, at most (Luna `storage_capacity`).
pub const STORAGE_CAPACITY: usize = 50;

/// A closet, locker or crate prototype from `content/closets.ron`.
#[derive(Debug, Clone, Deserialize)]
pub struct ClosetDef {
    pub id: String,
    #[serde(default, rename = "crate")]
    pub is_crate: bool,
    pub closed: String,
    pub open: String,
    #[serde(default)]
    pub locked: Option<String>,
    #[serde(default)]
    pub access: Vec<String>,
    #[serde(default)]
    pub contents: Vec<ContentRoll>,
}

/// One thing a closet may start with.
#[derive(Debug, Clone, Deserialize)]
pub struct ContentRoll {
    pub id: String,
    #[serde(default = "always")]
    pub chance: f32,
    #[serde(default)]
    pub or: Option<String>,
}

fn always() -> f32 {
    100.0
}

/// Closet prototypes, by id.
#[derive(Resource, Debug)]
pub struct ClosetProtos(pub HashMap<String, ClosetDef>);

impl Default for ClosetProtos {
    fn default() -> Self {
        let protos: Vec<ClosetDef> =
            rs13_content::parse_ron("content/closets.ron", include_str!("../content/closets.ron"))
                .expect("built-in closet prototypes are valid");
        Self(protos.into_iter().map(|p| (p.id.clone(), p)).collect())
    }
}

/// Whether a sprite is a closet, locker or crate: their ids start `closet`, `locker-` or
/// `crate` (checked against `content/closets.ron` in a test).
pub fn is_closet(sprite: &str) -> bool {
    ["closet", "crate"]
        .iter()
        .any(|k| sprite == *k || sprite.strip_prefix(k).is_some_and(|r| r.starts_with('-')))
        || sprite.starts_with("locker-")
        || sprite.starts_with("wardrobe-")
}

/// A closet, locker or crate, and what is inside it.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Closet {
    pub opened: bool,
    pub welded: bool,
    pub is_crate: bool,
    pub contents: Vec<Entity>,
    pub closed_look: String,
    pub open_look: String,
}

/// The card lock on a secure locker.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct ClosetLock {
    pub locked: bool,
    pub access: Vec<String>,
    pub locked_look: String,
}

/// Inside this closet (or morgue drawer). Items inside have no `GridPos`; people keep theirs
/// and are hidden.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InCloset(pub Entity);

/// People inside a closet who tried to walk out this tick (Luna `relaymove`).
#[derive(Resource, Debug, Default)]
pub struct PendingClosetExits(pub Vec<(Entity, Entity)>);

/// Spawn a closet with what it starts with inside (Luna `New()`). `None` if `proto` is no
/// closet.
pub fn spawn_closet(
    world: &mut World,
    spec: &ObjectSpec,
    pos: TilePos,
    id: rs13_world::NetId,
) -> Option<Result<Entity, String>> {
    world.init_resource::<ClosetProtos>();
    world.init_resource::<GameRng>();
    let def = world.resource::<ClosetProtos>().0.get(&spec.id).cloned()?;
    let closet = Closet {
        opened: false,
        welded: false,
        is_crate: def.is_crate,
        contents: Vec::new(),
        closed_look: def.closed.clone(),
        open_look: def.open.clone(),
    };
    let lock = def.locked.as_ref().map(|look| ClosetLock {
        locked: true,
        access: def.access.clone(),
        locked_look: look.clone(),
    });
    let look = closet_look(&closet, lock.as_ref());
    let mut entity = world.spawn((
        GridPos(pos),
        id,
        Appearance::new(def.id.as_str(), look),
        closet,
        Dense,
    ));
    if let Some(lock) = lock {
        entity.insert(lock);
    }
    let entity = entity.id();
    for roll in &def.contents {
        let hit = world.resource_mut::<GameRng>().prob(roll.chance);
        let Some(proto) = (if hit { Some(&roll.id) } else { roll.or.as_ref() }) else {
            continue;
        };
        let item = match crate::objects::spawn_object(world, &ObjectSpec::new(proto.as_str()), pos) {
            Ok(item) => item,
            Err(e) => return Some(Err(e)),
        };
        put_inside(world, entity, item);
    }
    Some(Ok(entity))
}

/// Closed closets take in the loose items on their tile once the map is laid out (Luna
/// `closet/New`, after a tick).
pub fn take_in_loose_items(world: &mut World) {
    let mut closets = world.query::<(Entity, &GridPos, &Closet)>();
    let shut: Vec<(Entity, TilePos)> = closets
        .iter(world)
        .filter(|(_, _, c)| !c.opened)
        .map(|(e, p, _)| (e, p.0))
        .collect();
    let mut items = world.query_filtered::<(Entity, &GridPos), With<Item>>();
    let loose: Vec<(Entity, TilePos)> = items.iter(world).map(|(e, p)| (e, p.0)).collect();
    for (closet, pos) in shut {
        for (item, _) in loose.iter().filter(|(_, p)| *p == pos) {
            put_inside(world, closet, *item);
        }
    }
}

/// Put a thing inside a closet or crate, out of the world, as closing it over the thing would.
pub fn put_inside(world: &mut World, closet: Entity, thing: Entity) {
    let is_mob = world.get::<Mob>(thing).is_some();
    let mut e = world.entity_mut(thing);
    if is_mob {
        e.insert((InCloset(closet), Hidden));
    } else {
        e.remove::<GridPos>().insert(InCloset(closet));
    }
    if let Some(mut c) = world.get_mut::<Closet>(closet) {
        c.contents.push(thing);
    }
}

/// Open a closet: everything inside comes out onto its tile (Luna `open` and
/// `dump_contents`). The caller checks it may open.
pub fn open_closet(world: &mut World, closet: Entity) {
    let Some(pos) = world.get::<GridPos>(closet).map(|p| p.0) else {
        return;
    };
    let Some(mut c) = world.get_mut::<Closet>(closet) else {
        return;
    };
    if c.opened {
        return;
    }
    c.opened = true;
    let is_crate = c.is_crate;
    let contents = std::mem::take(&mut c.contents);
    for thing in contents {
        let Ok(mut e) = world.get_entity_mut(thing) else {
            continue;
        };
        if e.contains::<Mob>() {
            e.remove::<(InCloset, Hidden)>();
        } else {
            e.remove::<InCloset>().insert(GridPos(pos));
        }
    }
    // A crate stays in the way when open; a closet doesn't.
    if !is_crate {
        world.entity_mut(closet).remove::<Dense>();
    }
}

/// Close a closet over what is on its tile: items, and people unless it is a crate (Luna
/// `close`). The caller checks it may close.
pub fn close_closet(world: &mut World, closet: Entity) {
    let Some(pos) = world.get::<GridPos>(closet).map(|p| p.0) else {
        return;
    };
    let Some(c) = world.get::<Closet>(closet) else {
        return;
    };
    if !c.opened {
        return;
    }
    let is_crate = c.is_crate;
    let mut items = world.query_filtered::<(Entity, &GridPos), With<Item>>();
    let mut taken: Vec<Entity> = items
        .iter(world)
        .filter(|(_, p)| p.0 == pos)
        .map(|(e, _)| e)
        .collect();
    if !is_crate {
        let mut mobs = world.query_filtered::<(Entity, &GridPos), (With<Mob>, Without<InCloset>)>();
        taken.extend(mobs.iter(world).filter(|(_, p)| p.0 == pos).map(|(e, _)| e));
    }
    taken.truncate(STORAGE_CAPACITY);
    if let Some(mut c) = world.get_mut::<Closet>(closet) {
        c.opened = false;
    }
    for thing in taken {
        put_inside(world, closet, thing);
    }
    world.entity_mut(closet).insert(Dense);
}

/// A closet's sprite state (Luna `update_icon`): open, locked or closed, and the weld.
fn closet_look(closet: &Closet, lock: Option<&ClosetLock>) -> String {
    if closet.opened {
        return closet.open_look.clone();
    }
    let mut look = match lock {
        Some(lock) if lock.locked => lock.locked_look.clone(),
        _ => closet.closed_look.clone(),
    };
    if closet.welded {
        look.push_str("+welded");
    }
    look
}

/// Keep closet sprites in step with them.
pub fn update_closet_looks(
    mut closets: Query<
        (&Closet, Option<&ClosetLock>, &mut Appearance),
        Or<(Changed<Closet>, Changed<ClosetLock>)>,
    >,
) {
    for (closet, lock, mut look) in &mut closets {
        let state = closet_look(closet, lock);
        if look.state != state {
            look.state = state;
        }
    }
}

/// People inside who walk try to get out (Luna `relaymove`): an unlocked locker opens even
/// when welded, as in Luna; a welded closet won't budge.
pub fn leave_closets(
    mut exits: ResMut<PendingClosetExits>,
    closets: Query<(&Closet, Option<&ClosetLock>)>,
    mut outboxes: Query<&mut Outbox>,
    mut commands: Commands,
) {
    for (mob, closet) in exits.0.drain(..) {
        let Ok((c, lock)) = closets.get(closet) else {
            // Or a morgue drawer (`morgue.md`).
            commands.queue(move |world: &mut World| crate::morgue::leave(world, mob, closet));
            continue;
        };
        if c.opened {
            continue;
        }
        let refused = match lock {
            Some(lock) if lock.locked => Some("locker-locked-inside"),
            Some(_) => None,
            None if c.welded => Some("closet-wont-budge"),
            None => None,
        };
        match refused {
            Some(id) => {
                if let Ok(mut outbox) = outboxes.get_mut(mob) {
                    outbox.0.push(LocMsg::new(id));
                }
            }
            None => {
                commands.queue(move |world: &mut World| open_closet(world, closet));
            }
        }
    }
}

/// Everything clicking closets needs.
#[derive(SystemParam)]
pub struct ClosetWork<'w, 's> {
    closets: Query<'w, 's, (Entity, &'static GridPos, &'static mut Closet)>,
    locks: Query<'w, 's, &'static mut ClosetLock>,
    looks: Query<'w, 's, &'static Appearance>,
    cards: Query<'w, 's, &'static IdCard>,
    worn: Query<'w, 's, &'static Equipment>,
}

impl ClosetWork<'_, '_> {
    pub fn contains(&self, target: Entity) -> bool {
        self.closets.contains(target)
    }

    /// The cards this mob shows. `hands` comes apart from the query, which the caller holds mutably.
    pub(crate) fn shown(&self, mob: Entity, hands: &Hands) -> Shown<'_> {
        Shown::of(Some(hands), self.worn.get(mob).ok(), &self.cards)
    }

    /// Luna `secure_closet/togglelock`: a shut locker locks or unlocks for someone with its
    /// access. `None` when `target` is no locker or is open. The caller checks it is beside
    /// the mob.
    pub fn toggle_lock(&mut self, mob: Entity, hands: &Hands, target: Entity) -> Option<ClickOutcome> {
        let (_, _, closet) = self.closets.get(target).ok()?;
        if closet.opened {
            return None;
        }
        let access = self.locks.get(target).ok()?.access.clone();
        if !self.shown(mob, hands).opens(&access) {
            return Some(ClickOutcome::told("locker-access-denied"));
        }
        let mut lock = self.locks.get_mut(target).ok()?;
        lock.locked = !lock.locked;
        let id = if lock.locked {
            "locker-locked"
        } else {
            "locker-unlocked"
        };
        Some(seen(LocMsg::new(id)))
    }

    /// Clicking a closet, with or without something in the hand. The caller checks reach
    /// and whether the held item is a lit welding tool. Messages in `seen` go to everyone
    /// who can see, the clicker too.
    pub fn use_on(
        &mut self,
        mob: Entity,
        hands: &mut Hands,
        lit_welder: bool,
        target: Entity,
        commands: &mut Commands,
    ) -> Option<ClickOutcome> {
        let (_, pos, closet) = self.closets.get(target).ok()?;
        let (pos, closet) = (pos.0, closet.clone());
        let noun = self
            .looks
            .get(target)
            .map(|a| a.sprite.clone())
            .unwrap_or_default();
        let held = hands.active_item();

        if closet.opened {
            if lit_welder && !closet.is_crate {
                // Cut apart into a metal sheet.
                commands.entity(target).despawn();
                commands.queue(move |world: &mut World| {
                    let _ = crate::objects::spawn_object(world, &ObjectSpec::new("metal-sheet"), pos);
                });
                return Some(seen(LocMsg::new("closet-cut-apart").arg("item", noun)));
            }
            if let Some(item) = held {
                // Put down in it (Luna `drop_item`).
                let active = hands.active;
                *hands.slot_mut(active) = None;
                commands.entity(item).remove::<HeldBy>().insert(GridPos(pos));
                return Some(ClickOutcome::default());
            }
            // Another closet on the tile is in the way (Luna `can_close`).
            let crowded = self.closets.iter().any(|(e, p, _)| p.0 == pos && e != target);
            if crowded {
                return Some(ClickOutcome::told("closet-wont-budge"));
            }
            commands.queue(move |world: &mut World| close_closet(world, target));
            return Some(ClickOutcome::default());
        }

        if lit_welder && !closet.is_crate {
            let welded = !closet.welded;
            if let Ok((_, _, mut c)) = self.closets.get_mut(target) {
                c.welded = welded;
            }
            let id = if welded {
                "closet-welded"
            } else {
                "closet-unwelded"
            };
            return Some(seen(LocMsg::new(id).arg("item", noun)));
        }

        if let Ok(lock) = self.locks.get(target) {
            // An item, or an empty hand on a locker that won't open, works the lock.
            let opens = held.is_none() && !lock.locked && !closet.welded;
            if !opens {
                let access = lock.access.clone();
                if !self.shown(mob, hands).opens(&access) {
                    return Some(ClickOutcome::told("locker-access-denied"));
                }
                let mut lock = self.locks.get_mut(target).ok()?;
                lock.locked = !lock.locked;
                let id = if lock.locked {
                    "locker-locked"
                } else {
                    "locker-unlocked"
                };
                return Some(seen(LocMsg::new(id)));
            }
        } else if closet.welded {
            return Some(ClickOutcome::told("closet-wont-budge"));
        }
        commands.queue(move |world: &mut World| open_closet(world, target));
        Some(ClickOutcome::default())
    }
}

fn seen(msg: LocMsg) -> ClickOutcome {
    ClickOutcome {
        to_mob: None,
        seen: Some(msg),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_closet_prototype_parses() {
        let protos = ClosetProtos::default();
        assert!(protos.0.contains_key("closet-tools"));
        assert!(protos.0["crate"].is_crate);
        assert_eq!(protos.0["locker-engineer"].access, ["engine_equip"]);
        assert!(protos.0.keys().all(|id| is_closet(id)));
        assert!(!is_closet("crateful") && !is_closet("light"));
    }

    #[test]
    fn looks_follow_luna_states() {
        let mut closet = Closet {
            opened: false,
            welded: true,
            is_crate: false,
            contents: Vec::new(),
            closed_look: "secureeng".into(),
            open_look: "secureengopen".into(),
        };
        let mut lock = ClosetLock {
            locked: true,
            access: Vec::new(),
            locked_look: "secureeng1".into(),
        };
        assert_eq!(closet_look(&closet, Some(&lock)), "secureeng1+welded");
        lock.locked = false;
        closet.welded = false;
        assert_eq!(closet_look(&closet, Some(&lock)), "secureeng");
        closet.opened = true;
        assert_eq!(closet_look(&closet, Some(&lock)), "secureengopen");
    }
}
