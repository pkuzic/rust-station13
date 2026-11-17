//! Rubber stamps, folders and the photocopier (Luna's `paperwork/stamps.dm`, `folders.dm`,
//! `photocopier.dm` and the old paper's `attackby`). Spec: `docs/reference/paperwork.md`.
//!
//! - **Stamps** mark a sheet (or a clipboard's top sheet): a line saying which stamp goes on the
//!   end of its text, its sprite shows the stamp, and no pen can change it after. A stamped
//!   supply manifest sent back in its crate earns cargo 4 points.
//! - **Folders** hold sheets; used in the hand, their window takes one out or reads it.
//! - **Noticeboards** hold up to five sheets, to read, write on or take down from their window.
//! - **The photocopier** takes one sheet and prints up to ten copies of its name and text, one a
//!   second and a half, a unit of toner each; an empty one takes a new toner cartridge.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, Dense, GridPos, NetId, ObjectSpec, PaperMode, PickListView, TilePos};

use crate::actions::{Outbox, in_reach};
use crate::filing::{PendingPicks, PickListOut};
use crate::hands::{Hands, HeldBy};
use crate::paperwork::{CLIPBOARD, Clipboard, Filed, PAPER, Paper, PaperOut, PaperWindow, Stamped};
use crate::power::{Channel, PowerUser};
use crate::rng::GameRng;
use crate::sound::SoundEvents;
use crate::time::WorldTime;
use crate::wires::PanelViewer;

pub const PHOTOCOPIER: &str = "photocopier";
pub const NOTICEBOARD: &str = "noticeboard";
/// Most sheets on a noticeboard (Luna `notices < 5`).
pub const MAX_NOTICES: usize = 5;
pub const TONER: &str = "toner";
/// Toner in a full photocopier, one per copy (Luna `toner`).
pub const TONER_FULL: u8 = 30;
/// Most copies printed at once (Luna `maxcopies`).
pub const MAX_COPIES: u8 = 10;
/// Between one copy and the next, ds (Luna `sleep(15)`).
pub const COPY_DS: f32 = 15.0;

pub fn is_stamp(sprite: &str) -> bool {
    sprite == "stamp" || sprite.starts_with("stamp-")
}

pub fn is_folder(sprite: &str) -> bool {
    sprite.starts_with("folder-")
}

/// A folder: the sheets in it, in the order they went in.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct Folder(pub Vec<Entity>);

/// A noticeboard: the sheets pinned to it, in the order they went up.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct Noticeboard(pub Vec<Entity>);

/// A photocopier: the sheet in it, how many copies to print, toner left, whether it is
/// wrenched down, and the copies still to come out.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Photocopier {
    pub copy: Option<Entity>,
    pub copies: u8,
    pub toner: u8,
    pub anchored: bool,
    pub printing: u8,
    pub next_ds: f32,
}

impl Default for Photocopier {
    fn default() -> Self {
        Self {
            copy: None,
            copies: 1,
            toner: TONER_FULL,
            anchored: true,
            printing: 0,
            next_ds: 0.0,
        }
    }
}

/// Give a new item its parts: a folder holds sheets.
pub fn equip_item(world: &mut World, item: Entity, proto: &str) {
    if is_folder(proto) {
        world.entity_mut(item).insert(Folder::default());
    }
    if proto == NOTICEBOARD {
        world.entity_mut(item).insert(Noticeboard::default());
    }
}

pub fn spawn_photocopier(world: &mut World, pos: TilePos, id: NetId) -> Entity {
    world
        .spawn((
            GridPos(pos),
            id,
            Dense,
            Appearance::new(PHOTOCOPIER, ""),
            Photocopier::default(),
            PowerUser::new(Channel::Equipment, 30.0),
        ))
        .id()
}

fn tell(world: &mut World, mob: Entity, msg: LocMsg) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(msg);
    }
}

fn sprite(world: &World, e: Entity) -> Option<String> {
    world.get::<Appearance>(e).map(|a| a.sprite.clone())
}

fn at(world: &World, e: Entity) -> Option<TilePos> {
    world.get::<GridPos>(e).map(|p| p.0)
}

/// Whether a click with `tool` (the held item's sprite) on `target` is ours: a stamp on a sheet or
/// clipboard, a sheet into a folder, anything on a photocopier, a sheet or an empty hand on a
/// noticeboard.
pub fn takes(target: &str, tool: Option<&str>) -> bool {
    target == PHOTOCOPIER
        || (target == NOTICEBOARD && matches!(tool, None | Some(PAPER)))
        || (tool.is_some_and(is_stamp) && (target == PAPER || target == CLIPBOARD))
        || (tool == Some(PAPER) && is_folder(target))
}

