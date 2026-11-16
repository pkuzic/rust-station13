//! Implants (Luna's `weapons/implants.dm`, `computer/prisoners.dm`): the implanter, glass case and
//! implant pad, the tracking, chemical, death alarm, freedom, explosive and compressed matter
//! implants, and the prisoner management console. Spec: `docs/reference/implants.md`.
//!
//! - **Putting one in**: an implanter with an implant, used on someone in reach (or yourself),
//!   takes 6 seconds while both stay put; the implant goes into a body part that has none.
//! - **Cases and the pad** move implants in and out of implanters; the pad shows what's in a case
//!   and sets a tracking implant's frequency and id.
//! - **What they do**: tracking implants are teleporter targets and show on the prisoner console,
//!   which also injects from chemical implants; a chemical implant empties itself when its host
//!   dies; a death alarm tells the common channel; the freedom and compressed matter implants answer
//!   an emote of the host's choice, the explosive one a phrase anyone near says.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{
    Appearance, Dense, DisplayName, GridPos, MapLayer, MobStat, NetId, PickListView, Slot, TilePos,
};

use crate::actions::{Outbox, in_reach};
use crate::chemistry::Reagents;
use crate::combat::VisibleEvents;
use crate::equipment::{Equipment, WornBy};
use crate::filing::{PendingPicks, PickListOut};
use crate::hands::{Hands, HeldBy};
use crate::health::{Vitals, Zone};
use crate::power::{Channel, PowerUser};
use crate::radio::{COMMON_FREQUENCY, Radio, format_frequency};
use crate::rng::GameRng;
use crate::time::WorldTime;
use crate::wires::PanelViewer;

pub const IMPLANTER: &str = "implanter";
pub const CASE: &str = "implant-case";
pub const PAD: &str = "implant-pad";
pub const CONSOLE: &str = "prisoner-console";
/// The compressed matter implanter, which scans an item before it implants.
pub const COMPRESS_IMPLANTER: &str = "implanter-compressed";

pub const TRACKING: &str = "implant-tracking";
pub const CHEM: &str = "implant-chem";
pub const DEATH_ALARM: &str = "implant-death-alarm";
pub const FREEDOM: &str = "implant-freedom";
pub const EXPLOSIVE: &str = "implant-explosive";
pub const COMPRESSED: &str = "implant-compressed";

/// How long implanting takes, ds (Luna `do_mob(user, target, 60)`).
pub const IMPLANT_DS: f32 = 60.0;
/// What a chemical implant holds (Luna `volume`).
pub const CHEM_HOLDS: f32 = 50.0;
/// What a syringe puts into a chemical implant in its case at a time.
pub const CASE_FILL: f32 = 5.0;
/// A tracking implant's frequency and id to start (Luna `frequency`, `id`).
pub const TRACKING_FREQ: u16 = 1451;
/// A host dead longer than this no longer shows as a teleporter target, ds (Luna's 6000).
pub const TRACKING_DEAD_DS: f32 = 6000.0;
/// What the freedom implant gives (Luna's synaptizine; its hyperzine isn't built).
pub const FREEDOM_DOSE: f32 = 10.0;
/// What an explosive implant sets off (Luna `explosion(..., 1, 3, 4, 6)`).
pub const BLAST: (i32, i32, i32, i32) = (1, 3, 4, 6);

/// The emotes an implant can be set to answer (Luna's list).
pub const EMOTE_CHOICES: [&str; 17] = [
    "blink", "blink-r", "eyebrow", "chuckle", "twitch-s", "frown", "nod", "blush", "giggle", "grin", "groan",
    "shrug", "smile", "pale", "sniff", "whimper", "wink",
];

#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Tracking {
        frequency: u16,
        id: u8,
    },
    Chem,
    DeathAlarm {
        fired: bool,
    },
    Freedom {
        uses: u8,
        emote: Option<String>,
    },
    Explosive {
        phrase: Option<String>,
    },
    Compressed {
        emote: Option<String>,
        scanned: Option<Entity>,
    },
}

/// An implant: what it is, and who it's in and where.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Implant {
    pub kind: Kind,
    pub host: Option<Entity>,
    pub zone: Option<Zone>,
}

/// The implant in an implanter, a glass case, or the case in an implant pad.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct Implanter(pub Option<Entity>);

#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct ImplantCase(pub Option<Entity>);

#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct ImplantPad(pub Option<Entity>);

/// The prisoner management console: unlocked with armoury access.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct PrisonerConsole {
    pub unlocked: bool,
}

/// Someone implanting someone: done at this time, if both stay put.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Implanting {
    pub target: Entity,
    pub implanter: Entity,
    pub done_ds: f32,
    pub from: TilePos,
    pub target_from: TilePos,
}

/// The next thing this host says sets this implant's phrase, unheard.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ChoosingPhrase(pub Entity);

/// When a host died, for how long a tracking implant in them still shows.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct DiedAt(pub f32);

