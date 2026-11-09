//! Surgery: procedures done step by step on someone lying down, and the operating table
//! (Luna `modules/surgery`, `OpTable.dm`). Spec: docs/reference/surgery.md.
//!
//! - **Drapes** used on a person lying down, on help intent, over an uncovered body part,
//!   offer the procedures that part allows; picking one starts it there.
//! - **Each step** wants a tool (or an empty hand), aimed at that part, on help intent. It
//!   takes a few seconds; standing still the whole time, the step works by its tool's chance
//!   times the place's (an operating table is best, the floor worst). A failed step usually
//!   has to be done again.
//! - **The operating table** takes someone put on it with a grab, lying down.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{
    Appearance, Dense, DisplayName, GridPos, HumanLook, Intent, MobStat, NetId, ObjectSpec, PickListView,
    TilePos,
};

use crate::actions::{Outbox, in_reach};
use crate::closets::InCloset;
use crate::combat::{AimZone, CombatIntent, PendingAttacks, VisibleEvents};
use crate::equipment::{Equipment, Wearable};
use crate::filing::{PendingPicks, PickListOut};
use crate::hands::{Hands, HeldBy, Item};
use crate::health::{Body, Vitals, Zone};
use crate::rng::GameRng;
use crate::time::WorldTime;
use crate::wires::PanelViewer;

pub const TABLE: &str = "operating-table";
pub const DRAPES: &str = "surgical-drapes";
pub const APPENDIX: &str = "appendix";
pub const BRAIN: &str = "brain";

/// A procedure, as Luna's `/datum/surgery` types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Procedure {
    Appendectomy,
    BrainRemoval,
    EyeSurgery,
    PlasticSurgery,
    GenderReassignment,
    CavityImplant,
}

impl Procedure {
    pub const ALL: [Procedure; 6] = [
        Procedure::Appendectomy,
        Procedure::BrainRemoval,
        Procedure::EyeSurgery,
        Procedure::PlasticSurgery,
        Procedure::GenderReassignment,
        Procedure::CavityImplant,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Appendectomy => "appendectomy",
            Self::BrainRemoval => "brain-removal",
            Self::EyeSurgery => "eye-surgery",
            Self::PlasticSurgery => "plastic-surgery",
            Self::GenderReassignment => "gender-reassignment",
            Self::CavityImplant => "cavity-implant",
        }
    }

    /// Where it can be done. Luna's "eyes" are part of the head here.
    pub fn allows(self, zone: Zone) -> bool {
        match self {
            Self::Appendectomy | Self::GenderReassignment => zone == Zone::Groin,
            Self::BrainRemoval | Self::EyeSurgery | Self::PlasticSurgery => zone == Zone::Head,
            Self::CavityImplant => true,
        }
    }

    pub fn steps(self) -> &'static [Step] {
        use Step::*;
        match self {
            Self::Appendectomy => &[Incise, Clamp, Retract, Incise, ExtractAppendix, Close],
            Self::BrainRemoval => &[Incise, Clamp, Retract, Saw, ExtractBrain],
            Self::EyeSurgery => &[Incise, Retract, Clamp, FixEyes, Close],
            Self::PlasticSurgery => &[Incise, Retract, ReshapeFace, Close],
            Self::GenderReassignment => &[Incise, Clamp, ReshapeGenitals, Close],
            Self::CavityImplant => &[Incise, Clamp, Retract, Incise, HandleCavity, Close],
        }
    }
}

/// One step of a procedure (Luna's `/datum/surgery_step` types).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Incise,
    Clamp,
    Retract,
    Close,
    Saw,
    ExtractAppendix,
    ExtractBrain,
    FixEyes,
    ReshapeFace,
    ReshapeGenitals,
    HandleCavity,
}

/// What a step can be done with.
enum Takes {
    /// These tools, each with its chance in percent.
    Tools(&'static [(&'static str, f32)]),
    /// An empty hand.
    Hand,
    /// An empty hand or anything held.
    Anything,
}

