//! Paper bins, paper, pens and clipboards: write on paper with a pen, read it, label it,
//! keep sheets on a clipboard. Spec: `docs/reference/paperwork.md`.

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, ClipboardView, GridPos, MobStat, NetId, ObjectSpec, PaperMode, TilePos};

use crate::actions::{ClickOutcome, in_reach};
use crate::hands::{Hands, HeldBy};
use crate::health::Vitals;
use crate::wires::PanelViewer;

pub const PAPER_BIN: &str = "paper-bin";
pub const PAPER: &str = "paper";
pub const CLIPBOARD: &str = "clipboard";
/// Sheets in a new paper bin (Luna `amount`).
pub const BIN_SHEETS: u16 = 30;
/// Longest text on a sheet, characters (Luna `MAX_PAPER_MESSAGE_LEN` 3 072, less one).
pub const MAX_TEXT: usize = 3071;
/// Longest label, characters (Luna `copytext(n, 1, 32)`).
pub const MAX_LABEL: usize = 31;

pub fn is_pen(sprite: &str) -> bool {
    matches!(sprite, "pen" | "pen-blue" | "pen-red")
}

/// A paper bin: sheets left, and the ones put back (handed out first).
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct PaperBin {
    pub amount: u16,
    pub returned: Vec<Entity>,
}

/// A sheet of paper: its name (Luna `name`: "Paper", or "paper- 'label'"), and what is
/// written on it (Luna `info`).
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct Paper {
    pub name: String,
    pub text: String,
}

/// A clipboard: its sheets (the last on top), and a pen in its slot.
#[derive(Component, Debug, Clone, PartialEq, Eq, Default)]
pub struct Clipboard {
    pub papers: Vec<Entity>,
    pub pen: Option<Entity>,
}

/// On a sheet in a paper bin, or a sheet or pen on a clipboard (it is off the map).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Filed(pub Entity);

/// A paper window to show: read, write or label a sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaperWindow {
    pub sheet: Entity,
    pub title: String,
    pub text: String,
    pub mode: PaperMode,
}

/// Paper windows for the server to send to the player.
#[derive(Component, Debug, Default)]
pub struct PaperOut(pub Vec<PaperWindow>);

/// What the mob's open clipboard shows, for the server to send when it changes.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct ClipboardOut(pub Option<ClipboardView>);

/// Paper and clipboard requests waiting for the next tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaperOp {
    /// The whole new text of a sheet (Luna's text box edits it all).
    Write(Entity, String),
    Label(Entity, String),
    /// A clipboard's pen into the empty hand, or the pen in the hand into it.
    Pen(Entity),
    /// On a clipboard: write on, take out, read or move to the top its `n`th sheet, top
    /// first.
    WriteSheet(Entity, usize),
    Remove(Entity, usize),
    Read(Entity, usize),
    Top(Entity, usize),
}

#[derive(Component, Debug, Default)]
pub struct PendingPaperOps(pub Vec<PaperOp>);

/// Spawn a bin, or give a new sheet its paper (the map may name it and write on it).
pub fn spawn_paperwork(entity: &mut EntityWorldMut, spec: &ObjectSpec) {
    match spec.id.as_str() {
        PAPER_BIN => {
            entity.insert(PaperBin {
                amount: BIN_SHEETS,
                returned: Vec::new(),
            });
        }
        PAPER => {
            let var = |name: &str| {
                spec.vars
                    .get(name)
                    .map(|v| v.trim().to_owned())
                    .filter(|v| v != "null")
            };
            entity.insert(Paper {
                name: var("name").unwrap_or_else(|| "Paper".to_owned()),
                text: var("info").unwrap_or_default().replace("\\n", "\n"),
            });
        }
        CLIPBOARD => {
            entity.insert(Clipboard::default());
        }
        _ => {}
    }
}

/// Everything clicks with paperwork need.
#[derive(SystemParam)]
pub struct PaperWork<'w, 's> {
    bins: Query<'w, 's, &'static mut PaperBin>,
    papers: Query<'w, 's, &'static Paper>,
    clipboards: Query<'w, 's, &'static mut Clipboard>,
    looks: Query<'w, 's, &'static Appearance>,
    out: Query<'w, 's, &'static mut PaperOut>,
}