/// What implants may hear this tick: lines said aloud (speaker, where, text), emotes done
/// (who, which), and phrases chosen (who, text).
#[derive(Resource, Debug, Default)]
pub struct Heard {
    pub said: Vec<(Entity, TilePos, String)>,
    pub emoted: Vec<(Entity, String)>,
    pub phrases: Vec<(Entity, String)>,
}

pub fn is_implant(sprite: &str) -> bool {
    sprite == "implant"
}

/// What a fresh implant of this prototype is.
fn new_kind(proto: &str, rng: &mut GameRng) -> Option<Kind> {
    Some(match proto {
        TRACKING => Kind::Tracking {
            frequency: TRACKING_FREQ,
            id: 1,
        },
        CHEM => Kind::Chem,
        DEATH_ALARM => Kind::DeathAlarm { fired: false },
        FREEDOM => Kind::Freedom {
            uses: rng.range(1, 5) as u8,
            emote: None,
        },
        EXPLOSIVE => Kind::Explosive { phrase: None },
        COMPRESSED => Kind::Compressed {
            emote: None,
            scanned: None,
        },
        _ => return None,
    })
}

/// What a prototype comes with inside it: an implanter or case with its implant.
fn comes_with(proto: &str) -> Option<&'static str> {
    Some(match proto {
        "implant-case-tracking" => TRACKING,
        "implant-case-chem" => CHEM,
        "implant-case-death-alarm" => DEATH_ALARM,
        "implanter-freedom" => FREEDOM,
        "implanter-explosive" => EXPLOSIVE,
        COMPRESS_IMPLANTER => COMPRESSED,
        _ => return None,
    })
}

/// The look of an implant, a case or an implanter by what's in it (Luna's `update` procs).
fn implant_state(kind: &Kind) -> &'static str {
    match kind {
        Kind::Chem => "b",
        Kind::Tracking { .. } | Kind::DeathAlarm { .. } => "",
        Kind::Freedom { .. } | Kind::Explosive { .. } | Kind::Compressed { .. } => "s-b",
    }
}

/// The parts a newly spawned implant, implanter, case, pad or console needs.
pub fn equip_item(world: &mut World, item: Entity, proto: &str) {
    let kind = {
        let mut rng = world.resource_mut::<GameRng>();
        new_kind(proto, &mut rng)
    };
    if let Some(kind) = kind {
        let chem = kind == Kind::Chem;
        let state = implant_state(&kind);
        world.entity_mut(item).insert(Implant {
            kind,
            host: None,
            zone: None,
        });
        if chem {
            world.entity_mut(item).insert(Reagents::new(CHEM_HOLDS));
        }
        if let Some(mut look) = world.get_mut::<Appearance>(item) {
            look.state = state.to_owned();
        }
        return;
    }
    let sprite = world.get::<Appearance>(item).map(|a| a.sprite.clone());
    match sprite.as_deref() {
        Some(IMPLANTER) => {
            world.entity_mut(item).insert(Implanter(None));
        }
        Some(CASE) => {
            world.entity_mut(item).insert(ImplantCase(None));
        }
        Some(PAD) => {
            world.entity_mut(item).insert(ImplantPad(None));
        }
        _ => return,
    }
    let Some(inner) = comes_with(proto) else {
        return;
    };
    let tile = world.get::<GridPos>(item).map_or(TilePos::new(0, 0, 0), |p| p.0);
    let Ok(implant) = crate::objects::spawn_object(world, &rs13_world::ObjectSpec::new(inner), tile) else {
        return;
    };
    put_into(world, implant, item);
}

fn tell(world: &mut World, mob: Entity, msg: LocMsg) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(msg);
    }
}

fn name_of(world: &World, who: Entity) -> String {
    world
        .get::<DisplayName>(who)
        .map(|n| n.0.clone())
        .unwrap_or_default()
}

fn at(world: &World, e: Entity) -> Option<TilePos> {
    world.get::<GridPos>(e).map(|p| p.0)
}

fn sprite(world: &World, e: Entity) -> Option<String> {
    world.get::<Appearance>(e).map(|a| a.sprite.clone())
}

/// Where a thing is: its tile, or its holder's, or its host's.
fn where_is(world: &World, e: Entity) -> Option<TilePos> {
    if let Some(host) = world.get::<Implant>(e).and_then(|i| i.host) {
        return at(world, host);
    }
    at(world, e)
        .or_else(|| world.get::<HeldBy>(e).and_then(|h| at(world, h.0)))
        .or_else(|| {
            world
                .get::<crate::storage::InStorage>(e)
                .and_then(|s| where_is(world, s.0))
        })
        .or_else(|| crate::devices::where_is(world, e))
}

/// Take a thing out of a hand.
fn out_of_hand(world: &mut World, thing: Entity) {
    if let Some(holder) = world.get::<HeldBy>(thing).map(|h| h.0)
        && let Some(mut hands) = world.get_mut::<Hands>(holder)
    {
        if hands.left == Some(thing) {
            hands.left = None;
        }
        if hands.right == Some(thing) {
            hands.right = None;
        }
    }
    world.entity_mut(thing).remove::<(GridPos, HeldBy)>();
}