impl Step {
    pub fn id(self) -> &'static str {
        match self {
            Self::Incise => "incise",
            Self::Clamp => "clamp",
            Self::Retract => "retract",
            Self::Close => "close",
            Self::Saw => "saw",
            Self::ExtractAppendix => "extract-appendix",
            Self::ExtractBrain => "extract-brain",
            Self::FixEyes => "fix-eyes",
            Self::ReshapeFace => "reshape-face",
            Self::ReshapeGenitals => "reshape-genitals",
            Self::HandleCavity => "cavity",
        }
    }

    /// How long it takes, ds.
    pub fn time(self) -> f32 {
        match self {
            Self::Incise => 24.0,
            Self::Clamp => 48.0,
            Self::Retract | Self::Close | Self::HandleCavity => 32.0,
            _ => 64.0,
        }
    }

    fn takes(self) -> Takes {
        match self {
            Self::Incise => Takes::Tools(&[("scalpel", 100.0), ("knife", 85.0), ("shard", 65.0)]),
            Self::Clamp => Takes::Tools(&[("hemostat", 100.0), ("wirecutters", 70.0), ("cable-coil", 15.0)]),
            Self::Retract => {
                Takes::Tools(&[("retractor", 100.0), ("screwdriver", 65.0), ("wirecutters", 35.0)])
            }
            // The welding tool only lit (checked when it's used).
            Self::Close => Takes::Tools(&[("cautery", 100.0), ("welder", 70.0)]),
            Self::Saw => Takes::Tools(&[("circular-saw", 100.0), ("hatchet", 55.0)]),
            Self::ExtractAppendix => Takes::Hand,
            Self::ExtractBrain => Takes::Tools(&[("hemostat", 100.0), ("crowbar", 55.0)]),
            Self::FixEyes => Takes::Tools(&[("hemostat", 100.0), ("screwdriver", 65.0), ("pen", 45.0)]),
            Self::ReshapeFace => Takes::Tools(&[("scalpel", 100.0), ("knife", 50.0), ("wirecutters", 35.0)]),
            Self::ReshapeGenitals => {
                Takes::Tools(&[("scalpel", 100.0), ("hatchet", 50.0), ("wirecutters", 35.0)])
            }
            Self::HandleCavity => Takes::Anything,
        }
    }

    /// The chance a held tool (by prototype, `None` for an empty hand) does this step, if it
    /// can at all.
    pub fn chance(self, tool: Option<&str>) -> Option<f32> {
        match (self.takes(), tool) {
            (Takes::Hand, None) | (Takes::Anything, _) => Some(100.0),
            (Takes::Tools(list), Some(tool)) => list.iter().find(|(t, _)| *t == tool).map(|(_, c)| *c),
            _ => None,
        }
    }
}

/// A procedure under way on someone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Operation {
    pub procedure: Procedure,
    pub zone: Zone,
    /// The next step, by index.
    pub step: usize,
}

/// The procedures under way on someone.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct Surgeries(pub Vec<Operation>);

/// Things put inside someone's body parts by cavity implant.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct Cavities(pub Vec<(Zone, Entity)>);

/// Organs taken out of someone.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Removed {
    pub appendix: bool,
    pub brain: bool,
}

/// An operating table.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OperatingTable;

/// Someone laid on an operating table: lying down until they leave it.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct OnTable(pub TilePos);

/// The procedures drapes offered, waiting for a pick.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct DrapeChoice {
    pub patient: Entity,
    pub zone: Zone,
    pub offered: Vec<Procedure>,
}

/// A step being done: finished at `until`, if surgeon and patient stay put and the surgeon
/// keeps the same tool in hand.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Operating {
    pub patient: Entity,
    pub zone: Zone,
    pub tool: Option<Entity>,
    pub chance: f32,
    pub until: f32,
    pub at: TilePos,
    pub patient_at: TilePos,
}

pub fn spawn_table(world: &mut World, pos: TilePos, id: NetId) -> Entity {
    world
        .spawn((
            GridPos(pos),
            id,
            Dense,
            Appearance::new(TABLE, ""),
            OperatingTable,
        ))
        .id()
}

fn now(world: &World) -> f32 {
    world.get_resource::<WorldTime>().map_or(0.0, |t| t.deciseconds())
}

fn at(world: &World, e: Entity) -> Option<TilePos> {
    world.get::<GridPos>(e).map(|p| p.0)
}

fn name_of(world: &World, e: Entity) -> String {
    world
        .get::<DisplayName>(e)
        .map_or_else(String::new, |n| n.0.clone())
}

fn sprite(world: &World, e: Entity) -> Option<String> {
    world.get::<Appearance>(e).map(|a| a.sprite.clone())
}