impl PaperWork<'_, '_> {
    fn is_pen(&self, item: Option<Entity>) -> bool {
        item.and_then(|i| self.looks.get(i).ok())
            .is_some_and(|a| is_pen(&a.sprite))
    }

    /// Whether this click is ours: on a bin, a sheet with a pen, a clipboard with a sheet
    /// or with anything while it has a sheet on top.
    pub fn takes(&self, hands: &Hands, target: Entity) -> bool {
        let active = hands.active_item();
        let held = active.is_some_and(|a| a != target);
        self.bins.contains(target)
            || (self.papers.contains(target) && self.is_pen(active))
            || (self
                .clipboards
                .get(target)
                .is_ok_and(|c| !c.papers.is_empty() || held)
                && held
                && active.is_some_and(|a| self.papers.contains(a) || self.is_pen(active)))
    }

    /// Luna `paper_bin/attack_hand` and `attackby`, `paper/attackby` (a pen) and
    /// `clipboard/attackby`. The caller checks reach.
    pub fn use_on(
        &mut self,
        mob: Entity,
        hands: &mut Hands,
        target: Entity,
        commands: &mut Commands,
    ) -> Option<ClickOutcome> {
        let active = hands.active_item();
        if let Ok(mut bin) = self.bins.get_mut(target) {
            if let Some(sheet) = active.filter(|a| self.papers.contains(*a)) {
                // A sheet goes back in.
                *hands.slot_mut(hands.active) = None;
                commands.entity(sheet).remove::<HeldBy>().insert(Filed(target));
                bin.returned.push(sheet);
                bin.amount += 1;
                return Some(told("paper-bin-put", PAPER));
            }
            if active.is_some() {
                return None;
            }
            if bin.amount == 0 {
                return Some(ClickOutcome::told("paper-bin-empty"));
            }
            bin.amount -= 1;
            let hand = hands.active;
            match bin.returned.pop() {
                Some(sheet) => {
                    commands.entity(sheet).remove::<Filed>().insert(HeldBy(mob));
                    *hands.slot_mut(hand) = Some(sheet);
                }
                None => {
                    commands.queue(move |world: &mut World| {
                        let at = world.get::<GridPos>(mob).map_or(TilePos::new(0, 0, 0), |p| p.0);
                        let Ok(sheet) = crate::objects::spawn_object(world, &ObjectSpec::new(PAPER), at)
                        else {
                            return;
                        };
                        world.entity_mut(sheet).remove::<GridPos>().insert(HeldBy(mob));
                        if let Some(mut hands) = world.get_mut::<Hands>(mob) {
                            *hands.slot_mut(hand) = Some(sheet);
                        }
                    });
                }
            }
            return Some(told("paper-bin-take", PAPER));
        }
        if let Ok(mut board) = self.clipboards.get_mut(target) {
            let item = active?;
            if self.papers.contains(item) {
                *hands.slot_mut(hands.active) = None;
                commands.entity(item).remove::<HeldBy>().insert(Filed(target));
                board.papers.push(item);
                return Some(told("clipboard-clipped", CLIPBOARD));
            }
            // Anything else goes to the top sheet: a pen writes on it.
            let top = *board.papers.last()?;
            self.open(mob, top, PaperMode::Write);
            return Some(ClickOutcome::default());
        }
        // A pen on a sheet: its text box.
        self.open(mob, target, PaperMode::Write);
        Some(ClickOutcome::default())
    }

    /// Queue a paper window for the mob to see.
    fn open(&mut self, mob: Entity, paper: Entity, mode: PaperMode) {
        let Ok(sheet) = self.papers.get(paper) else { return };
        let view = PaperWindow {
            sheet: paper,
            title: sheet.name.clone(),
            text: sheet.text.clone(),
            mode,
        };
        if let Ok(mut out) = self.out.get_mut(mob) {
            out.0.push(view);
        }
    }

    /// Examining a sheet opens it to read (Luna `paper/examine`).
    pub fn examine(&mut self, mob: Entity, target: Entity) -> bool {
        let reads = self.papers.contains(target);
        if reads {
            self.open(mob, target, PaperMode::Read);
        }
        reads
    }

    /// Examining a bin counts its sheets (Luna `paper_bin/examine`).
    pub fn examine_bin(&self, target: Entity) -> Option<LocMsg> {
        let bin = self.bins.get(target).ok()?;
        Some(LocMsg::new("examine-paper-bin").num("count", i64::from(bin.amount)))
    }

