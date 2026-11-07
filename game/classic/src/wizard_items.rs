//! The wizard's things (Luna `gamemodes/wizard`): the spellbook, the magic scrolls, the
//! teleportation scroll, the staffs and the scrying orb. Spec: `docs/reference/spells.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{
    Appearance, GridPos, HumanLook, MapLayer, MobStat, NetId, ObjectSpec, PickListView, Slot, SpellbookView,
    TilePos,
};

use crate::actions::{Outbox, in_reach};
use crate::combat::VisibleEvents;
use crate::doors::Airlock;
use crate::equipment::Equipment;
use crate::filing::{PendingPicks, PickListOut};
use crate::hands::HeldBy;
use crate::health::Vitals;
use crate::movement::Mob;
use crate::objects::{PlayerControlled, spawn_object};
use crate::rng::GameRng;
use crate::spells;
use crate::wires::PanelViewer;

pub const SPELLBOOK: &str = "spellbook";
pub const TELEPORT_SCROLL: &str = "teleportation-scroll";
pub const MAGIC_STAFF: &str = "magic-staff";
pub const SCRYING_ORB: &str = "scrying-orb";
/// The one-use scrolls and the spell each teaches (Luna `spellbook/oneuse`). Smoke's waits on
/// smoke.
pub const MAGIC_SCROLLS: [(&str, &str); 3] = [
    ("magic-scroll-fireball", "fireball"),
    ("magic-scroll-blink", "blink"),
    ("magic-scroll-forcewall", "forcewall"),
];
/// Spells a book teaches, then how many it starts with (Luna `uses`).
pub const BOOK_USES: u8 = 6;
/// A teleportation scroll's jumps.
pub const SCROLL_USES: u8 = 4;
/// How long the magic staff's wall stands, ds (Luna `lesserforcewall`'s `summon_lifespan`).
pub const STAFF_WALL_DS: f32 = 50.0;

/// What a spellbook offers, in its order. The spells Luna's book lists that aren't built yet
/// (`spells.md`) are left out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Spell(&'static str),
    SummonGuns,
    Armor,
    Scrying,
    Rememorize,
}

pub const BOOK_SPELLS: [&str; 7] = [
    "magic-missile",
    "fireball",
    "disable-tech",
    "forcewall",
    "blink",
    "teleport",
    "knock",
];
const ARTEFACTS: [(&str, Choice); 2] = [("armor", Choice::Armor), ("scrying", Choice::Scrying)];

/// The book's lines: its spells, Summon Guns, the artefacts, then Re-memorize.
pub fn choices() -> Vec<Choice> {
    let mut all: Vec<Choice> = BOOK_SPELLS.iter().map(|s| Choice::Spell(s)).collect();
    all.push(Choice::SummonGuns);
    all.extend(ARTEFACTS.iter().map(|(_, c)| *c));
    all.push(Choice::Rememorize);
    all
}

/// Luna's Summon Guns, from the guns built so far (`spells.md`).
pub const SUMMONED_GUNS: [&str; 8] = [
    "taser",
    "energy-gun",
    "laser-gun",
    "revolver",
    "syndicate-revolver",
    "pistol",
    "double-barrel",
    "combat-shotgun",
];
/// A summoned gun's holder becomes a survivor this often, % (Luna `rightandwrong`).
pub const SURVIVOR_CHANCE: f32 = 25.0;

/// A spellbook's spells left to learn, and at most.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spellbook {
    pub uses: u8,
    pub max: u8,
}

impl Default for Spellbook {
    fn default() -> Self {
        Self {
            uses: BOOK_USES,
            max: BOOK_USES,
        }
    }
}

/// A teleportation scroll's jumps left.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TeleportScroll(pub u8);

/// A one-use scroll that has been read.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollRead;

/// The spellbook window as last worked out, for the server to send.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellbookOut(pub Option<SpellbookView>);

/// On a ghost that stared into a scrying orb: it may go back to its body.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MayReturn;

fn tell(world: &mut World, mob: Entity, msg: LocMsg) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(msg);
    }
}

fn sprite(world: &World, entity: Entity) -> Option<&str> {
    world.get::<Appearance>(entity).map(|a| a.sprite.as_str())
}

