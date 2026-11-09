//! Forensics: the fingerprints and fibres hands leave on what they touch, the detective's
//! scanner and fingerprint cards, and the forensic scanning computer (Luna `add_fingerprint`,
//! `DetectiveWork.dm`, `detective_scanner`). Spec: docs/reference/forensics.md.
//!
//! - **Prints**: a person's fingerprint is a hash of their genes' unique identifier, so it
//!   changes with how they look. Anything a bare hand touches keeps the last three different
//!   prints; gloves and the no-prints gene leave none.
//! - **Fibres**: items touched keep, by chance, a trace of what the toucher wore: the suit,
//!   the uniform where the suit leaves the arms bare, and the gloves where it leaves the hands.
//! - **The scanner** reads a person's prints or the prints on a thing, and while printing is
//!   on makes a fingerprint card of them (it holds 20 blank cards; blank cards top it up).
//! - **The computer** (forensics access) takes in an item, scans it in 15 s, and prints what
//!   it found on paper.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{
    Appearance, Dense, DisplayName, GridPos, MobStat, NetId, ObjectSpec, PickListView, Slot, TilePos,
};

use crate::actions::{Outbox, in_reach};
use crate::closets::InCloset;
use crate::equipment::{Equipment, Wearable};
use crate::filing::{PendingPicks, PickListOut};
use crate::genetics::Gene;
use crate::hands::{Hands, HeldBy, Item};
use crate::health::Vitals;
use crate::movement::Mob;
use crate::power::{Channel, PowerUser};
use crate::rng::GameRng;
use crate::time::WorldTime;
use crate::wires::PanelViewer;

pub const SCANNER: &str = "forensic-scanner";
pub const CARD: &str = "fingerprint-card";
pub const COMPUTER: &str = "forensic-computer";
/// The detective's older model (Luna `forensic_scanning/detective`, "PowerScan Mk.I").
pub const OLD_COMPUTER: &str = "forensic-computer-old";
const PAPER: &str = "paper";
const ACCESS: &str = "forensics_lockers";

/// The most different prints a thing keeps (Luna keeps three).
pub const MAX_PRINTS: usize = 3;
/// Blank cards a scanner holds (Luna `amount`).
pub const SCANNER_CARDS: u8 = 20;
/// Blank cards in a fresh fingerprint card stack (Luna `f_card.amount`).
pub const CARD_STACK: u8 = 10;
/// Each of the computer's three scanning stages, ds.
pub const STAGE_DS: f32 = 50.0;

/// Luna's `body_parts_covered` bits for arms and hands.
const COVERS_ARMS: u16 = 32;
const COVERS_HANDS: u16 = 64;

/// Which worn thing a fibre came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FibreKind {
    Suit,
    Uniform,
    Gloves,
}

impl FibreKind {
    fn id(self) -> &'static str {
        match self {
            Self::Suit => "suit",
            Self::Uniform => "uniform",
            Self::Gloves => "gloves",
        }
    }
}

/// What hands have left on a thing.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct Traces {
    /// Fingerprints, oldest first.
    pub prints: Vec<String>,
    /// Fibres: from what kind of clothing, and its prototype id.
    pub fibres: Vec<(FibreKind, String)>,
}

/// A forensic scanner: blank cards left and whether it prints them.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scanner {
    pub cards: u8,
    pub printing: bool,
}

/// A fingerprint card: blank ones come in stacks, printed ones carry prints.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct FingerprintCard {
    pub blanks: u8,
    pub prints: Vec<String>,
    /// Whose prints, when taken from a person.
    pub of: Option<String>,
}

/// What a scan found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanResult {
    /// The item's prototype id.
    pub item: String,
    pub prints: Vec<String>,
    pub fibres: Vec<(FibreKind, String)>,
}

/// A forensic scanning computer.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct ForensicComputer {
    pub authenticated: bool,
    /// The item inside.
    pub item: Option<Entity>,
    /// A scan under way: when each of its stages ends, ds, and the stage reached (0 to 2).
    pub scanning: Option<(f32, u8)>,
    pub result: Option<ScanResult>,
    /// A short note on the screen until cleared (Luna `temp`), by message id.
    pub note: Option<&'static str>,
}

/// A button of the computer's window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Text,
    Clear,
    LogIn,
    LogOut,
    Insert,
    Eject,
    Scan,
    Cancel,
    Print,
    Erase,
}