    /// Whether `item` does something used in the hand: a sheet is labelled, a clipboard
    /// opens.
    pub fn uses_self(&self, item: Entity) -> bool {
        self.papers.contains(item) || self.clipboards.contains(item)
    }

    /// Luna `paper/attack_self` (its label box) and `clipboard/attack_self` (its window).
    pub fn use_self(&mut self, mob: Entity, item: Entity, commands: &mut Commands) -> Option<ClickOutcome> {
        if self.papers.contains(item) {
            self.open(mob, item, PaperMode::Label);
            return Some(ClickOutcome::default());
        }
        self.clipboards.get(item).ok()?;
        commands.entity(mob).insert(PanelViewer(item));
        Some(ClickOutcome::default())
    }
}

fn told(id: &str, item: &str) -> ClickOutcome {
    ClickOutcome {
        to_mob: Some(LocMsg::new(id).arg("item", item.to_owned())),
        seen: None,
    }
}

/// Where a sheet is, for whether the mob can still reach it: in a hand, worn, filed on a
/// clipboard they carry, or on the floor within a tile.
fn reachable(
    mob: Entity,
    mob_pos: TilePos,
    thing: Entity,
    places: &Query<(Option<&GridPos>, Option<&HeldBy>, Option<&Filed>)>,
) -> bool {
    match places.get(thing) {
        Ok((Some(p), ..)) => in_reach(mob_pos, p.0),
        Ok((_, Some(h), _)) => h.0 == mob,
        Ok((_, _, Some(f))) => reachable(mob, mob_pos, f.0, places),
        _ => false,
    }
}