/// Luna `paper/attackby` with a stamp, `clipboard/attackby`, `folder/attackby`,
/// `photocopier/attackby` and `attack_hand`. The caller checks reach.
pub fn use_on(world: &mut World, mob: Entity, held: Option<Entity>, target: Entity) {
    let tool = held.and_then(|h| sprite(world, h));
    let Some(look) = sprite(world, target) else {
        return;
    };
    if look == PHOTOCOPIER {
        use_on_copier(world, mob, held, tool.as_deref(), target);
        return;
    }
    if world.get::<Noticeboard>(target).is_some() {
        pin(world, mob, held, target);
        return;
    }
    let Some(held) = held else { return };
    if tool.as_deref().is_some_and(is_stamp) {
        // On a clipboard, the stamp goes on its top sheet.
        let sheet = match world.get::<Clipboard>(target) {
            Some(board) => board.papers.last().copied(),
            None => Some(target),
        };
        if let Some(sheet) = sheet {
            stamp(world, mob, held, sheet);
        }
        return;
    }
    if world.get::<Folder>(target).is_some() && world.get::<Paper>(held).is_some() {
        drop_from_hand(world, mob, held);
        world.entity_mut(held).insert(Filed(target));
        if let Some(mut folder) = world.get_mut::<Folder>(target) {
            folder.0.push(held);
        }
        tell(
            world,
            mob,
            LocMsg::new("folder-put").arg("item", PAPER).arg("folder", look),
        );
    }
}

/// Out of the mob's hand, onto nothing yet.
fn drop_from_hand(world: &mut World, mob: Entity, item: Entity) {
    if let Some(mut hands) = world.get_mut::<Hands>(mob) {
        for hand in [hands.active, hands.active.other()] {
            if hands.get(hand) == Some(item) {
                *hands.slot_mut(hand) = None;
            }
        }
    }
    world.entity_mut(item).remove::<(HeldBy, GridPos)>();
}

/// Into a free hand, or at the mob's feet (Luna `put_in_hands`).
fn give(world: &mut World, mob: Entity, item: Entity) {
    world.entity_mut(item).remove::<Filed>();
    if !crate::traitor::put_in_hand(world, mob, item)
        && let Some(pos) = at(world, mob)
    {
        world.entity_mut(item).insert(GridPos(pos));
    }
}

/// A stamp on a sheet (Luna `paper/attackby`): a line naming the stamp goes on the end, shown in
/// the reader's language.
fn stamp(world: &mut World, mob: Entity, stamp: Entity, sheet: Entity) {
    let Some(stamp_id) = sprite(world, stamp) else {
        return;
    };
    let Some(mut paper) = world.get_mut::<Paper>(sheet) else {
        return;
    };
    paper
        .text
        .push_str(&format!("\n[i][msg:paper-stamped-with] [noun:{stamp_id}].[/i]\n"));
    world.entity_mut(sheet).insert(Stamped);
    if let Some(mut look) = world.get_mut::<Appearance>(sheet) {
        look.state = "stamped".to_owned();
    }
    tell(world, mob, LocMsg::new("paper-stamp"));
}

/// Luna `noticeboard/attackby` and `attack_hand`: a sheet is pinned up, while there's room; an
/// empty hand opens its window.
fn pin(world: &mut World, mob: Entity, held: Option<Entity>, board: Entity) {
    let Some(sheet) = held.filter(|h| world.get::<Paper>(*h).is_some()) else {
        world.entity_mut(mob).insert(PanelViewer(board));
        return;
    };
    if world
        .get::<Noticeboard>(board)
        .is_some_and(|b| b.0.len() >= MAX_NOTICES)
    {
        tell(world, mob, LocMsg::new("notice-too-many"));
        return;
    }
    drop_from_hand(world, mob, sheet);
    world.entity_mut(sheet).insert(Filed(board));
    if let Some(mut b) = world.get_mut::<Noticeboard>(board) {
        b.0.push(sheet);
    }
    tell(world, mob, LocMsg::new("notice-pinned"));
}