pub fn is_computer(proto: &str) -> bool {
    matches!(proto, COMPUTER | OLD_COMPUTER)
}

pub fn spawn_computer(world: &mut World, proto: &str, pos: TilePos, id: NetId) -> Entity {
    let state = if proto == OLD_COMPUTER { "old" } else { "forensic" };
    world
        .spawn((
            GridPos(pos),
            id,
            Dense,
            Appearance::new(proto, state),
            ForensicComputer::default(),
            PowerUser::new(Channel::Equipment, 100.0),
        ))
        .id()
}

/// Give a freshly spawned item what it needs to work (the scanner's cards, a card stack).
pub fn equip_item(world: &mut World, proto: &str, item: Entity) {
    match proto {
        SCANNER => {
            world.entity_mut(item).insert(Scanner {
                cards: SCANNER_CARDS,
                printing: false,
            });
        }
        CARD => {
            world.entity_mut(item).insert(FingerprintCard {
                blanks: CARD_STACK,
                prints: Vec::new(),
                of: None,
            });
        }
        _ => {}
    }
}

fn tell(world: &mut World, mob: Entity, msg: LocMsg) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(msg);
    }
}

fn sprite(world: &World, e: Entity) -> Option<String> {
    world.get::<Appearance>(e).map(|a| a.sprite.clone())
}

fn name_of(world: &World, e: Entity) -> String {
    world
        .get::<DisplayName>(e)
        .map_or_else(String::new, |n| n.0.clone())
}

fn now(world: &World) -> f32 {
    world.get_resource::<WorldTime>().map_or(0.0, |t| t.deciseconds())
}

/// A fingerprint from a genes' unique identifier: 32 hex digits, the same for the same genes.
pub fn print_of(ui: &str) -> String {
    let hash = |seed: u64| {
        ui.bytes().fold(seed, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
        })
    };
    format!(
        "{:016x}{:016x}",
        hash(0xcbf2_9ce4_8422_2325),
        hash(0x8422_2325_cbf2_9ce4)
    )
}

/// Someone's fingerprint, if they're a person.
pub fn fingerprint(world: &mut World, mob: Entity) -> Option<String> {
    if !crate::genetics::is_person(world, mob) {
        return None;
    }
    crate::genetics::genome(world, mob).map(|g| print_of(&g.ui))
}

fn worn(world: &World, mob: Entity, slot: Slot) -> Option<Entity> {
    world.get::<Equipment>(mob).and_then(|e| e.get(slot))
}

/// Whether someone's hands leave prints: not in gloves, and not with the no-prints gene.
pub fn leaves_prints(world: &World, mob: Entity) -> bool {
    worn(world, mob, Slot::Gloves).is_none() && !crate::genetics::powers::has(world, mob, Gene::NoPrints)
}

/// Add a print to a list, keeping only the latest [`MAX_PRINTS`] different ones.
pub fn add_print(prints: &mut Vec<String>, print: String) {
    prints.retain(|p| *p != print);
    while prints.len() >= MAX_PRINTS {
        prints.remove(0);
    }
    prints.push(print);
}

/// The fibres someone might leave, each with its chance in percent (Luna `add_fibers`).
fn fibre_chances(world: &World, mob: Entity) -> Vec<(FibreKind, Entity, f32)> {
    let suit = worn(world, mob, Slot::Suit);
    let uniform = worn(world, mob, Slot::Uniform);
    let gloves = worn(world, mob, Slot::Gloves);
    let mut out = Vec::new();
    match suit {
        Some(suit) => {
            out.push((FibreKind::Suit, suit, 10.0));
            let covers = world.get::<Wearable>(suit).map_or(0, |w| w.covers);
            if covers & COVERS_ARMS == 0
                && let Some(u) = uniform
            {
                out.push((FibreKind::Uniform, u, 12.0));
            }
            if covers & COVERS_HANDS == 0
                && let Some(g) = gloves
            {
                out.push((FibreKind::Gloves, g, 20.0));
            }
        }
        None => {
            if let Some(u) = uniform {
                out.push((FibreKind::Uniform, u, 25.0));
            }
            if let Some(g) = gloves {
                out.push((FibreKind::Gloves, g, 20.0));
            }
        }
    }
    out
}

