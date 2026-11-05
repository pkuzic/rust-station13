//! Revolution and RP revolution: head revolutionaries turning the crew against the heads of staff
//! (Luna's `gamemodes/revolution`). Spec: `docs/reference/revolution.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{DisplayName, GridPos, MobStat, ObjectSpec, Slot, TilePos};

use crate::actions::Outbox;
use crate::combat::VisibleEvents;
use crate::equipment::Equipment;
use crate::health::Vitals;
use crate::jobs::HasJob;
use crate::movement::Mob;
use crate::objects::{PlayerControlled, spawn_object};
use crate::rng::GameRng;
use crate::round::{Phase, Round};
use crate::security::Cuffed;
use crate::storage::{InStorage, Storage};
use crate::time::WorldTime;
use crate::traitor::{WantsTraitor, put_in_hand};

pub const REVOLUTION: &str = "revolution";
pub const RP_REVOLUTION: &str = "rp-revolution";
/// How many lead it.
pub const HEAD_REVS: usize = 3;
/// How long someone can't be asked again, ds (Luna `rev_cooldown`).
pub const ASK_COOLDOWN_DS: f32 = 50.0;
/// When an RP revolution's leaders are named to everyone, ds (Luna's two timers, one after the other).
pub const REVEAL_DS: f32 = 66_000.0;
/// How far away someone can be asked (Luna `oview`).
const ASK_RANGE: i32 = 7;
/// Where a captured leader is held.
const BRIG_AREA: &str = "security-brig";

/// The heads of staff, by job id: what the revolution wants rid of (Luna `get_living_heads`).
pub const HEADS: [&str; 5] = ["captain", "hos", "hop", "chief-engineer", "research-director"];
/// Those who can't be turned: the heads and security (Luna `get_unconvertables`).
pub const UNCONVERTABLE: [&str; 7] = [
    "captain",
    "hos",
    "hop",
    "chief-engineer",
    "research-director",
    "security-officer",
    "forensic-technician",
];

/// One of the revolution's leaders.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeadRev;

/// Someone they've turned.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Revolutionary;

/// Who's asked someone to join, waiting for their answer, and when they can be asked again.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct RevAsk {
    pub from: Option<Entity>,
    pub again_at: f32,
}

/// How the revolution stands.
#[derive(Resource, Debug, Default, Clone, PartialEq)]
pub struct RevState {
    pub leaders: Vec<Entity>,
    pub heads: Vec<Entity>,
    pub converted: Vec<Entity>,
    /// Over: the revolution won, or the heads did.
    pub revs_won: Option<bool>,
    pub reveal_at: Option<f32>,
}

fn current_mode(world: &World) -> Option<&'static str> {
    match world.get_resource::<Round>()?.phase {
        Phase::Playing { mode } => Some(mode),
        _ => None,
    }
}

fn job_of(world: &World, mob: Entity) -> Option<String> {
    world.get::<HasJob>(mob).map(|j| j.0.clone())
}

fn tell(world: &mut World, mob: Entity, msg: LocMsg) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(msg);
    }
}

fn tell_everyone(world: &mut World, msg: &LocMsg) {
    let mut people = world.query::<&mut Outbox>();
    for mut outbox in people.iter_mut(world) {
        outbox.0.push(msg.clone());
    }
}

fn name_of(world: &World, mob: Entity) -> String {
    world
        .get::<DisplayName>(mob)
        .map_or_else(String::new, |name| name.0.clone())
}

fn is_dead(world: &World, mob: Entity) -> bool {
    world.get::<Vitals>(mob).is_some_and(|v| v.stat == MobStat::Dead)
}

/// A flash in their pack, in a hand, or at their feet (Luna `equip_revolutionary`).
fn give_flash(world: &mut World, mob: Entity) {
    let Some(at) = world.get::<GridPos>(mob).map(|p| p.0) else {
        return;
    };
    let Ok(flash) = spawn_object(world, &ObjectSpec::new(crate::flashes::FLASH), at) else {
        return;
    };
    let pack = world
        .get::<Equipment>(mob)
        .and_then(|e| e.get(Slot::Back))
        .filter(|pack| world.get::<Storage>(*pack).is_some());
    if let Some(pack) = pack {
        world
            .entity_mut(flash)
            .remove::<GridPos>()
            .insert(InStorage(pack));
        if let Some(mut storage) = world.get_mut::<Storage>(pack) {
            storage.items.push(flash);
        }
        return;
    }
    put_in_hand(world, mob, flash);
}