/// Put an implant into an implanter or case, or a case into a pad, and show it.
fn put_into(world: &mut World, thing: Entity, holder: Entity) {
    out_of_hand(world, thing);
    world.entity_mut(thing).insert(crate::devices::PartOf(holder));
    if let Some(mut i) = world.get_mut::<Implanter>(holder) {
        i.0 = Some(thing);
    } else if let Some(mut c) = world.get_mut::<ImplantCase>(holder) {
        c.0 = Some(thing);
    } else if let Some(mut p) = world.get_mut::<ImplantPad>(holder) {
        p.0 = Some(thing);
    }
    refresh(world, holder);
}

fn refresh(world: &mut World, holder: Entity) {
    let state = if let Some(i) = world.get::<Implanter>(holder) {
        if i.0.is_some() { "full" } else { "" }.to_owned()
    } else if let Some(c) = world.get::<ImplantCase>(holder) {
        match c.0.and_then(|imp| world.get::<Implant>(imp)) {
            None => String::new(),
            Some(imp) if implant_state(&imp.kind).starts_with('s') => "s".to_owned(),
            Some(_) => "b".to_owned(),
        }
    } else if let Some(p) = world.get::<ImplantPad>(holder) {
        if p.0.is_some() { "full" } else { "" }.to_owned()
    } else {
        return;
    };
    if let Some(mut look) = world.get_mut::<Appearance>(holder) {
        look.state = state;
    }
}

/// Into the mob's active hand.
fn into_hand(world: &mut World, mob: Entity, thing: Entity) {
    let Some(hand) = world.get::<Hands>(mob).map(|h| h.active) else {
        return;
    };
    world
        .entity_mut(thing)
        .remove::<(GridPos, crate::devices::PartOf)>()
        .insert(HeldBy(mob));
    if let Some(mut hands) = world.get_mut::<Hands>(mob) {
        *hands.slot_mut(hand) = Some(thing);
    }
}

// --- Using the tools ---

/// Something used on an implanter, case or pad, or one of them used on something.
pub fn use_on(world: &mut World, mob: Entity, tool: Option<Entity>, target: Entity) {
    let target_sprite = sprite(world, target).unwrap_or_default();
    let tool_sprite = tool.and_then(|t| sprite(world, t)).unwrap_or_default();
    match (tool, tool_sprite.as_str(), target_sprite.as_str()) {
        // Luna `implantcase/attackby` with an implanter: the implant goes whichever way it can.
        (Some(implanter), IMPLANTER, CASE) => {
            let held = world.get::<Implanter>(implanter).and_then(|i| i.0);
            let cased = world.get::<ImplantCase>(target).and_then(|c| c.0);
            match (held, cased) {
                (Some(imp), None) => {
                    if let Some(mut i) = world.get_mut::<Implanter>(implanter) {
                        i.0 = None;
                    }
                    refresh(world, implanter);
                    put_into(world, imp, target);
                }
                (None, Some(imp)) => {
                    if let Some(mut c) = world.get_mut::<ImplantCase>(target) {
                        c.0 = None;
                    }
                    refresh(world, target);
                    put_into(world, imp, implanter);
                }
                _ => {}
            }
        }
        // A syringe into a chemical implant in its case.
        (Some(syringe), s, CASE) if crate::chemistry::is_syringe(s) => {
            let Some(imp) = world.get::<ImplantCase>(target).and_then(|c| c.0) else {
                return;
            };
            if world.get::<Implant>(imp).is_none_or(|i| i.kind != Kind::Chem) {
                return;
            }
            let full = world.get::<Reagents>(imp).is_none_or(|r| r.total() >= r.max);
            if full {
                tell(world, mob, LocMsg::new("implant-case-full"));
                return;
            }
            crate::chemistry::transfer(world, syringe, imp, CASE_FILL);
            let left = world.get::<Reagents>(syringe).map_or(0.0, Reagents::total);
            tell(
                world,
                mob,
                LocMsg::new("implant-case-injected").num("value", left.round() as i64),
            );
        }
        // A case into a pad.
        (Some(case), CASE, PAD) => {
            if world.get::<ImplantPad>(target).is_some_and(|p| p.0.is_none()) {
                put_into(world, case, target);
            }
        }
        // The pad in hand: its window; clicked from the other hand, the case comes out.
        (None, _, PAD) => {
            let case = world.get::<ImplantPad>(target).and_then(|p| p.0);
            let held_by_mob = world.get::<HeldBy>(target).is_some_and(|h| h.0 == mob);
            if let (Some(case), true) = (case, held_by_mob) {
                if let Some(mut p) = world.get_mut::<ImplantPad>(target) {
                    p.0 = None;
                }
                refresh(world, target);
                into_hand(world, mob, case);
            } else {
                world.entity_mut(mob).insert(PanelViewer(target));
            }
        }
        (Some(pad), PAD, PAD) if pad == target => {
            world.entity_mut(mob).insert(PanelViewer(target));
        }
        _ => {}
    }
}