/// A person's hand touches a thing: prints if they leave them, and on items fibres by chance.
pub fn touch(world: &mut World, mob: Entity, thing: Entity) {
    if thing == mob || world.get::<Mob>(thing).is_some() || world.get_entity(thing).is_err() {
        return;
    }
    let Some(print) = fingerprint(world, mob) else {
        return;
    };
    let mut fibres = Vec::new();
    if world.get::<Item>(thing).is_some() {
        for (kind, cloth, chance) in fibre_chances(world, mob) {
            let Some(id) = sprite(world, cloth) else {
                continue;
            };
            if world.resource_mut::<GameRng>().prob(chance) {
                fibres.push((kind, id));
            }
        }
    }
    let prints = leaves_prints(world, mob);
    if !prints && fibres.is_empty() {
        return;
    }
    let mut traces = world.get::<Traces>(thing).cloned().unwrap_or_default();
    if prints {
        add_print(&mut traces.prints, print);
    }
    for fibre in fibres {
        if !traces.fibres.contains(&fibre) {
            traces.fibres.push(fibre);
        }
    }
    world.entity_mut(thing).insert(traces);
}

/// A hand used on something, with or without a held item: the item is touched, and the
/// target too unless the item is the scanner (which reads prints without leaving any).
pub fn touch_with(world: &mut World, mob: Entity, target: Entity, held: Option<Entity>) {
    if let Some(item) = held {
        touch(world, mob, item);
    }
    let scanning = held.and_then(|i| sprite(world, i)).as_deref() == Some(SCANNER);
    if held != Some(target) && !scanning {
        touch(world, mob, target);
    }
}

fn set_state(world: &mut World, item: Entity, state: &str) {
    if let Some(mut look) = world.get_mut::<Appearance>(item)
        && look.state != state
    {
        look.state = state.to_owned();
    }
}

/// The scanner used in the hand: printing on or off.
pub fn toggle_printing(world: &mut World, mob: Entity, scanner: Entity) {
    let Some(mut s) = world.get_mut::<Scanner>(scanner) else {
        return;
    };
    s.printing = !s.printing;
    let on = s.printing;
    set_state(world, scanner, if on { "forensic1" } else { "forensic0" });
    tell(
        world,
        mob,
        LocMsg::new(if on {
            "forensic-printing-on"
        } else {
            "forensic-printing-off"
        }),
    );
}

/// Blank cards put into the scanner, up to what it holds; any left over stay in the stack.
pub fn load_cards(world: &mut World, mob: Entity, scanner: Entity, card: Entity) {
    let Some(blanks) = world
        .get::<FingerprintCard>(card)
        .filter(|c| c.prints.is_empty())
        .map(|c| c.blanks)
    else {
        return;
    };
    let Some(mut s) = world.get_mut::<Scanner>(scanner) else {
        return;
    };
    let room = SCANNER_CARDS.saturating_sub(s.cards);
    if room == 0 {
        return;
    }
    let moved = room.min(blanks);
    s.cards += moved;
    if moved == blanks {
        world.entity_mut(card).despawn();
    } else if let Some(mut c) = world.get_mut::<FingerprintCard>(card) {
        c.blanks -= moved;
    }
    tell(
        world,
        mob,
        LocMsg::new("forensic-cards-loaded").num("value", i64::from(moved)),
    );
}

/// Make a card of some prints if printing is on and a blank is left (Luna prints it at the
/// user's feet). Without blanks printing turns off.
fn print_card(world: &mut World, mob: Entity, scanner: Entity, prints: &[String], of: Option<String>) {
    let Some(s) = world.get::<Scanner>(scanner).copied() else {
        return;
    };
    if !s.printing {
        return;
    }
    if s.cards == 0 {
        if let Some(mut s) = world.get_mut::<Scanner>(scanner) {
            s.printing = false;
        }
        set_state(world, scanner, "forensic0");
        tell(world, mob, LocMsg::new("forensic-need-cards"));
        return;
    }
    if let Some(mut s) = world.get_mut::<Scanner>(scanner) {
        s.cards -= 1;
    }
    let Some(at) = world.get::<GridPos>(mob).map(|p| p.0) else {
        return;
    };
    if let Ok(card) = crate::objects::spawn_object(world, &ObjectSpec::new(CARD), at) {
        world.entity_mut(card).insert(FingerprintCard {
            blanks: 0,
            prints: prints.to_vec(),
            of,
        });
        set_state(world, card, "fingerprint1");
    }
    tell(world, mob, LocMsg::new("forensic-printed"));
}