/// Choose the leaders and tell them who they're after (Luna's `post_setup`). False if there's no one
/// to lead it or no one to overthrow.
pub fn setup_round(world: &mut World, rp: bool) -> bool {
    let mut crew = world.query_filtered::<(Entity, &HasJob, Has<WantsTraitor>), With<PlayerControlled>>();
    let mut all: Vec<(Entity, String, bool)> =
        crew.iter(world).map(|(e, j, w)| (e, j.0.clone(), w)).collect();
    all.sort_by_key(|(e, ..)| *e);
    let heads: Vec<Entity> = all
        .iter()
        .filter(|(_, job, _)| HEADS.contains(&job.as_str()))
        .map(|(e, ..)| *e)
        .collect();
    let can_lead = |(_, job, _): &&(Entity, String, bool)| !UNCONVERTABLE.contains(&job.as_str());
    let wanting: Vec<Entity> = all
        .iter()
        .filter(can_lead)
        .filter(|(_, _, w)| *w)
        .map(|(e, ..)| *e)
        .collect();
    let mut candidates: Vec<Entity> = if wanting.is_empty() {
        all.iter().filter(can_lead).map(|(e, ..)| *e).collect()
    } else {
        wanting
    };
    if candidates.is_empty() || heads.is_empty() {
        tell_everyone(world, &LocMsg::new("rev-not-enough"));
        return false;
    }
    let mut leaders = Vec::new();
    let mut rng = world.resource_mut::<GameRng>();
    while leaders.len() < HEAD_REVS && !candidates.is_empty() {
        let i = rng.range(0, candidates.len() as i32 - 1) as usize;
        leaders.push(candidates.remove(i));
    }
    let names: Vec<String> = heads.iter().map(|head| name_of(world, *head)).collect();
    for leader in leaders.iter().copied() {
        world.entity_mut(leader).insert(HeadRev);
        tell(world, leader, LocMsg::new("rev-leader"));
        let objective = if rp {
            "rev-objective-capture"
        } else {
            "rev-objective-kill"
        };
        for name in &names {
            tell(world, leader, LocMsg::new(objective).arg("name", name.clone()));
        }
        if rp {
            tell(world, leader, LocMsg::new("rev-rp-howto"));
        } else {
            give_flash(world, leader);
            tell(world, leader, LocMsg::new("rev-flash-howto"));
        }
    }
    let now = world.resource::<WorldTime>().deciseconds();
    world.insert_resource(RevState {
        leaders,
        heads,
        converted: Vec::new(),
        revs_won: None,
        reveal_at: rp.then_some(now + REVEAL_DS),
    });
    true
}

/// Join the revolution, if they can (Luna `add_revolutionary`).
pub fn add_revolutionary(world: &mut World, mob: Entity) -> bool {
    let already = world.get::<HeadRev>(mob).is_some() || world.get::<Revolutionary>(mob).is_some();
    let barred = job_of(world, mob).is_some_and(|job| UNCONVERTABLE.contains(&job.as_str()));
    if already || barred || world.get::<Mob>(mob).is_none() {
        return false;
    }
    world.entity_mut(mob).insert(Revolutionary);
    if let Some(mut state) = world.get_resource_mut::<RevState>() {
        state.converted.push(mob);
    }
    tell(world, mob, LocMsg::new("rev-joined"));
    true
}