/// An implanter used on someone (Luna `implanter/attack`): six seconds, then it's in.
pub fn start_implanting(world: &mut World, mob: Entity, implanter: Entity, target: Entity) {
    let Some(imp) = world.get::<Implanter>(implanter).and_then(|i| i.0) else {
        return;
    };
    if let Some(Implant {
        kind: Kind::Compressed { scanned: None, .. },
        ..
    }) = world.get::<Implant>(imp)
    {
        tell(world, mob, LocMsg::new("implant-scan-first"));
        return;
    }
    let (Some(from), Some(target_from)) = (at(world, mob), at(world, target)) else {
        return;
    };
    let now = world.resource::<WorldTime>().deciseconds();
    world.entity_mut(mob).insert(Implanting {
        target,
        implanter,
        done_ds: now + IMPLANT_DS,
        from,
        target_from,
    });
    let msg = if target == mob {
        LocMsg::new("implant-trying-self").arg("name", name_of(world, mob))
    } else {
        LocMsg::new("implant-trying")
            .arg("name", name_of(world, mob))
            .arg("target", name_of(world, target))
    };
    world.resource_mut::<VisibleEvents>().0.push((from, msg, None));
}

/// The compressed matter implanter used on an item (Luna `implanter/compress/afterattack`): the
/// item is taken in, to come out again when the implant is set off.
pub fn scan(world: &mut World, mob: Entity, implanter: Entity, item: Entity) {
    let Some(imp) = world.get::<Implanter>(implanter).and_then(|i| i.0) else {
        return;
    };
    let Some(mut implant) = world.get_mut::<Implant>(imp) else {
        return;
    };
    let Kind::Compressed { scanned, .. } = &mut implant.kind else {
        return;
    };
    // Luna loses whatever it held before.
    let lost = scanned.replace(item);
    if let Some(lost) = lost {
        world.entity_mut(lost).despawn();
    }
    if world.get::<InStore>(item).is_none() {
        out_of_hand(world, item);
        world.entity_mut(item).insert(InStore(imp));
    }
    tell(
        world,
        mob,
        LocMsg::new("implant-scanned").arg("item", sprite(world, item).unwrap_or_default()),
    );
}

/// An item held compressed in an implant.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct InStore(pub Entity);

/// Free zones to put an implant in.
const ZONES: [Zone; 11] = [
    Zone::Head,
    Zone::Chest,
    Zone::Groin,
    Zone::LArm,
    Zone::RArm,
    Zone::LHand,
    Zone::RHand,
    Zone::LLeg,
    Zone::RLeg,
    Zone::LFoot,
    Zone::RFoot,
];

/// The implants in someone.
pub fn implants_in(world: &mut World, host: Entity) -> Vec<Entity> {
    let mut q = world.query::<(Entity, &Implant)>();
    q.iter(world)
        .filter(|(_, i)| i.host == Some(host))
        .map(|(e, _)| e)
        .collect()
}

fn finish_implanting(world: &mut World, mob: Entity, job: Implanting) {
    let Some(imp) = world.get::<Implanter>(job.implanter).and_then(|i| i.0) else {
        return;
    };
    let taken: Vec<Zone> = implants_in(world, job.target)
        .into_iter()
        .filter_map(|e| world.get::<Implant>(e).and_then(|i| i.zone))
        .collect();
    let free: Vec<Zone> = ZONES.into_iter().filter(|z| !taken.contains(z)).collect();
    if free.is_empty() {
        let msg = LocMsg::new("implant-nowhere")
            .arg("name", name_of(world, mob))
            .arg("target", name_of(world, job.target));
        world
            .resource_mut::<VisibleEvents>()
            .0
            .push((job.from, msg, None));
        return;
    }
    let zone = free[world.resource_mut::<GameRng>().range(0, free.len() as i32 - 1) as usize];
    let msg = if job.target == mob {
        LocMsg::new("implant-done-self").arg("name", name_of(world, mob))
    } else {
        LocMsg::new("implant-done")
            .arg("name", name_of(world, mob))
            .arg("target", name_of(world, job.target))
    };
    world
        .resource_mut::<VisibleEvents>()
        .0
        .push((job.from, msg, None));
    if let Some(mut i) = world.get_mut::<Implanter>(job.implanter) {
        i.0 = None;
    }
    refresh(world, job.implanter);
    world
        .entity_mut(imp)
        .remove::<(GridPos, HeldBy, crate::devices::PartOf)>();
    let kind = {
        let Some(mut implant) = world.get_mut::<Implant>(imp) else {
            return;
        };
        implant.host = Some(job.target);
        implant.zone = Some(zone);
        implant.kind.clone()
    };
    implanted(world, job.target, imp, &kind);
}