fn tell(world: &mut World, mob: Entity, msg: LocMsg) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(msg);
    }
}

/// Everyone around sees what the surgeon does to the patient.
fn seen(world: &mut World, id: &str, surgeon: Entity, patient: Entity, zone: Zone) {
    let Some(pos) = at(world, surgeon) else {
        return;
    };
    let msg = LocMsg::new(id)
        .arg("name", name_of(world, surgeon))
        .arg("target", name_of(world, patient))
        .arg("zone", zone.noun_id());
    world.resource_mut::<VisibleEvents>().0.push((pos, msg, None));
}

/// Lying down: resting, knocked down, out cold or dead.
pub fn lying(world: &World, mob: Entity) -> bool {
    world.get::<Appearance>(mob).is_some_and(|a| !a.state.is_empty())
}

/// Whether a body part is bare: no worn clothing covers it (Luna `get_location_accessible`).
pub fn accessible(world: &World, patient: Entity, zone: Zone) -> bool {
    let covered = world.get::<Equipment>(patient).map_or(0, |e| {
        e.0.values()
            .filter_map(|item| world.get::<Wearable>(*item))
            .fold(0u16, |bits, w| bits | w.covers)
    });
    covered & zone.cover_bit() == 0
}

/// How much where the patient lies helps (Luna `get_location_modifier`).
pub fn place_modifier(world: &mut World, patient: Entity) -> f32 {
    let Some(pos) = at(world, patient) else {
        return 0.4;
    };
    let mut q = world.query::<(&GridPos, &Appearance)>();
    let here: Vec<String> = q
        .iter(world)
        .filter(|(p, _)| p.0 == pos)
        .map(|(_, a)| a.sprite.clone())
        .collect();
    let has = |names: &[&str]| here.iter().any(|s| names.contains(&s.as_str()));
    if has(&[TABLE]) {
        1.0
    } else if has(&["table", "table-reinforced", "table-wood"]) {
        0.8
    } else if has(&["roller-bed"]) {
        0.9
    } else if has(&["bed"]) {
        0.6
    } else {
        0.4
    }
}

/// The procedures drapes over this part offer.
pub fn offered(world: &World, patient: Entity, zone: Zone) -> Vec<Procedure> {
    let under_way = world.get::<Surgeries>(patient).cloned().unwrap_or_default();
    Procedure::ALL
        .into_iter()
        .filter(|p| p.allows(zone))
        .filter(|p| !under_way.0.iter().any(|o| o.procedure == *p))
        .collect()
}

fn aim(world: &World, mob: Entity) -> Zone {
    world.get::<AimZone>(mob).copied().unwrap_or_default().0
}

fn helping(world: &World, mob: Entity) -> bool {
    world.get::<CombatIntent>(mob).is_none_or(|i| i.0 == Intent::Help)
}

/// Take surgery out of this tick's clicks on people: drapes starting a procedure, and tools
/// (or empty hands) doing its next step. Anything else is left to land as a blow.
pub fn take_surgery_clicks(world: &mut World) {
    let pending = std::mem::take(&mut world.resource_mut::<PendingAttacks>().0);
    let mut left = Vec::new();
    for (surgeon, patient) in pending {
        if !surgery_click(world, surgeon, patient) {
            left.push((surgeon, patient));
        }
    }
    world.resource_mut::<PendingAttacks>().0.extend(left);
}