fn use_on_copier(world: &mut World, mob: Entity, held: Option<Entity>, tool: Option<&str>, copier: Entity) {
    let Some(state) = world.get::<Photocopier>(copier).cloned() else {
        return;
    };
    match (held, tool) {
        (Some(sheet), Some(PAPER)) if world.get::<Paper>(sheet).is_some() => {
            if state.copy.is_some() {
                tell(world, mob, LocMsg::new("copier-full"));
                return;
            }
            drop_from_hand(world, mob, sheet);
            world.entity_mut(sheet).insert(Filed(copier));
            if let Some(mut c) = world.get_mut::<Photocopier>(copier) {
                c.copy = Some(sheet);
            }
            tell(world, mob, LocMsg::new("copier-inserted").arg("item", PAPER));
        }
        (Some(cartridge), Some(TONER)) => {
            if state.toner > 0 {
                tell(world, mob, LocMsg::new("copier-toner-not-empty"));
                return;
            }
            drop_from_hand(world, mob, cartridge);
            world.entity_mut(cartridge).despawn();
            if let Some(mut c) = world.get_mut::<Photocopier>(copier) {
                c.toner = TONER_FULL;
            }
            tell(world, mob, LocMsg::new("copier-inserted").arg("item", TONER));
        }
        (Some(_), Some("wrench")) => {
            if let Some(pos) = at(world, copier)
                && let Some(mut sounds) = world.get_resource_mut::<SoundEvents>()
            {
                sounds.tool_used(Some("wrench"), pos);
            }
            let anchored = !state.anchored;
            if let Some(mut c) = world.get_mut::<Photocopier>(copier) {
                c.anchored = anchored;
            }
            let id = if anchored {
                "copier-wrenched"
            } else {
                "copier-unwrenched"
            };
            tell(world, mob, LocMsg::new(id));
        }
        (None, _) => {
            world.entity_mut(mob).insert(PanelViewer(copier));
        }
        _ => {}
    }
}

/// An explosion reaching a photocopier (Luna `photocopier/ex_act`): whether it is destroyed. If
/// not, it may lose its toner.
pub fn blasted(world: &mut World, copier: Entity, severity: u8) -> bool {
    let gone = match severity {
        1 => true,
        2 => world.resource_mut::<GameRng>().prob(50.0),
        _ => false,
    };
    if gone {
        if let Some(sheet) = world.get::<Photocopier>(copier).and_then(|c| c.copy)
            && world.get_entity(sheet).is_ok()
        {
            world.entity_mut(sheet).despawn();
        }
        return true;
    }
    if (severity == 2 || world.resource_mut::<GameRng>().prob(50.0))
        && let Some(mut c) = world.get_mut::<Photocopier>(copier)
    {
        c.toner = 0;
    }
    false
}

// --- Windows ---

#[derive(Debug, Clone, Copy, PartialEq)]
enum Line {
    Info,
    Take(usize),
    Read(usize),
    NoticeWrite(usize),
    NoticeRemove(usize),
    Remove,
    Copy,
    Less,
    More,
}

fn lines(world: &World, target: Entity) -> Vec<(String, Line)> {
    let mut out = Vec::new();
    if let Some(folder) = world.get::<Folder>(target) {
        for (i, sheet) in folder.0.iter().enumerate() {
            let name = world
                .get::<Paper>(*sheet)
                .map(|p| p.name.clone())
                .unwrap_or_default();
            out.push((format!("msg:folder-remove:{name}"), Line::Take(i)));
            out.push((format!("msg:folder-read:{name}"), Line::Read(i)));
        }
        return out;
    }
    if let Some(board) = world.get::<Noticeboard>(target) {
        for (i, sheet) in board.0.iter().enumerate() {
            let name = world
                .get::<Paper>(*sheet)
                .map(|p| p.name.clone())
                .unwrap_or_default();
            out.push((format!("msg:notice-read:{name}"), Line::Read(i)));
            out.push(("msg:notice-write".to_owned(), Line::NoticeWrite(i)));
            out.push(("msg:notice-remove".to_owned(), Line::NoticeRemove(i)));
        }
        return out;
    }
    let Some(copier) = world.get::<Photocopier>(target) else {
        return out;
    };
    if copier.copy.is_some() {
        out.push(("msg:copier-remove".to_owned(), Line::Remove));
        if copier.toner > 0 {
            out.push(("msg:copier-copy".to_owned(), Line::Copy));
            out.push((format!("msg:copier-printing:{}", copier.copies), Line::Info));
            out.push(("msg:copier-less".to_owned(), Line::Less));
            out.push(("msg:copier-more".to_owned(), Line::More));
        }
    } else if copier.toner > 0 {
        out.push(("msg:copier-insert".to_owned(), Line::Info));
    }
    out.push((format!("msg:copier-toner:{}", copier.toner), Line::Info));
    if copier.toner == 0 {
        out.push(("msg:copier-no-toner".to_owned(), Line::Info));
    }
    out
}