/// Luna's `implanted` procs: what a new implant asks of its host.
fn implanted(world: &mut World, host: Entity, imp: Entity, kind: &Kind) {
    match kind {
        Kind::Freedom { .. } | Kind::Compressed { .. } => {
            tell(world, host, LocMsg::new("implant-choose-emote"));
            world.entity_mut(host).insert(PanelViewer(imp));
        }
        Kind::Explosive { .. } => {
            tell(world, host, LocMsg::new("implant-choose-phrase"));
            world.entity_mut(host).insert(ChoosingPhrase(imp));
        }
        _ => {}
    }
}

// --- Windows ---

#[derive(Debug, Clone, Copy, PartialEq)]
enum Line {
    Info,
    Freq(i16),
    Id(i8),
    Emote(usize),
    Unlock,
    Lock,
    Inject(Entity, u8),
}

fn lines(world: &mut World, target: Entity) -> Vec<(String, Line)> {
    let mut out = Vec::new();
    if let Some(implant) = world.get::<Implant>(target) {
        // Choosing an implant's emote.
        if implant.host.is_some() {
            out.push(("msg:implant-emote-title".to_owned(), Line::Info));
            for (i, e) in EMOTE_CHOICES.iter().enumerate() {
                out.push((format!("msg:implant-emote:{e}"), Line::Emote(i)));
            }
        }
        return out;
    }
    if let Some(pad) = world.get::<ImplantPad>(target).copied() {
        let Some(case) = pad.0 else {
            out.push(("msg:implant-pad-insert".to_owned(), Line::Info));
            return out;
        };
        let Some(kind) = world
            .get::<ImplantCase>(case)
            .and_then(|c| c.0)
            .and_then(|imp| world.get::<Implant>(imp))
            .map(|i| i.kind.clone())
        else {
            out.push(("msg:implant-pad-empty".to_owned(), Line::Info));
            return out;
        };
        match kind {
            Kind::Tracking { frequency, id } => {
                out.push(("msg:implant-pad-tracking".to_owned(), Line::Info));
                out.push((
                    format!("msg:implant-pad-freq:{}", format_frequency(frequency)),
                    Line::Info,
                ));
                for step in [-10, -2, 2, 10] {
                    out.push((format!("msg:implant-pad-step:{step:+}"), Line::Freq(step)));
                }
                out.push((format!("msg:implant-pad-id:{id}"), Line::Info));
                for step in [-10, -1, 1, 10] {
                    out.push((format!("msg:implant-pad-step:{step:+}"), Line::Id(step)));
                }
            }
            Kind::Freedom { .. } => out.push(("msg:implant-pad-freedom".to_owned(), Line::Info)),
            _ => out.push(("msg:implant-pad-unknown".to_owned(), Line::Info)),
        }
        return out;
    }
    if let Some(console) = world.get::<PrisonerConsole>(target).copied() {
        if !console.unlocked {
            out.push(("msg:prisoner-unlock".to_owned(), Line::Unlock));
            return out;
        }
        out.push(("msg:prisoner-chem-title".to_owned(), Line::Info));
        let mut q = world.query::<(Entity, &Implant, Option<&Reagents>)>();
        let mut chems: Vec<(Entity, Entity, f32)> = q
            .iter(world)
            .filter(|(_, i, _)| i.kind == Kind::Chem)
            .filter_map(|(e, i, r)| Some((e, i.host?, r.map_or(0.0, Reagents::total))))
            .collect();
        chems.sort_by_key(|c| c.0);
        for (imp, host, units) in chems {
            let row = format!(
                "msg:prisoner-chem:{}: {}",
                name_of(world, host),
                units.round() as i64
            );
            out.push((row, Line::Info));
            for n in [1, 5, 10] {
                out.push((format!("msg:prisoner-inject:{n}"), Line::Inject(imp, n)));
            }
        }
        out.push(("msg:prisoner-tracking-title".to_owned(), Line::Info));
        let mut q = world.query::<(Entity, &Implant)>();
        let mut tracks: Vec<(Entity, u8, Entity)> = q
            .iter(world)
            .filter_map(|(e, i)| match i.kind {
                Kind::Tracking { id, .. } => Some((e, id, i.host?)),
                _ => None,
            })
            .collect();
        tracks.sort();
        for (_, id, host) in tracks {
            let area = at(world, host).map(|p| area_of(world, p)).unwrap_or_default();
            out.push((format!("msg:prisoner-tracking:{id}: {area}"), Line::Info));
        }
        out.push(("msg:prisoner-lock".to_owned(), Line::Lock));
    }
    out
}

fn area_of(world: &mut World, tile: TilePos) -> String {
    let mut layers = world.query::<&MapLayer>();
    layers
        .iter(world)
        .find(|l| l.0.z == tile.z)
        .and_then(|l| l.0.area(tile).map(str::to_owned))
        .unwrap_or_default()
}

fn is_ours(world: &World, target: Entity) -> bool {
    world.get::<Implant>(target).is_some()
        || world.get::<ImplantPad>(target).is_some()
        || world.get::<PrisonerConsole>(target).is_some()
}