/// The scanner used on someone or something (Luna `detective_scanner/attack`, `afterattack`).
pub fn scan(world: &mut World, mob: Entity, scanner: Entity, target: Entity) {
    if world.get::<Scanner>(scanner).is_none() {
        return;
    }
    if world.get::<Mob>(target).is_some() {
        let name = name_of(world, target);
        let print = fingerprint(world, target).filter(|_| leaves_prints(world, target));
        match print {
            None => tell(world, mob, LocMsg::new("forensic-no-prints-on").arg("name", name)),
            Some(print) => {
                print_card(
                    world,
                    mob,
                    scanner,
                    std::slice::from_ref(&print),
                    Some(name.clone()),
                );
                tell(
                    world,
                    mob,
                    LocMsg::new("forensic-prints-of")
                        .arg("name", name)
                        .arg("print", print),
                );
            }
        }
        return;
    }
    let item = sprite(world, target).unwrap_or_default();
    let prints = world
        .get::<Traces>(target)
        .map(|t| t.prints.clone())
        .unwrap_or_default();
    if prints.is_empty() {
        tell(world, mob, LocMsg::new("forensic-no-prints").arg("item", item));
        return;
    }
    print_card(world, mob, scanner, &prints, None);
    tell(
        world,
        mob,
        LocMsg::new("forensic-isolated").num("value", prints.len() as i64),
    );
    for print in prints {
        tell(world, mob, LocMsg::new("forensic-print-line").arg("print", print));
    }
}

/// A fingerprint card read in the hand.
pub fn read_card(world: &mut World, mob: Entity, card: Entity) {
    let Some(c) = world.get::<FingerprintCard>(card).cloned() else {
        return;
    };
    if c.prints.is_empty() {
        tell(
            world,
            mob,
            LocMsg::new("forensic-card-blank").num("value", i64::from(c.blanks)),
        );
        return;
    }
    if let Some(name) = c.of {
        tell(world, mob, LocMsg::new("forensic-card-of").arg("name", name));
    }
    for print in c.prints {
        tell(world, mob, LocMsg::new("forensic-print-line").arg("print", print));
    }
}

fn powered(world: &World, machine: Entity) -> bool {
    world.get::<PowerUser>(machine).is_some_and(|p| p.powered)
}

/// A click on the computer opens its window.
pub fn click(world: &mut World, mob: Entity, computer: Entity) {
    if world.get::<ForensicComputer>(computer).is_none() || !powered(world, computer) {
        return;
    }
    world.entity_mut(mob).insert(PanelViewer(computer));
}

/// The window's lines, each with what pressing it does.
fn lines(world: &World, computer: &ForensicComputer) -> Vec<(String, Op)> {
    let mut out = Vec::new();
    if let Some(note) = computer.note {
        out.push((format!("msg:{note}"), Op::Text));
        out.push(("msg:forensic-clear".to_owned(), Op::Clear));
        return out;
    }
    if !computer.authenticated {
        out.push(("msg:forensic-log-in".to_owned(), Op::LogIn));
        return out;
    }
    out.push(("msg:forensic-log-out".to_owned(), Op::LogOut));
    match computer.item.and_then(|i| sprite(world, i)) {
        Some(item) => {
            let row = format!("msg:forensic-object:{item}");
            if computer.scanning.is_some() {
                out.push((row, Op::Text));
                out.push(("msg:forensic-cancel".to_owned(), Op::Cancel));
            } else {
                out.push((row, Op::Eject));
                out.push(("msg:forensic-scan".to_owned(), Op::Scan));
                out.push(("msg:forensic-print".to_owned(), Op::Print));
            }
        }
        None => {
            out.push(("msg:forensic-no-object".to_owned(), Op::Insert));
            out.push(("msg:forensic-print".to_owned(), Op::Print));
        }
    }
    if let Some((_, stage)) = computer.scanning {
        let done = 25 * (i64::from(stage) + 1);
        out.push((format!("msg:forensic-scanning:{done}"), Op::Text));
    } else if let Some(result) = &computer.result {
        out.extend(result_rows(result).into_iter().map(|r| (r, Op::Text)));
        out.push(("msg:forensic-erase".to_owned(), Op::Erase));
    }
    out
}