/// Leave it: only those who've been turned can be turned back (Luna `remove_revolutionary`).
pub fn remove_revolutionary(world: &mut World, mob: Entity) -> bool {
    if world.get::<Revolutionary>(mob).is_none() {
        return false;
    }
    world.entity_mut(mob).remove::<Revolutionary>();
    if let Some(mut state) = world.get_resource_mut::<RevState>() {
        state.converted.retain(|m| *m != mob);
    }
    tell(world, mob, LocMsg::new("rev-brainwashed"));
    if let Some(at) = world.get::<GridPos>(mob).map(|p| p.0) {
        let msg = LocMsg::new("rev-remembered").arg("name", name_of(world, mob));
        world.resource_mut::<VisibleEvents>().0.push((at, msg, None));
    }
    true
}

/// A leader's flash in the face turns someone, in a plain revolution.
pub fn flashed(world: &mut World, user: Entity, target: Entity) {
    if world.get::<HeadRev>(user).is_some() && current_mode(world) == Some(REVOLUTION) {
        add_revolutionary(world, target);
    }
}

/// Knocked out by a blow to the head: half the time, it knocks the revolution out of them. Only in a
/// plain revolution: in Luna an RP revolutionary stays one.
pub fn knocked_on_head(world: &mut World, target: Entity) {
    if current_mode(world) == Some(REVOLUTION) && world.resource_mut::<GameRng>().prob(50.0) {
        remove_revolutionary(world, target);
    }
}

/// "Rev-Convert", in an RP revolution: ask someone in sight to join. Their answer comes back by `answer`.
pub fn ask(world: &mut World, asker: Entity, target: Entity) {
    if current_mode(world) != Some(RP_REVOLUTION) {
        return;
    }
    if world.get::<HeadRev>(asker).is_none() && world.get::<Revolutionary>(asker).is_none() {
        return;
    }
    let near = match (world.get::<GridPos>(asker), world.get::<GridPos>(target)) {
        (Some(a), Some(b)) => a.0.z == b.0.z && (a.0.x - b.0.x).abs().max((a.0.y - b.0.y).abs()) <= ASK_RANGE,
        _ => false,
    };
    if !near {
        return;
    }
    let name = name_of(world, target);
    if world.get::<HeadRev>(target).is_some() || world.get::<Revolutionary>(target).is_some() {
        tell(world, asker, LocMsg::new("rev-already").arg("name", name));
        return;
    }
    if job_of(world, target).is_some_and(|job| UNCONVERTABLE.contains(&job.as_str())) {
        tell(world, asker, LocMsg::new("rev-cannot").arg("name", name));
        return;
    }
    let now = world.resource::<WorldTime>().deciseconds();
    let again_at = world.get::<RevAsk>(target).map_or(0.0, |a| a.again_at);
    if now < again_at {
        tell(world, asker, LocMsg::new("rev-wait"));
        return;
    }
    tell(world, asker, LocMsg::new("rev-asking").arg("name", name));
    world.entity_mut(target).insert(RevAsk {
        from: Some(asker),
        again_at: now + ASK_COOLDOWN_DS,
    });
}

/// Someone's answer to being asked.
pub fn answer(world: &mut World, mob: Entity, yes: bool) {
    let Some(from) = world.get_mut::<RevAsk>(mob).and_then(|mut ask| ask.from.take()) else {
        return;
    };
    let name = name_of(world, mob);
    if yes && add_revolutionary(world, mob) {
        tell(world, from, LocMsg::new("rev-accepted").arg("name", name));
    } else if !yes {
        tell(world, mob, LocMsg::new("rev-rejected"));
        tell(world, from, LocMsg::new("rev-refused").arg("name", name));
    }
}

fn in_brig(world: &mut World, at: TilePos) -> bool {
    let mut maps = world.query::<&rs13_world::MapLayer>();
    maps.iter(world)
        .any(|m| m.0.z == at.z && m.0.area(at).is_some_and(|area| area == BRIG_AREA))
}

/// Whether a head of staff is dealt with: dead or gone, or in an RP revolution, cuffed or out cold.
fn head_done(world: &World, head: Entity, rp: bool) -> bool {
    if world.get_entity(head).is_err() || is_dead(world, head) {
        return true;
    }
    rp && (world.get::<Cuffed>(head).is_some()
        || world
            .get::<Vitals>(head)
            .is_some_and(|v| v.stat != MobStat::Conscious))
}