/// Whether an item answers being used in the hand here.
pub fn uses_self(sprite: &str) -> bool {
    matches!(sprite, SPELLBOOK | TELEPORT_SCROLL | SCRYING_ORB) || scroll_spell(sprite).is_some()
}

fn scroll_spell(sprite: &str) -> Option<&'static str> {
    MAGIC_SCROLLS
        .iter()
        .find(|(s, _)| *s == sprite)
        .map(|(_, spell)| *spell)
}

/// Held by `mob`, or on the floor within reach (Luna's `loc == H || in_range` on a turf).
fn within(world: &World, mob: Entity, item: Entity) -> bool {
    if world.get::<HeldBy>(item).is_some_and(|h| h.0 == mob) {
        return true;
    }
    match (world.get::<GridPos>(mob), world.get::<GridPos>(item)) {
        (Some(a), Some(b)) => in_reach(a.0, b.0),
        _ => false,
    }
}

/// Luna's `Topic` checks: a person, awake and not cuffed.
fn able(world: &World, mob: Entity) -> bool {
    world.get::<HumanLook>(mob).is_some()
        && world
            .get::<Vitals>(mob)
            .is_none_or(|v| v.stat == MobStat::Conscious)
        && world
            .get::<Equipment>(mob)
            .is_none_or(|e| e.get(Slot::Cuffs).is_none())
}

/// The thing in the hand used on itself (Luna `attack_self`).
pub fn use_self(world: &mut World, mob: Entity, item: Entity) {
    let Some(sprite) = sprite(world, item).map(str::to_owned) else {
        return;
    };
    match sprite.as_str() {
        SPELLBOOK => {
            if world.get::<Spellbook>(item).is_none() {
                world.entity_mut(item).insert(Spellbook::default());
            }
            world.entity_mut(mob).insert(PanelViewer(item));
        }
        TELEPORT_SCROLL => {
            if world.get::<TeleportScroll>(item).is_none() {
                world.entity_mut(item).insert(TeleportScroll(SCROLL_USES));
            }
            world.entity_mut(mob).insert(PanelViewer(item));
        }
        SCRYING_ORB => stare(world, mob),
        other => {
            let Some(spell) = scroll_spell(other) else {
                return;
            };
            if world.get::<ScrollRead>(item).is_some() {
                return;
            }
            world.entity_mut(item).insert(ScrollRead);
            // Luna teaches it again to someone who knows it; here they just know it.
            spells::give(world, mob, spell);
            tell(
                world,
                mob,
                LocMsg::new("scroll-learned").msg("spell", format!("spell-{spell}")),
            );
        }
    }
}

/// Luna `scrying/attack_self`: out of the body as a ghost, free to go back.
fn stare(world: &mut World, mob: Entity) {
    let Some(at) = world.get::<GridPos>(mob).map(|p| p.0) else {
        return;
    };
    let name = world
        .get::<rs13_world::DisplayName>(mob)
        .map(|n| n.0.clone())
        .unwrap_or_default();
    tell(world, mob, LocMsg::new("orb-see-everything"));
    world.resource_mut::<VisibleEvents>().0.push((
        at,
        LocMsg::new("orb-stares").arg("name", name),
        Some(mob),
    ));
    if let Some(ghost) = crate::ghosts::leave_body(world, mob) {
        world.entity_mut(ghost).insert(MayReturn);
    }
}

/// Lines picked in the book's window and the scroll's list (after the filing cabinets have
/// taken theirs).
pub fn run_picks(world: &mut World) {
    let mut q = world.query::<(Entity, &mut PendingPicks)>();
    let picks: Vec<(Entity, Vec<(Entity, usize)>)> = q
        .iter_mut(world)
        .filter(|(_, p)| !p.0.is_empty())
        .map(|(mob, mut p)| (mob, std::mem::take(&mut p.0)))
        .collect();
    for (mob, list) in picks {
        let mut others = Vec::new();
        for (target, line) in list {
            let ours = matches!(sprite(world, target), Some(SPELLBOOK | TELEPORT_SCROLL));
            if !ours {
                // A DNA console's, after.
                others.push((target, line));
                continue;
            }
            if !within(world, mob, target) || !able(world, mob) {
                continue;
            }
            match sprite(world, target) {
                Some(SPELLBOOK) => book_pick(world, mob, target, line),
                Some(TELEPORT_SCROLL) => scroll_pick(world, mob, target, line),
                _ => {}
            }
        }
        if !others.is_empty()
            && let Some(mut pending) = world.get_mut::<PendingPicks>(mob)
        {
            pending.0.extend(others);
        }
    }
}