fn is_ours(world: &World, target: Entity) -> bool {
    world.get::<Folder>(target).is_some()
        || world.get::<Photocopier>(target).is_some()
        || world.get::<Noticeboard>(target).is_some()
}

/// Whether the filing cabinets should leave this viewer's list alone.
pub fn keeps_own_list(sprite: &str) -> bool {
    sprite == PHOTOCOPIER || sprite == NOTICEBOARD || is_folder(sprite)
}

/// The sheets a window lists: a folder's or a noticeboard's.
fn sheet_at(world: &World, target: Entity, i: usize) -> Option<Entity> {
    world
        .get::<Folder>(target)
        .map(|f| &f.0)
        .or_else(|| world.get::<Noticeboard>(target).map(|b| &b.0))
        .and_then(|sheets| sheets.get(i).copied())
}

/// A folder only while carried (Luna `usr.contents.Find(src)`); a photocopier or noticeboard
/// within a tile.
fn can_work(world: &World, mob: Entity, target: Entity) -> bool {
    if world.get::<Folder>(target).is_some() {
        return world.get::<HeldBy>(target).is_some_and(|h| h.0 == mob);
    }
    at(world, mob)
        .zip(at(world, target))
        .is_some_and(|(a, b)| in_reach(a, b))
}

/// The awake mob (Luna's `usr.stat || usr.restrained()`).
fn awake(world: &World, mob: Entity) -> bool {
    world
        .get::<crate::health::Vitals>(mob)
        .is_none_or(|v| v.stat == rs13_world::MobStat::Conscious)
}

/// A folder used in the hand opens its window (Luna `folder/attack_self`).
pub fn open_folder(world: &mut World, mob: Entity, folder: Entity) {
    if world.get::<Folder>(folder).is_some() {
        world.entity_mut(mob).insert(PanelViewer(folder));
    }
}

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
            if !is_ours(world, target) {
                others.push((target, line));
                continue;
            }
            if !can_work(world, mob, target) || !awake(world, mob) {
                continue;
            }
            if let Some(chosen) = lines(world, target).get(line).map(|(_, l)| *l) {
                do_line(world, mob, target, chosen);
            }
        }
        if !others.is_empty()
            && let Some(mut pending) = world.get_mut::<PendingPicks>(mob)
        {
            pending.0.extend(others);
        }
    }
}

fn do_line(world: &mut World, mob: Entity, target: Entity, line: Line) {
    match line {
        Line::Info => {}
        Line::Take(i) => {
            let sheet = world
                .get_mut::<Folder>(target)
                .and_then(|mut f| (i < f.0.len()).then(|| f.0.remove(i)));
            if let Some(sheet) = sheet {
                give(world, mob, sheet);
            }
        }
        Line::NoticeWrite(_) | Line::NoticeRemove(_)
            if world.get::<crate::security::Cuffed>(mob).is_some() =>
        {
            // Luna's `usr.restrained()`.
            let id = if matches!(line, Line::NoticeWrite(_)) {
                "notice-cuffed-write"
            } else {
                "notice-cuffed-remove"
            };
            tell(world, mob, LocMsg::new(id));
        }
        Line::NoticeWrite(i) => {
            let Some(sheet) = sheet_at(world, target, i) else {
                return;
            };
            let pen = world
                .get::<Hands>(mob)
                .and_then(Hands::active_item)
                .and_then(|p| sprite(world, p))
                .is_some_and(|s| crate::paperwork::is_pen(&s));
            if !pen {
                tell(world, mob, LocMsg::new("notice-no-pen"));
                return;
            }
            if world.get::<Stamped>(sheet).is_some() {
                tell(world, mob, LocMsg::new("paper-stamped-no-edit"));
                return;
            }
            let Some(paper) = world.get::<Paper>(sheet).cloned() else {
                return;
            };
            if let Some(mut out) = world.get_mut::<PaperOut>(mob) {
                out.0.push(PaperWindow {
                    sheet,
                    title: paper.name,
                    text: paper.text,
                    mode: PaperMode::Write,
                });
            }
        }
        Line::NoticeRemove(i) => {
            // Down onto the floor under the board.
            let sheet = world
                .get_mut::<Noticeboard>(target)
                .and_then(|mut b| (i < b.0.len()).then(|| b.0.remove(i)));
            if let (Some(sheet), Some(pos)) = (sheet, at(world, target)) {
                world.entity_mut(sheet).remove::<Filed>().insert(GridPos(pos));
            }
        }
        Line::Read(i) => {
            let Some(sheet) = sheet_at(world, target, i) else {
                return;
            };
            let Some(paper) = world.get::<Paper>(sheet).cloned() else {
                return;
            };
            if let Some(mut out) = world.get_mut::<PaperOut>(mob) {
                out.0.push(PaperWindow {
                    sheet,
                    title: paper.name,
                    text: paper.text,
                    mode: PaperMode::Read,
                });
            }
        }
        Line::Remove => {
            let sheet = world.get_mut::<Photocopier>(target).and_then(|mut c| {
                c.printing = 0;
                c.copy.take()
            });
            if let Some(sheet) = sheet {
                give(world, mob, sheet);
                tell(world, mob, LocMsg::new("copier-took").arg("item", PAPER));
            }
        }
        Line::Copy => {
            let now = world.get_resource::<WorldTime>().map_or(0.0, |t| t.deciseconds());
            if let Some(mut c) = world.get_mut::<Photocopier>(target)
                && c.copy.is_some()
                && c.printing == 0
            {
                c.printing = c.copies;
                c.next_ds = now;
            }
        }
        Line::Less => {
            if let Some(mut c) = world.get_mut::<Photocopier>(target) {
                c.copies = c.copies.saturating_sub(1).max(1);
            }
        }
        Line::More => {
            if let Some(mut c) = world.get_mut::<Photocopier>(target) {
                c.copies = (c.copies + 1).min(MAX_COPIES);
            }
        }
    }
}