/// One click on a person: whether it was surgery.
fn surgery_click(world: &mut World, surgeon: Entity, patient: Entity) -> bool {
    if surgeon == patient
        || world.get::<Body>(patient).is_none()
        || !helping(world, surgeon)
        || !lying(world, patient)
        || world.get::<Operating>(surgeon).is_some()
    {
        return false;
    }
    let near = at(world, surgeon)
        .zip(at(world, patient))
        .is_some_and(|(a, b)| in_reach(a, b));
    if !near {
        return false;
    }
    let held = world.get::<Hands>(surgeon).and_then(Hands::active_item);
    let tool = held.and_then(|h| sprite(world, h));
    let zone = aim(world, surgeon);
    if tool.as_deref() == Some(DRAPES) {
        return drape(world, surgeon, patient, zone);
    }
    let Some(op) = world
        .get::<Surgeries>(patient)
        .and_then(|s| s.0.iter().find(|o| o.zone == zone).copied())
    else {
        return false;
    };
    let Some(step) = op.procedure.steps().get(op.step).copied() else {
        return false;
    };
    let Some(mut chance) = step.chance(tool.as_deref()) else {
        return false;
    };
    if tool.as_deref() == Some("welder")
        && !held
            .and_then(|h| world.get::<crate::welding::Welder>(h))
            .is_some_and(|w| w.lit)
    {
        return false;
    }
    if !accessible(world, patient, zone) {
        tell(
            world,
            surgeon,
            LocMsg::new("surgery-expose").arg("zone", zone.noun_id()),
        );
        return true;
    }
    chance *= place_modifier(world, patient);
    let (Some(here), Some(there)) = (at(world, surgeon), at(world, patient)) else {
        return true;
    };
    let until = now(world) + step.time();
    world.entity_mut(surgeon).insert(Operating {
        patient,
        zone,
        tool: held,
        chance,
        until,
        at: here,
        patient_at: there,
    });
    let begins = match step {
        Step::ExtractAppendix if world.get::<Removed>(patient).is_some_and(|r| r.appendix) => {
            "surgery-begin-look-appendix".to_owned()
        }
        Step::ExtractBrain if world.get::<Removed>(patient).is_some_and(|r| r.brain) => {
            "surgery-begin-look-brain".to_owned()
        }
        Step::ReshapeGenitals if world.get::<HumanLook>(patient).is_some_and(|l| l.female) => {
            "surgery-begin-reshape-genitals-male".to_owned()
        }
        Step::HandleCavity if held.is_none() => "surgery-begin-cavity-check".to_owned(),
        _ => format!("surgery-begin-{}", step.id()),
    };
    seen(world, &begins, surgeon, patient, zone);
    true
}

/// Drapes over a bare part: the procedures it allows, to pick from.
fn drape(world: &mut World, surgeon: Entity, patient: Entity, zone: Zone) -> bool {
    let offered = offered(world, patient, zone);
    if offered.is_empty() {
        return false;
    }
    if !accessible(world, patient, zone) {
        tell(
            world,
            surgeon,
            LocMsg::new("surgery-expose").arg("zone", zone.noun_id()),
        );
        return true;
    }
    world.entity_mut(surgeon).insert((
        DrapeChoice {
            patient,
            zone,
            offered,
        },
        PanelViewer(patient),
    ));
    true
}

/// A procedure picked from the drapes' list.
pub fn run_picks(world: &mut World) {
    let mut q = world.query::<(Entity, &mut PendingPicks, &DrapeChoice)>();
    let picks: Vec<(Entity, Vec<(Entity, usize)>, DrapeChoice)> = q
        .iter_mut(world)
        .filter(|(_, p, _)| !p.0.is_empty())
        .map(|(mob, mut p, c)| (mob, std::mem::take(&mut p.0), c.clone()))
        .collect();
    for (surgeon, list, choice) in picks {
        let mut others = Vec::new();
        for (target, line) in list {
            if target != choice.patient {
                others.push((target, line));
                continue;
            }
            world.entity_mut(surgeon).remove::<(DrapeChoice, PanelViewer)>();
            if let Some(mut out) = world.get_mut::<PickListOut>(surgeon) {
                out.0 = None;
            }
            let Some(procedure) = choice.offered.get(line).copied() else {
                continue;
            };
            // Still free to start, and still bare.
            if !offered(world, choice.patient, choice.zone).contains(&procedure)
                || !accessible(world, choice.patient, choice.zone)
            {
                continue;
            }
            let mut surgeries = world
                .get::<Surgeries>(choice.patient)
                .cloned()
                .unwrap_or_default();
            surgeries.0.push(Operation {
                procedure,
                zone: choice.zone,
                step: 0,
            });
            world.entity_mut(choice.patient).insert(surgeries);
            let Some(pos) = at(world, surgeon) else {
                continue;
            };
            let msg = LocMsg::new("surgery-draped")
                .arg("name", name_of(world, surgeon))
                .arg("target", name_of(world, choice.patient))
                .arg("zone", choice.zone.noun_id())
                .msg("procedure", format!("surgery-{}", procedure.id()));
            world.resource_mut::<VisibleEvents>().0.push((pos, msg, None));
        }
        if !others.is_empty()
            && let Some(mut pending) = world.get_mut::<PendingPicks>(surgeon)
        {
            pending.0.extend(others);
        }
    }
}