fn area_of(world: &mut World, pos: TilePos) -> Option<String> {
    let mut layers = world.query::<&MapLayer>();
    layers
        .iter(world)
        .find(|l| l.0.z == pos.z)
        .and_then(|l| l.0.area(pos).map(str::to_owned))
}

/// Luna `spellbook/Topic`.
fn book_pick(world: &mut World, mob: Entity, book: Entity, line: usize) {
    let Some(choice) = choices().get(line).copied() else {
        return;
    };
    let mut state = world.get::<Spellbook>(book).copied().unwrap_or_default();
    let Some(at) = world.get::<GridPos>(mob).map(|p| p.0) else {
        return;
    };
    if choice == Choice::Rememorize {
        if area_of(world, at).is_some_and(|a| a.contains("wizard")) {
            state.uses = state.max;
            spells::forget_all(world, mob);
            tell(world, mob, LocMsg::new("book-rememorized"));
        } else {
            tell(world, mob, LocMsg::new("book-rememorize-elsewhere"));
        }
        world.entity_mut(book).insert(state);
        return;
    }
    if state.uses < 1 || state.max < 1 {
        return;
    }
    state.uses -= 1;
    match choice {
        Choice::Spell(id) => {
            if spells::knows(world, mob, id) {
                state.uses += 1;
                tell(world, mob, LocMsg::new("book-known"));
            } else {
                spells::give(world, mob, id);
                tell(
                    world,
                    mob,
                    LocMsg::new("book-learned").msg("spell", format!("spell-{id}")),
                );
            }
        }
        Choice::SummonGuns => {
            summon_guns(world);
            state.max -= 1;
            tell(world, mob, LocMsg::new("book-summoned-guns"));
        }
        Choice::Armor => {
            // The sandals in case they've lost theirs, and gloves to finish the outfit.
            for id in [
                "sandals",
                "purple-gloves",
                "wizard-hardsuit",
                "wizard-hardsuit-helmet",
            ] {
                let _ = spawn_object(world, &ObjectSpec::new(id), at);
            }
            state.max -= 1;
            tell(world, mob, LocMsg::new("book-bought-armor"));
        }
        Choice::Scrying => {
            let _ = spawn_object(world, &ObjectSpec::new(SCRYING_ORB), at);
            state.max -= 1;
            tell(world, mob, LocMsg::new("book-bought-orb"));
        }
        Choice::Rememorize => {}
    }
    world.entity_mut(book).insert(state);
}

/// Luna `rightandwrong`: a gun at the feet of every living person with a player who isn't
/// already an antagonist; one in four of them becomes a survivor.
pub fn summon_guns(world: &mut World) {
    let mut people = world.query_filtered::<(Entity, &GridPos, Option<&Vitals>), (
        With<HumanLook>,
        With<Mob>,
        With<PlayerControlled>,
        Without<crate::ghosts::LeftBody>,
        Without<crate::traitor::Traitor>,
        Without<crate::changeling::Changeling>,
        Without<crate::revolution::HeadRev>,
        Without<crate::revolution::Revolutionary>,
        Without<crate::nuke::NukeAgent>,
    )>();
    let mut armed: Vec<(Entity, TilePos)> = people
        .iter(world)
        .filter(|(_, _, v)| v.is_none_or(|v| v.stat != MobStat::Dead))
        .map(|(e, p, _)| (e, p.0))
        .collect();
    armed.sort_by_key(|(e, _)| *e);
    for (person, at) in armed {
        let mut rng = world.resource_mut::<GameRng>();
        let survivor = rng.prob(SURVIVOR_CHANCE);
        let gun = SUMMONED_GUNS[rng.range(0, SUMMONED_GUNS.len() as i32 - 1) as usize];
        if survivor {
            crate::traitor::make_survivor(world, person);
        }
        let _ = spawn_object(world, &ObjectSpec::new(gun), at);
    }
}