/// What a scan found, as window rows.
pub fn result_rows(result: &ScanResult) -> Vec<String> {
    let mut out = vec![format!("msg:forensic-scan-of:{}", result.item)];
    if result.prints.is_empty() {
        out.push("msg:forensic-no-prints-found".to_owned());
    } else {
        out.push(format!("msg:forensic-isolated:{}", result.prints.len()));
        for (i, print) in result.prints.iter().enumerate() {
            out.push(format!("msg:forensic-print-row:#{} - {print}", i + 1));
        }
    }
    if result.fibres.is_empty() {
        out.push("msg:forensic-no-fibres".to_owned());
    } else {
        out.push("msg:forensic-fibres-found".to_owned());
        for (kind, id) in &result.fibres {
            out.push(format!("msg:forensic-fibre-{}:{id}", kind.id()));
        }
    }
    out
}

/// What a scan found, as a printed sheet in the reader's language.
pub fn result_paper(result: &ScanResult) -> String {
    let mut out = format!("[msg:forensic-paper-title] [noun:{}]\n\n", result.item);
    if result.prints.is_empty() {
        out.push_str("[msg:forensic-paper-no-prints]\n");
    } else {
        out.push_str(&format!("[msg:forensic-paper-prints] {}\n", result.prints.len()));
        for (i, print) in result.prints.iter().enumerate() {
            out.push_str(&format!("#{} - {print}\n", i + 1));
        }
    }
    out.push('\n');
    if result.fibres.is_empty() {
        out.push_str("[msg:forensic-paper-no-fibres]\n");
    } else {
        out.push_str("[msg:forensic-paper-fibres]\n");
        for (kind, id) in &result.fibres {
            out.push_str(&format!(
                "- [msg:forensic-paper-fibre-{}] [noun:{id}]\n",
                kind.id()
            ));
        }
    }
    out
}

/// Buttons pressed on forensic computers (sent as picks); others are left for the next module.
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
            if world.get::<ForensicComputer>(target).is_some() {
                press(world, mob, target, line);
            } else {
                others.push((target, line));
            }
        }
        if !others.is_empty()
            && let Some(mut pending) = world.get_mut::<PendingPicks>(mob)
        {
            pending.0.extend(others);
        }
    }
}

fn awake(world: &World, mob: Entity) -> bool {
    world
        .get::<Vitals>(mob)
        .is_none_or(|v| v.stat == MobStat::Conscious)
}

fn near(world: &World, mob: Entity, machine: Entity) -> bool {
    match (world.get::<GridPos>(mob), world.get::<GridPos>(machine)) {
        (Some(a), Some(b)) => in_reach(a.0, b.0),
        _ => false,
    }
}

fn press(world: &mut World, mob: Entity, computer: Entity, line: usize) {
    if !awake(world, mob) || !near(world, mob, computer) || !powered(world, computer) {
        return;
    }
    let Some(c) = world.get::<ForensicComputer>(computer).cloned() else {
        return;
    };
    let Some(op) = lines(world, &c).get(line).map(|(_, op)| *op) else {
        return;
    };
    let mut c = c;
    match op {
        Op::Text => return,
        Op::Clear => c.note = None,
        Op::LogIn => {
            // Luna checks the card held in the hand.
            let held = world.get::<Hands>(mob).and_then(Hands::active_item);
            let card = held.and_then(|i| world.get::<crate::access::IdCard>(i));
            if card.is_some_and(|card| card.access.iter().any(|a| a == ACCESS)) {
                c.authenticated = true;
            }
        }
        Op::LogOut => c.authenticated = false,
        Op::Insert => {
            let held = world.get::<Hands>(mob).and_then(Hands::active_item);
            match held {
                Some(item) => {
                    if let Some(mut hands) = world.get_mut::<Hands>(mob) {
                        let active = hands.active;
                        *hands.slot_mut(active) = None;
                    }
                    world
                        .entity_mut(item)
                        .remove::<HeldBy>()
                        .insert(InCloset(computer));
                    c.item = Some(item);
                }
                None => c.note = Some("forensic-note-rejected"),
            }
        }
        Op::Eject => {
            if let (Some(item), Some(at)) = (c.item.take(), world.get::<GridPos>(computer).copied()) {
                world.entity_mut(item).remove::<InCloset>().insert(at);
            }
        }
        Op::Scan => {
            if c.item.is_some() {
                c.scanning = Some((now(world) + STAGE_DS, 0));
                c.result = None;
            }
        }
        Op::Cancel => {
            c.scanning = None;
        }
        Op::Print => match &c.result {
            Some(result) => {
                let info = result_paper(result);
                c.note = Some("forensic-note-printed");
                if let Some(at) = world.get::<GridPos>(computer).map(|p| p.0) {
                    let mut sheet = ObjectSpec::new(PAPER);
                    sheet
                        .vars
                        .insert("name".to_owned(), "[msg:forensic-scan-paper]".to_owned());
                    sheet.vars.insert("info".to_owned(), info);
                    let _ = crate::objects::spawn_object(world, &sheet, at);
                }
            }
            None => c.note = Some("forensic-note-no-data"),
        },
        Op::Erase => c.result = None,
    }
    world.entity_mut(computer).insert(c);
}