/// Whether a leader is dealt with: dead or gone, or in an RP revolution, cuffed or in the brig.
fn leader_done(world: &mut World, leader: Entity, rp: bool) -> bool {
    if world.get_entity(leader).is_err() || is_dead(world, leader) {
        return true;
    }
    if !rp {
        return false;
    }
    if world.get::<Cuffed>(leader).is_some() {
        return true;
    }
    world
        .get::<GridPos>(leader)
        .map(|p| p.0)
        .is_some_and(|at| in_brig(world, at))
}

/// Who wins, as soon as anyone has (Luna `check_win`), and RP's naming of the leaders.
pub fn run_revolution(world: &mut World) {
    let Some(mode) = current_mode(world) else {
        return;
    };
    if mode != REVOLUTION && mode != RP_REVOLUTION {
        return;
    }
    let rp = mode == RP_REVOLUTION;
    let Some(state) = world.get_resource::<RevState>().cloned() else {
        return;
    };
    if state.revs_won.is_some() || state.leaders.is_empty() {
        return;
    }
    let now = world.resource::<WorldTime>().deciseconds();
    if state.reveal_at.is_some_and(|at| now >= at) {
        world.resource_mut::<RevState>().reveal_at = None;
        let names: Vec<String> = state.leaders.iter().map(|m| name_of(world, *m)).collect();
        tell_everyone(world, &LocMsg::new("rev-reveal").arg("names", names.join(", ")));
    }
    let revs_won = state.heads.iter().all(|head| head_done(world, *head, rp));
    let heads_won = state.leaders.iter().all(|leader| leader_done(world, *leader, rp));
    if !revs_won && !heads_won {
        return;
    }
    // As in Luna, the revolution's win is looked at first.
    world.resource_mut::<RevState>().revs_won = Some(revs_won);
    let id = if revs_won { "rev-win" } else { "rev-lose" };
    tell_everyone(world, &LocMsg::new(id));
    for line in roll_call(world) {
        tell_everyone(world, &line);
    }
}

/// Whether the round's over: the server moves on.
pub fn over(world: &World) -> bool {
    world
        .get_resource::<RevState>()
        .is_some_and(|state| state.revs_won.is_some())
}

fn roll_call(world: &mut World) -> Vec<LocMsg> {
    let Some(state) = world.get_resource::<RevState>().cloned() else {
        return Vec::new();
    };
    let mut lines = Vec::new();
    let mut list = |heading: &str, id: &str, people: &[Entity], world: &World| {
        lines.push(LocMsg::new(heading.to_owned()));
        for mob in people {
            if world.get_entity(*mob).is_err() {
                continue;
            }
            let dead = if is_dead(world, *mob) {
                "rev-dead"
            } else {
                "rev-alive"
            };
            lines.push(
                LocMsg::new(id.to_owned())
                    .arg("name", name_of(world, *mob))
                    .msg("state", dead),
            );
        }
    };
    list("rev-roll-leaders", "rev-roll-line", &state.leaders, world);
    list("rev-roll-converted", "rev-roll-line", &state.converted, world);
    list("rev-roll-heads", "rev-roll-line", &state.heads, world);
    lines
}

/// What the round has to say as the pods arrive, if it's still undecided.
pub fn report(world: &mut World) -> Vec<LocMsg> {
    let undecided = world
        .get_resource::<RevState>()
        .is_some_and(|state| state.revs_won.is_none() && !state.leaders.is_empty());
    if undecided { roll_call(world) } else { Vec::new() }
}

/// Everyone who's in on it, and whether each leads, for those who are in on it to see (Luna's icons).
pub fn members(world: &mut World) -> Vec<(Entity, bool)> {
    let mut leaders = world.query_filtered::<Entity, With<HeadRev>>();
    let mut all: Vec<(Entity, bool)> = leaders.iter(world).map(|e| (e, true)).collect();
    let mut turned = world.query_filtered::<Entity, With<Revolutionary>>();
    all.extend(turned.iter(world).map(|e| (e, false)));
    all
}