/// Luna `teleportation_scroll/Topic`: a jump to a clear floor of the place picked.
fn scroll_pick(world: &mut World, mob: Entity, scroll: Entity, line: usize) {
    let left = world.get::<TeleportScroll>(scroll).map_or(SCROLL_USES, |s| s.0);
    if left < 1 {
        return;
    }
    let Some(area) = spells::teleport_areas(world).get(line).cloned() else {
        return;
    };
    world.entity_mut(scroll).insert(TeleportScroll(left - 1));
    if let Some(to) = spells::clear_tile_in(world, &area) {
        world.entity_mut(mob).insert(GridPos(to));
    }
}

/// Keep the book's window and the scroll's list in step; close them once out of reach.
pub fn update_views(world: &mut World) {
    let mut viewers = world.query::<(Entity, Option<&PanelViewer>, Option<&SpellbookOut>)>();
    let all: Vec<(Entity, Option<Entity>, bool)> = viewers
        .iter(world)
        .filter(|(_, v, out)| v.is_some() || out.is_some_and(|o| o.0.is_some()))
        .map(|(mob, v, out)| (mob, v.map(|v| v.0), out.is_some_and(|o| o.0.is_some())))
        .collect();
    for (mob, target, had_book) in all {
        let looked = target.and_then(|t| sprite(world, t).map(|s| (t, s.to_owned())));
        let mut book_view = None;
        match looked {
            Some((book, ref s)) if s == SPELLBOOK => {
                if within(world, mob, book) {
                    let state = world.get::<Spellbook>(book).copied().unwrap_or_default();
                    book_view = world.get::<NetId>(book).map(|id| SpellbookView {
                        target: *id,
                        uses: state.uses,
                        spells: BOOK_SPELLS
                            .iter()
                            .map(|s| {
                                let ds = spells::spell(s).map_or(0.0, |d| d.cooldown);
                                ((*s).to_owned(), (ds / 10.0).round() as u16)
                            })
                            .chain(std::iter::once(("summon-guns".to_owned(), 0)))
                            .collect(),
                        artefacts: ARTEFACTS.iter().map(|(id, _)| (*id).to_owned()).collect(),
                    });
                } else {
                    world.entity_mut(mob).remove::<PanelViewer>();
                }
            }
            Some((scroll, ref s)) if s == TELEPORT_SCROLL => {
                let shown = if within(world, mob, scroll) {
                    let areas = spells::teleport_areas(world);
                    world.get::<NetId>(scroll).map(|id| PickListView {
                        target: *id,
                        title: TELEPORT_SCROLL.to_owned(),
                        rows: areas,
                    })
                } else {
                    world.entity_mut(mob).remove::<PanelViewer>();
                    None
                };
                if let Some(mut out) = world.get_mut::<PickListOut>(mob)
                    && out.0 != shown
                {
                    out.0 = shown;
                }
            }
            _ => {}
        }
        if book_view.is_some() || had_book {
            let mut entity = world.entity_mut(mob);
            match entity.get_mut::<SpellbookOut>() {
                Some(mut out) => {
                    if out.0 != book_view {
                        out.0 = book_view;
                    }
                }
                None => {
                    entity.insert(SpellbookOut(book_view));
                }
            }
        }
    }
}

/// Whether the filing cabinets should leave this viewer's list alone.
pub fn keeps_own_list(sprite: &str) -> bool {
    sprite == TELEPORT_SCROLL
}

/// Luna `staff/magical/afterattack`, for the test staff: a click further than reach. A door
/// is knocked open; open floor gets a short-lived wall. Its fireball is handed the person,
/// not their floor, so in Luna nothing flies, and nothing does here either.
pub fn staff_at_thing(world: &mut World, target: Entity) {
    let Some(pos) = world.get::<GridPos>(target).map(|p| p.0) else {
        return;
    };
    if world.get::<Airlock>(target).is_some() {
        spells::knock(world, pos, 0);
    }
}

pub fn staff_at_tile(world: &mut World, to: TilePos) {
    let mut layers = world.query::<&MapLayer>();
    let solid = layers
        .iter(world)
        .find(|l| l.0.z == to.z)
        .and_then(|l| l.0.tile(to))
        .is_none_or(|t| t.solid);
    if !solid {
        spells::wall(world, to, true, STAFF_WALL_DS);
    }
}