/// Scans under way go stage by stage; the last reads what's on the item.
pub fn run_computers(world: &mut World) {
    let now = now(world);
    let mut q = world.query::<(Entity, &ForensicComputer)>();
    let due: Vec<(Entity, u8, Option<Entity>)> = q
        .iter(world)
        .filter_map(|(e, c)| {
            c.scanning
                .filter(|(at, _)| now >= *at)
                .map(|(_, s)| (e, s, c.item))
        })
        .collect();
    for (computer, stage, item) in due {
        let result = (stage >= 2).then(|| {
            item.map(|item| {
                let traces = world.get::<Traces>(item).cloned().unwrap_or_default();
                ScanResult {
                    item: sprite(world, item).unwrap_or_default(),
                    prints: traces.prints,
                    fibres: traces.fibres,
                }
            })
        });
        if let Some(mut c) = world.get_mut::<ForensicComputer>(computer) {
            match result {
                Some(found) => {
                    c.scanning = None;
                    c.result = found;
                }
                None => c.scanning = Some((now + STAGE_DS, stage + 1)),
            }
        }
    }
}

/// Keep each open computer window in step, and close it out of reach or without power.
pub fn update_views(world: &mut World) {
    let mut viewers = world.query::<(Entity, &PanelViewer)>();
    let all: Vec<(Entity, Entity)> = viewers.iter(world).map(|(m, v)| (m, v.0)).collect();
    for (mob, target) in all {
        let Some(c) = world.get::<ForensicComputer>(target).cloned() else {
            continue;
        };
        let open = near(world, mob, target) && powered(world, target) && awake(world, mob);
        let view = match (open, world.get::<NetId>(target).copied(), sprite(world, target)) {
            (true, Some(id), Some(title)) => Some(PickListView {
                target: id,
                title,
                rows: lines(world, &c).into_iter().map(|(row, _)| row).collect(),
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

/// Whether the filing cabinets' list update should leave this window alone.
pub fn keeps_own_list(sprite: &str) -> bool {
    is_computer(sprite)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prints_are_stable_and_distinct() {
        assert_eq!(print_of("abc"), print_of("abc"));
        assert_ne!(print_of("abc"), print_of("abd"));
        assert_eq!(print_of("abc").len(), 32);
    }

    #[test]
    fn only_the_latest_three_different_prints_are_kept() {
        let mut prints = Vec::new();
        for p in ["a", "b", "a", "c", "d"] {
            add_print(&mut prints, p.to_owned());
        }
        assert_eq!(prints, ["a", "c", "d"]);
    }

    #[test]
    fn a_scan_reads_out_prints_and_fibres() {
        let result = ScanResult {
            item: "crowbar".to_owned(),
            prints: vec!["ff".to_owned()],
            fibres: vec![(FibreKind::Gloves, "black-gloves".to_owned())],
        };
        let rows = result_rows(&result);
        assert!(rows.contains(&"msg:forensic-print-row:#1 - ff".to_owned()));
        assert!(rows.contains(&"msg:forensic-fibre-gloves:black-gloves".to_owned()));
        let paper = result_paper(&result);
        assert!(paper.contains("[noun:crowbar]") && paper.contains("[noun:black-gloves]"));
    }
}