/// Copies come out one at a time onto the photocopier's tile, a unit of toner each: the sheet's
/// name and text, but not its stamp (Luna `photocopier/Topic`, `copy`).
pub fn run_copiers(world: &mut World) {
    let now = world.get_resource::<WorldTime>().map_or(0.0, |t| t.deciseconds());
    let mut q = world.query::<(Entity, &Photocopier, &GridPos)>();
    let due: Vec<(Entity, TilePos)> = q
        .iter(world)
        .filter(|(_, c, _)| c.printing > 0 && now >= c.next_ds)
        .map(|(e, _, p)| (e, p.0))
        .collect();
    for (copier, pos) in due {
        let Some(state) = world.get::<Photocopier>(copier).cloned() else {
            continue;
        };
        let original = state.copy.and_then(|s| world.get::<Paper>(s).cloned());
        let (Some(original), true) = (original, state.toner > 0) else {
            if let Some(mut c) = world.get_mut::<Photocopier>(copier) {
                c.printing = 0;
            }
            continue;
        };
        if let Ok(sheet) = crate::objects::spawn_object(world, &ObjectSpec::new(PAPER), pos)
            && let Some(mut paper) = world.get_mut::<Paper>(sheet)
        {
            *paper = original;
        }
        if let Some(mut c) = world.get_mut::<Photocopier>(copier) {
            c.toner -= 1;
            c.printing -= 1;
            c.next_ds = now + COPY_DS;
        }
    }
}

pub fn update_views(world: &mut World) {
    let mut viewers = world.query::<(Entity, &PanelViewer)>();
    let all: Vec<(Entity, Entity)> = viewers.iter(world).map(|(m, v)| (m, v.0)).collect();
    for (mob, target) in all {
        if !is_ours(world, target) {
            continue;
        }
        let title = sprite(world, target).unwrap_or_default();
        let view = match (can_work(world, mob, target), world.get::<NetId>(target).copied()) {
            (true, Some(id)) => Some(PickListView {
                target: id,
                title,
                rows: lines(world, target).into_iter().map(|(r, _)| r).collect(),
            }),
            _ => {
                world.entity_mut(mob).remove::<PanelViewer>();
                None
            }
        };
        if let Some(mut out) = world.get_mut::<PickListOut>(mob)
            && out.0 != view
        {
            out.0 = view;
        }
    }
}

/// A folder shows a sheet sticking out while it holds any (Luna `folder/update_icon`); a
/// noticeboard shows how many sheets are up (`nboard0N`).
pub fn update_looks(
    mut folders: Query<(&Folder, &mut Appearance), (Changed<Folder>, Without<Noticeboard>)>,
    mut boards: Query<(&Noticeboard, &mut Appearance), Changed<Noticeboard>>,
) {
    for (folder, mut look) in &mut folders {
        let state = if folder.0.is_empty() { "" } else { "+folder_paper" };
        if look.state != state {
            look.state = state.to_owned();
        }
    }
    for (board, mut look) in &mut boards {
        let state = match board.0.len() {
            0 => String::new(),
            n => format!("nboard0{n}"),
        };
        if look.state != state {
            look.state = state;
        }
    }
}