/// The drapes' list, kept open while the patient is in reach and lying down.
pub fn update_views(world: &mut World) {
    let mut q = world.query::<(Entity, &DrapeChoice)>();
    let all: Vec<(Entity, DrapeChoice)> = q.iter(world).map(|(m, c)| (m, c.clone())).collect();
    for (surgeon, choice) in all {
        let open = at(world, surgeon)
            .zip(at(world, choice.patient))
            .is_some_and(|(a, b)| in_reach(a, b))
            && lying(world, choice.patient)
            && world
                .get::<PanelViewer>(surgeon)
                .is_some_and(|v| v.0 == choice.patient);
        let view = match (open, world.get::<NetId>(choice.patient).copied()) {
            (true, Some(id)) => Some(PickListView {
                target: id,
                title: DRAPES.to_owned(),
                rows: choice
                    .offered
                    .iter()
                    .map(|p| format!("msg:surgery-{}", p.id()))
                    .collect(),
            }),
            _ => {
                world.entity_mut(surgeon).remove::<DrapeChoice>();
                if world
                    .get::<PanelViewer>(surgeon)
                    .is_some_and(|v| v.0 == choice.patient)
                {
                    world.entity_mut(surgeon).remove::<PanelViewer>();
                }
                None
            }
        };
        if let Some(mut out) = world.get_mut::<PickListOut>(surgeon)
            && out.0 != view
        {
            out.0 = view;
        }
    }
}

/// Steps that are due: done if everyone stayed put, then a roll for success.
pub fn run_operations(world: &mut World) {
    let now = now(world);
    let mut q = world.query::<(Entity, &Operating)>();
    let due: Vec<(Entity, Operating)> = q
        .iter(world)
        .filter(|(_, o)| now >= o.until)
        .map(|(e, o)| (e, *o))
        .collect();
    for (surgeon, work) in due {
        world.entity_mut(surgeon).remove::<Operating>();
        let held = world.get::<Hands>(surgeon).and_then(Hands::active_item);
        let awake = world
            .get::<Vitals>(surgeon)
            .is_none_or(|v| v.stat == MobStat::Conscious);
        if !awake
            || at(world, surgeon) != Some(work.at)
            || at(world, work.patient) != Some(work.patient_at)
            || held != work.tool
        {
            continue;
        }
        let Some(op) = world
            .get::<Surgeries>(work.patient)
            .and_then(|s| s.0.iter().find(|o| o.zone == work.zone).copied())
        else {
            continue;
        };
        let Some(step) = op.procedure.steps().get(op.step).copied() else {
            continue;
        };
        let worked = world.resource_mut::<GameRng>().prob(work.chance);
        let advance = if worked {
            succeed(world, surgeon, work.patient, work.zone, step, held)
        } else {
            fail(world, surgeon, work.patient, work.zone, step)
        };
        if advance {
            advance_op(world, work.patient, work.zone);
        }
    }
}

fn advance_op(world: &mut World, patient: Entity, zone: Zone) {
    let Some(mut s) = world.get_mut::<Surgeries>(patient) else {
        return;
    };
    if let Some(i) = s.0.iter().position(|o| o.zone == zone) {
        s.0[i].step += 1;
        if s.0[i].step >= s.0[i].procedure.steps().len() {
            s.0.remove(i);
        }
    }
}

fn spawn_organ(world: &mut World, id: &str, patient: Entity) {
    if let Some(pos) = at(world, patient) {
        let _ = crate::objects::spawn_object(world, &ObjectSpec::new(id), pos);
    }
}