/// Whether the filing cabinets should leave this viewer's list alone.
pub fn keeps_own_list(sprite: &str) -> bool {
    matches!(sprite, PAD | CONSOLE) || is_implant(sprite)
}

fn can_work(world: &World, mob: Entity, target: Entity) -> bool {
    if let Some(implant) = world.get::<Implant>(target) {
        return implant.host == Some(mob);
    }
    if world.get::<HeldBy>(target).is_some_and(|h| h.0 == mob) {
        return true;
    }
    let near = at(world, mob)
        .zip(at(world, target))
        .is_some_and(|(a, b)| in_reach(a, b));
    let powered = world.get::<PrisonerConsole>(target).is_none()
        || world.get::<PowerUser>(target).is_some_and(|p| p.powered);
    near && powered
}

/// Armoury access on the card held or worn (Luna `allowed`).
fn armoury(world: &World, mob: Entity) -> bool {
    let card = |item: Option<Entity>| item.and_then(|i| world.get::<crate::access::IdCard>(i));
    let held = card(world.get::<Hands>(mob).and_then(Hands::active_item));
    let worn = card(world.get::<Equipment>(mob).and_then(|e| e.get(Slot::Id)));
    let need = ["armory".to_owned()];
    crate::access::Shown { held, worn }.opens(&need)
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
            if !can_work(world, mob, target) {
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
        Line::Freq(step) => {
            if let Some(imp) = pad_implant(world, target)
                && let Some(mut implant) = world.get_mut::<Implant>(imp)
                && let Kind::Tracking { frequency, .. } = &mut implant.kind
            {
                let f = (i32::from(*frequency) + i32::from(step)).clamp(
                    i32::from(crate::devices::FREQ_MIN),
                    i32::from(crate::devices::FREQ_MAX),
                );
                *frequency = if f % 2 == 0 { f + 1 } else { f } as u16;
            }
        }
        Line::Id(step) => {
            if let Some(imp) = pad_implant(world, target)
                && let Some(mut implant) = world.get_mut::<Implant>(imp)
                && let Kind::Tracking { id, .. } = &mut implant.kind
            {
                *id = (i16::from(*id) + i16::from(step)).clamp(1, 100) as u8;
            }
        }
        Line::Emote(i) => {
            let chosen = EMOTE_CHOICES[i].to_owned();
            if let Some(mut implant) = world.get_mut::<Implant>(target) {
                match &mut implant.kind {
                    Kind::Freedom { emote, .. } | Kind::Compressed { emote, .. } => {
                        *emote = Some(chosen.clone());
                    }
                    _ => return,
                }
            }
            world.entity_mut(mob).remove::<PanelViewer>();
            if let Some(mut out) = world.get_mut::<PickListOut>(mob) {
                out.0 = None;
            }
            tell(world, mob, LocMsg::new("implant-emote-set").arg("value", chosen));
        }
        Line::Unlock | Line::Lock => {
            if armoury(world, mob) {
                if let Some(mut c) = world.get_mut::<PrisonerConsole>(target) {
                    c.unlocked = line == Line::Unlock;
                }
            } else {
                tell(world, mob, LocMsg::new("prisoner-denied"));
            }
        }
        Line::Inject(imp, n) => {
            if world.get::<PrisonerConsole>(target).is_some_and(|c| c.unlocked) {
                activate_chem(world, imp, f32::from(n));
            }
        }
    }
}