/// Paper and clipboard requests (Luna `paper/attackby` after its text box, `attack_self`
/// after its label box, `clipboard/Topic`). Awake, and the sheet or clipboard still at hand.
#[allow(clippy::type_complexity)]
pub fn process_paper_ops(
    mut mobs: Query<(
        Entity,
        &GridPos,
        &mut PendingPaperOps,
        &mut Hands,
        &mut PaperOut,
        &mut crate::actions::Outbox,
        Option<&Vitals>,
    )>,
    mut papers: Query<&mut Paper>,
    mut clipboards: Query<&mut Clipboard>,
    looks: Query<&Appearance>,
    places: Query<(Option<&GridPos>, Option<&HeldBy>, Option<&Filed>)>,
    mut commands: Commands,
) {
    for (mob, pos, mut pending, mut hands, mut out, mut outbox, vitals) in &mut mobs {
        let awake = vitals.is_none_or(|v| v.stat == MobStat::Conscious);
        let active_hand = hands.active;
        let holding_pen = hands
            .active_item()
            .and_then(|i| looks.get(i).ok())
            .is_some_and(|a| is_pen(&a.sprite));
        for op in std::mem::take(&mut pending.0) {
            if !awake {
                continue;
            }
            match op {
                PaperOp::Write(sheet, text) => {
                    if !holding_pen || !reachable(mob, pos.0, sheet, &places) {
                        continue;
                    }
                    if let Ok(mut paper) = papers.get_mut(sheet) {
                        paper.text = text.chars().take(MAX_TEXT).collect();
                    }
                }
                PaperOp::Label(sheet, label) => {
                    // Still in hand (Luna checks `loc == usr`).
                    if hands.left != Some(sheet) && hands.right != Some(sheet) {
                        continue;
                    }
                    if let Ok(mut paper) = papers.get_mut(sheet) {
                        let label: String = label.trim().chars().take(MAX_LABEL).collect();
                        paper.name = if label.is_empty() {
                            "paper".to_owned()
                        } else {
                            format!("paper- '{label}'")
                        };
                    }
                }
                op => {
                    let board = match &op {
                        PaperOp::Pen(b)
                        | PaperOp::WriteSheet(b, _)
                        | PaperOp::Remove(b, _)
                        | PaperOp::Read(b, _)
                        | PaperOp::Top(b, _) => *b,
                        _ => continue,
                    };
                    // Only a clipboard you carry (Luna: `src in usr.contents`).
                    if hands.left != Some(board) && hands.right != Some(board) {
                        continue;
                    }
                    let Ok(mut clip) = clipboards.get_mut(board) else {
                        continue;
                    };
                    let nth = |clip: &Clipboard, n: usize| clip.papers.iter().rev().nth(n).copied();
                    match op {
                        PaperOp::Pen(_) => match (clip.pen, hands.active_item()) {
                            (Some(pen), None) => {
                                clip.pen = None;
                                commands.entity(pen).remove::<Filed>().insert(HeldBy(mob));
                                *hands.slot_mut(active_hand) = Some(pen);
                            }
                            (None, Some(pen)) if holding_pen => {
                                clip.pen = Some(pen);
                                *hands.slot_mut(active_hand) = None;
                                commands.entity(pen).remove::<HeldBy>().insert(Filed(board));
                                outbox.0.push(LocMsg::new("clipboard-pen").arg("item", "pen"));
                            }
                            _ => {}
                        },
                        PaperOp::WriteSheet(_, n) => {
                            if let Some(sheet) = nth(&clip, n)
                                && holding_pen
                                && let Ok(paper) = papers.get(sheet)
                            {
                                out.0.push(view(sheet, paper, PaperMode::Write));
                            }
                        }
                        PaperOp::Read(_, n) => {
                            if let Some(sheet) = nth(&clip, n)
                                && let Ok(paper) = papers.get(sheet)
                            {
                                out.0.push(view(sheet, paper, PaperMode::Read));
                            }
                        }
                        PaperOp::Remove(_, n) => {
                            if let Some(sheet) = nth(&clip, n)
                                && hands.active_item().is_none()
                            {
                                clip.papers.retain(|e| *e != sheet);
                                commands.entity(sheet).remove::<Filed>().insert(HeldBy(mob));
                                *hands.slot_mut(active_hand) = Some(sheet);
                            }
                        }
                        PaperOp::Top(_, n) => {
                            if let Some(sheet) = nth(&clip, n) {
                                clip.papers.retain(|e| *e != sheet);
                                clip.papers.push(sheet);
                                outbox.0.push(LocMsg::new("clipboard-top").arg("item", PAPER));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

fn view(sheet: Entity, paper: &Paper, mode: PaperMode) -> PaperWindow {
    PaperWindow {
        sheet,
        title: paper.name.clone(),
        text: paper.text.clone(),
        mode,
    }
}

/// Keep each viewer's clipboard window in step; close it once they no longer carry it.
pub fn update_clipboard_views(
    mut viewers: Query<(Entity, &Hands, Option<&PanelViewer>, &mut ClipboardOut)>,
    clipboards: Query<(&Clipboard, &NetId)>,
    papers: Query<&Paper>,
    mut commands: Commands,
) {
    for (mob, hands, viewer, mut out) in &mut viewers {
        let target = viewer.map(|v| v.0).filter(|t| clipboards.contains(*t));
        let shown = target.and_then(|board| {
            if hands.left != Some(board) && hands.right != Some(board) {
                return None;
            }
            let (clip, id) = clipboards.get(board).ok()?;
            Some(ClipboardView {
                target: *id,
                has_pen: clip.pen.is_some(),
                papers: clip
                    .papers
                    .iter()
                    .rev()
                    .filter_map(|p| papers.get(*p).ok())
                    .map(|p| p.name.clone())
                    .collect(),
            })
        });
        if shown.is_none() && target.is_some() {
            commands.entity(mob).remove::<PanelViewer>();
        }
        if out.0 != shown {
            out.0 = shown;
        }
    }
}

/// Sprites: a bin full or empty; a clipboard with its top sheet, pen and clip (Luna
/// `update_icon`).
pub fn update_paperwork_looks(
    mut bins: Query<(&PaperBin, &mut Appearance), (Changed<PaperBin>, Without<Clipboard>)>,
    mut clipboards: Query<(&Clipboard, &mut Appearance), Changed<Clipboard>>,
) {
    for (bin, mut look) in &mut bins {
        let state = if bin.amount == 0 {
            "paper_bin0"
        } else {
            "paper_bin1"
        };
        if look.state != state {
            look.state = state.to_owned();
        }
    }
    for (clip, mut look) in &mut clipboards {
        let mut state = String::new();
        if !clip.papers.is_empty() {
            state.push_str("+paper");
        }
        if clip.pen.is_some() {
            state.push_str("+clipboard_pen");
        }
        state.push_str("+clipboard_over");
        if look.state != state {
            look.state = state;
        }
    }
}