/// The step worked: whether the procedure moves on.
fn succeed(
    world: &mut World,
    surgeon: Entity,
    patient: Entity,
    zone: Zone,
    step: Step,
    held: Option<Entity>,
) -> bool {
    let removed = world.get::<Removed>(patient).copied().unwrap_or_default();
    match step {
        Step::ExtractAppendix => {
            if removed.appendix {
                seen(world, "surgery-no-appendix", surgeon, patient, zone);
            } else {
                spawn_organ(world, APPENDIX, patient);
                world.entity_mut(patient).insert(Removed {
                    appendix: true,
                    ..removed
                });
                seen(world, "surgery-done-extract-appendix", surgeon, patient, zone);
            }
        }
        Step::ExtractBrain => {
            if removed.brain {
                seen(world, "surgery-no-brain", surgeon, patient, zone);
            } else {
                spawn_organ(world, BRAIN, patient);
                world.entity_mut(patient).insert(Removed {
                    brain: true,
                    ..removed
                });
                // Nobody lives without a brain.
                if let Some(mut vitals) = world.get_mut::<Vitals>(patient)
                    && vitals.stat != MobStat::Ghost
                {
                    vitals.stat = MobStat::Dead;
                }
                seen(world, "surgery-done-extract-brain", surgeon, patient, zone);
            }
        }
        Step::FixEyes => {
            let until = now(world) + 350.0;
            world
                .entity_mut(patient)
                .remove::<(crate::eyes::EyeBlind, crate::eyes::Nearsighted)>()
                .insert(crate::eyes::EyeBlurry(until));
            seen(world, "surgery-done-fix-eyes", surgeon, patient, zone);
        }
        Step::ReshapeFace => {
            let old = name_of(world, patient);
            let seed = world.resource_mut::<GameRng>().range(0, i32::MAX) as u64;
            let mut new = crate::names::default_name(seed);
            if new == old {
                new = crate::names::default_name(seed + 1);
            }
            world.entity_mut(patient).insert(DisplayName(new.clone()));
            if let Some(pos) = at(world, surgeon) {
                let msg = LocMsg::new("surgery-done-reshape-face")
                    .arg("name", name_of(world, surgeon))
                    .arg("target", old)
                    .arg("new", new);
                world.resource_mut::<VisibleEvents>().0.push((pos, msg, None));
            }
        }
        Step::ReshapeGenitals => {
            let mut look = world.get::<HumanLook>(patient).copied().unwrap_or_default();
            look.female = !look.female;
            world.entity_mut(patient).insert(look);
            let id = if look.female {
                "surgery-done-reshape-genitals-female"
            } else {
                "surgery-done-reshape-genitals-male"
            };
            seen(world, id, surgeon, patient, zone);
        }
        Step::HandleCavity => return handle_cavity(world, surgeon, patient, zone, held),
        _ => seen(
            world,
            &format!("surgery-done-{}", step.id()),
            surgeon,
            patient,
            zone,
        ),
    }
    true
}

/// The biggest thing a part's cavity holds (Luna `w_class`).
fn cavity_room(zone: Zone) -> u8 {
    match zone {
        Zone::Head | Zone::LHand | Zone::RHand | Zone::LFoot | Zone::RFoot => 1,
        Zone::LArm | Zone::RArm | Zone::LLeg | Zone::RLeg => 2,
        Zone::Chest | Zone::Groin => 3,
    }
}

fn handle_cavity(
    world: &mut World,
    surgeon: Entity,
    patient: Entity,
    zone: Zone,
    held: Option<Entity>,
) -> bool {
    let mut cavities = world.get::<Cavities>(patient).cloned().unwrap_or_default();
    let inside = cavities.0.iter().position(|(z, _)| *z == zone);
    match held {
        Some(item) => {
            let size = world.get::<Item>(item).map_or(u8::MAX, |i| i.size);
            let organ = matches!(sprite(world, item).as_deref(), Some(APPENDIX | BRAIN));
            if inside.is_some() || size > cavity_room(zone) || organ {
                seen(world, "surgery-cavity-no-fit", surgeon, patient, zone);
                return false;
            }
            if let Some(mut hands) = world.get_mut::<Hands>(surgeon) {
                let active = hands.active;
                *hands.slot_mut(active) = None;
            }
            world
                .entity_mut(item)
                .remove::<HeldBy>()
                .insert(InCloset(patient));
            cavities.0.push((zone, item));
            world.entity_mut(patient).insert(cavities);
            seen(world, "surgery-cavity-stuffed", surgeon, patient, zone);
            true
        }
        None => match inside {
            Some(i) => {
                let (_, item) = cavities.0.remove(i);
                world.entity_mut(patient).insert(cavities);
                world
                    .entity_mut(item)
                    .remove::<InCloset>()
                    .insert(HeldBy(surgeon));
                if let Some(mut hands) = world.get_mut::<Hands>(surgeon) {
                    let active = hands.active;
                    *hands.slot_mut(active) = Some(item);
                }
                seen(world, "surgery-cavity-pulled", surgeon, patient, zone);
                true
            }
            None => {
                seen(world, "surgery-cavity-empty", surgeon, patient, zone);
                false
            }
        },
    }
}