fn pad_implant(world: &World, pad: Entity) -> Option<Entity> {
    let case = world.get::<ImplantPad>(pad)?.0?;
    world.get::<ImplantCase>(case)?.0
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

// --- The console ---

pub fn spawn_console(world: &mut World, pos: TilePos, id: NetId) -> Entity {
    world
        .spawn((
            GridPos(pos),
            id,
            Dense,
            Appearance::new(CONSOLE, ""),
            PrisonerConsole::default(),
            PowerUser::new(Channel::Equipment, 100.0),
        ))
        .id()
}

/// A click on the console opens its window.
pub fn click_console(world: &mut World, mob: Entity, console: Entity) {
    if world.get::<PowerUser>(console).is_some_and(|p| p.powered) {
        world.entity_mut(mob).insert(PanelViewer(console));
    }
}

// --- What implants do ---

/// A chemical implant injects into its host (Luna `chem/activate`).
fn activate_chem(world: &mut World, imp: Entity, amount: f32) {
    let Some(host) = world.get::<Implant>(imp).and_then(|i| i.host) else {
        return;
    };
    if amount <= 0.0 {
        return;
    }
    crate::chemistry::transfer(world, imp, host, amount);
    tell(world, host, LocMsg::new("implant-beep"));
    if world.get::<Reagents>(imp).is_none_or(|r| r.total() <= 0.0) {
        tell(world, host, LocMsg::new("implant-click"));
        world.entity_mut(imp).despawn();
    }
}

/// An EMP reaching a chemical implant (Luna `chem/emp_act`).
pub fn emp(world: &mut World, at_: TilePos, heavy: i32, light: i32) {
    let mut q = world.query::<(Entity, &Implant)>();
    let chems: Vec<(Entity, Entity)> = q
        .iter(world)
        .filter(|(_, i)| i.kind == Kind::Chem)
        .filter_map(|(e, i)| Some((e, i.host?)))
        .collect();
    for (imp, host) in chems {
        let Some(here) = at(world, host) else {
            continue;
        };
        if here.z != at_.z {
            continue;
        }
        let d = (here.x - at_.x).abs().max((here.y - at_.y).abs());
        let dose = if d <= heavy {
            20.0
        } else if d <= light {
            5.0
        } else {
            continue;
        };
        if world.resource_mut::<GameRng>().prob(60.0) {
            activate_chem(world, imp, dose);
        }
    }
}

/// Implants answering what their hosts do and hear, and their hosts dying.
pub fn run_implants(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    // Implanting under way.
    let mut q = world.query::<(Entity, &Implanting)>();
    let jobs: Vec<(Entity, Implanting)> = q.iter(world).map(|(e, j)| (e, *j)).collect();
    for (mob, job) in jobs {
        let stayed = at(world, mob) == Some(job.from)
            && at(world, job.target) == Some(job.target_from)
            && world.get::<HeldBy>(job.implanter).is_some_and(|h| h.0 == mob);
        if !stayed {
            world.entity_mut(mob).remove::<Implanting>();
            continue;
        }
        if now >= job.done_ds {
            world.entity_mut(mob).remove::<Implanting>();
            finish_implanting(world, mob, job);
        }
    }
    let heard = std::mem::take(&mut *world.get_resource_or_init::<Heard>());
    for (host, text) in heard.phrases {
        let Some(imp) = world.get::<ChoosingPhrase>(host).map(|c| c.0) else {
            continue;
        };
        world.entity_mut(host).remove::<ChoosingPhrase>();
        if let Some(mut implant) = world.get_mut::<Implant>(imp)
            && let Kind::Explosive { phrase } = &mut implant.kind
        {
            *phrase = Some(text.to_lowercase());
            tell(world, host, LocMsg::new("implant-phrase-set").arg("value", text));
        }
    }
    for (host, act) in heard.emoted {
        for imp in implants_in(world, host) {
            emoted(world, host, imp, &act);
        }
    }
    for (_, from, text) in heard.said {
        let text = text.to_lowercase();
        let mut q = world.query::<(Entity, &Implant)>();
        let bombs: Vec<(Entity, Entity, String)> = q
            .iter(world)
            .filter_map(|(e, i)| match &i.kind {
                Kind::Explosive { phrase: Some(p) } => Some((e, i.host?, p.clone())),
                _ => None,
            })
            .collect();
        for (imp, host, phrase) in bombs {
            let near = at(world, host)
                .is_some_and(|p| crate::speech::within_range(from, p, crate::speech::HEARING_RANGE));
            if near && !phrase.is_empty() && text.contains(&phrase) {
                blow_up(world, imp, host);
            }
        }
    }
    deaths(world, now);
}

fn emoted(world: &mut World, host: Entity, imp: Entity, act: &str) {
    let Some(kind) = world.get::<Implant>(imp).map(|i| i.kind.clone()) else {
        return;
    };
    match kind {
        Kind::Freedom { uses, emote: Some(e) } if e == act && uses > 0 => {
            if let Some(mut implant) = world.get_mut::<Implant>(imp)
                && let Kind::Freedom { uses, .. } = &mut implant.kind
            {
                *uses -= 1;
            }
            tell(world, host, LocMsg::new("implant-faint-click"));
            crate::security::take_off(world, host);
            if let Some(mut blood) = world.get_mut::<Reagents>(host) {
                blood.add("synaptizine", FREEDOM_DOSE);
            }
        }
        Kind::Compressed {
            emote: Some(e),
            scanned: Some(item),
        } if e == act => {
            if let Some(tile) = at(world, host) {
                let name = sprite(world, item).unwrap_or_default();
                world.entity_mut(item).remove::<InStore>().insert(GridPos(tile));
                tell(world, host, LocMsg::new("implant-uncompress").arg("item", name));
            }
            world.entity_mut(imp).despawn();
        }
        _ => {}
    }
}

/// Luna `explosive/hear`: the host dies and it goes up.
fn blow_up(world: &mut World, imp: Entity, host: Entity) {
    let Some(tile) = at(world, host) else {
        return;
    };
    world.entity_mut(imp).despawn();
    if let Some(mut vitals) = world.get_mut::<Vitals>(host)
        && vitals.stat != MobStat::Ghost
    {
        vitals.stat = MobStat::Dead;
    }
    let (d, h, l, f) = BLAST;
    crate::explosions::explode(world, tile, d, h, l, f);
    if let Some(mut atmos) = world.get_resource_mut::<crate::atmos::StationAtmos>() {
        atmos.0.expose(tile, crate::igniters::SPARK_TEMPERATURE);
    }
}

/// Hosts dying: a chemical implant empties into them (Luna's `deathgasp` trigger) and a death
/// alarm calls out.
fn deaths(world: &mut World, now: f32) {
    let mut q = world.query::<(Entity, &Implant)>();
    let hosted: Vec<(Entity, Entity, Kind)> = q
        .iter(world)
        .filter_map(|(e, i)| Some((e, i.host?, i.kind.clone())))
        .collect();
    for (imp, host, kind) in hosted {
        let dead = world
            .get::<Vitals>(host)
            .is_some_and(|v| matches!(v.stat, MobStat::Dead | MobStat::Ghost));
        if !dead {
            if world.get::<DiedAt>(host).is_some() {
                world.entity_mut(host).remove::<DiedAt>();
            }
            continue;
        }
        let newly = world.get::<DiedAt>(host).is_none();
        if newly {
            world.entity_mut(host).insert(DiedAt(now));
        }
        match kind {
            Kind::Chem if newly => {
                let all = world.get::<Reagents>(imp).map_or(0.0, Reagents::total);
                activate_chem(world, imp, all);
            }
            Kind::DeathAlarm { fired: false } => {
                if let Some(mut implant) = world.get_mut::<Implant>(imp) {
                    implant.kind = Kind::DeathAlarm { fired: true };
                }
                let area = at(world, host).map(|p| area_of(world, p)).unwrap_or_default();
                let name = name_of(world, host);
                alert(
                    world,
                    LocMsg::new("implant-death-alarm")
                        .arg("name", name)
                        .arg("area", area),
                );
            }
            _ => {}
        }
    }
}

/// A message on the common channel, to everyone near a radio listening to it (Luna `radioalert`).
fn alert(world: &mut World, msg: LocMsg) {
    let mut radios = world.query::<(Entity, &Radio)>();
    let listening: Vec<(Entity, i32)> = radios
        .iter(world)
        .filter(|(_, r)| r.listening && r.frequency == COMMON_FREQUENCY)
        .map(|(e, r)| (e, r.range))
        .collect();
    let speakers: Vec<(TilePos, i32)> = listening
        .into_iter()
        .filter_map(|(e, range)| {
            let p = at(world, e)
                .or_else(|| world.get::<HeldBy>(e).and_then(|h| at(world, h.0)))
                .or_else(|| world.get::<WornBy>(e).and_then(|w| at(world, w.0)))?;
            Some((p, range))
        })
        .collect();
    let msg = msg.arg("frequency", format_frequency(COMMON_FREQUENCY));
    let mut mobs = world.query_filtered::<(Entity, &GridPos), With<Outbox>>();
    let hearers: Vec<Entity> = mobs
        .iter(world)
        .filter(|(_, p)| {
            speakers
                .iter()
                .any(|(s, r)| crate::speech::within_range(*s, p.0, *r))
        })
        .map(|(e, _)| e)
        .collect();
    for mob in hearers {
        tell(world, mob, msg.clone());
    }
}

// --- Taking them out ---

/// Implant removal surgery: the implant in this part comes out onto the floor. Whether there was one.
pub fn extract(world: &mut World, host: Entity, zone: Zone) -> Option<Entity> {
    let imp = implants_in(world, host)
        .into_iter()
        .find(|e| world.get::<Implant>(*e).is_some_and(|i| i.zone == Some(zone)))?;
    let tile = at(world, host)?;
    if let Some(mut implant) = world.get_mut::<Implant>(imp) {
        implant.host = None;
        implant.zone = None;
    }
    world.entity_mut(imp).insert(GridPos(tile));
    if world.get::<ChoosingPhrase>(host).is_some_and(|c| c.0 == imp) {
        world.entity_mut(host).remove::<ChoosingPhrase>();
    }
    Some(imp)
}

/// Tracking implants as teleporter targets: each one in a living host, or one dead under ten
/// minutes, named after the host (Luna's `teleporter` list).
pub fn tracking_targets(world: &mut World) -> Vec<(String, Entity)> {
    let now = world.resource::<WorldTime>().deciseconds();
    let mut q = world.query::<(Entity, &Implant)>();
    let mut all: Vec<(Entity, Entity)> = q
        .iter(world)
        .filter(|(_, i)| matches!(i.kind, Kind::Tracking { .. }))
        .filter_map(|(e, i)| Some((e, i.host?)))
        .collect();
    all.sort();
    all.into_iter()
        .filter(|(_, host)| {
            world
                .get::<DiedAt>(*host)
                .is_none_or(|d| now - d.0 < TRACKING_DEAD_DS)
        })
        .map(|(imp, host)| (name_of(world, host), imp))
        .collect()
}

/// Where a teleporter locked onto an implant sends you.
pub fn tracking_place(world: &World, imp: Entity) -> Option<TilePos> {
    where_is(world, imp)
}