/// The step went wrong: whether the procedure moves on anyway.
fn fail(world: &mut World, surgeon: Entity, patient: Entity, zone: Zone, step: Step) -> bool {
    match step {
        Step::FixEyes => {
            seen(world, "surgery-failed-fix-eyes", surgeon, patient, zone);
            if let Some(mut body) = world.get_mut::<Body>(patient) {
                body.take_damage(Zone::Head, 20.0, 0.0);
            }
            false
        }
        Step::ReshapeGenitals => {
            let female = world.resource_mut::<GameRng>().prob(50.0);
            let mut look = world.get::<HumanLook>(patient).copied().unwrap_or_default();
            look.female = female;
            world.entity_mut(patient).insert(look);
            seen(world, "surgery-failed-reshape-genitals", surgeon, patient, zone);
            true
        }
        _ => {
            seen(world, "surgery-failed", surgeon, patient, zone);
            false
        }
    }
}

/// A grab used on the operating table lays whoever it holds on it.
pub fn lay_on_table(world: &mut World, surgeon: Entity, victim: Entity, table: Entity) -> bool {
    let (Some(here), Some(spot)) = (at(world, surgeon), at(world, table)) else {
        return false;
    };
    if !in_reach(here, spot) {
        return false;
    }
    world
        .entity_mut(victim)
        .insert((GridPos(spot), crate::chairs::Resting, OnTable(spot)));
    let msg = LocMsg::new("surgery-laid-on-table")
        .arg("name", name_of(world, surgeon))
        .arg("target", name_of(world, victim));
    world.resource_mut::<VisibleEvents>().0.push((spot, msg, None));
    true
}

/// Off the table, they're on their feet again; the table shows whether someone's on it.
pub fn update_tables(world: &mut World) {
    let mut q = world.query::<(Entity, &GridPos, &OnTable)>();
    let off: Vec<Entity> = q
        .iter(world)
        .filter(|(_, p, t)| p.0 != t.0)
        .map(|(e, ..)| e)
        .collect();
    for mob in off {
        world
            .entity_mut(mob)
            .remove::<(OnTable, crate::chairs::Resting)>();
    }
    let mut patients = world.query::<(&GridPos, &OnTable, Option<&Vitals>)>();
    let lying: Vec<(TilePos, bool)> = patients
        .iter(world)
        .map(|(p, _, v)| (p.0, v.is_some_and(|v| v.stat == MobStat::Dead)))
        .collect();
    let mut tables = world.query_filtered::<(&GridPos, &mut Appearance), With<OperatingTable>>();
    for (pos, mut look) in tables.iter_mut(world) {
        let state = match lying.iter().find(|(p, _)| *p == pos.0) {
            Some((_, true)) => "table2-active-dead",
            Some((_, false)) => "table2-active",
            None => "",
        };
        if look.state != state {
            look.state = state.to_owned();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_take_their_tools_with_luna_odds() {
        assert_eq!(Step::Incise.chance(Some("scalpel")), Some(100.0));
        assert_eq!(Step::Incise.chance(Some("knife")), Some(85.0));
        assert_eq!(Step::Incise.chance(Some("crowbar")), None);
        assert_eq!(Step::Incise.chance(None), None);
        assert_eq!(Step::ExtractAppendix.chance(None), Some(100.0));
        assert_eq!(Step::ExtractAppendix.chance(Some("scalpel")), None);
        assert_eq!(Step::HandleCavity.chance(Some("pen")), Some(100.0));
        assert_eq!(Step::HandleCavity.chance(None), Some(100.0));
    }

    #[test]
    fn procedures_are_done_where_luna_does_them() {
        assert!(Procedure::Appendectomy.allows(Zone::Groin));
        assert!(!Procedure::Appendectomy.allows(Zone::Head));
        assert!(Procedure::BrainRemoval.allows(Zone::Head));
        assert!(Procedure::CavityImplant.allows(Zone::LFoot));
        assert_eq!(Procedure::BrainRemoval.steps().last(), Some(&Step::ExtractBrain));
    }
}
