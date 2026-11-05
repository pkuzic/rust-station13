//! The nuclear emergency: a Syndicate team, the bomb, the disk that arms it and the pinpointer that
//! finds it (Luna's `gamemodes/nuclear`). Spec: `docs/reference/nuke.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{
    Appearance, Dense, DisplayName, Facing, GridPos, MobStat, NetId, NukePanelView, ObjectSpec, TilePos,
};

use crate::actions::{Outbox, in_reach};
use crate::closets::InCloset;
use crate::equipment::{Equipment, WornBy};
use crate::escape::in_pod;
use crate::hands::{Hands, HeldBy};
use crate::health::Vitals;
use crate::jobs::{HasJob, wear};
use crate::objects::spawn_object;
use crate::rng::GameRng;
use crate::round::{Phase, Round};
use crate::storage::{InStorage, Storage};
use crate::time::WorldTime;
use crate::traitor::Uplink;
use crate::wires::PanelViewer;

pub const BOMB: &str = "nuclear-bomb";
pub const DISK: &str = "nuclear-disk";
pub const PINPOINTER: &str = "pinpointer";
/// The most agents there can be (Luna `pre_setup`).
pub const MOST_AGENTS: usize = 5;
/// Crystals in the team's uplink, for each agent.
pub const CRYSTALS_EACH: u8 = 10;
/// What the bomb's timer starts at, and its limits, in seconds.
const TIMER_START: i32 = 60;
const TIMER_MIN: i32 = 5;
const TIMER_MAX: i32 = 600;
/// The wait between the timer running out and the blast.
const BLAST_DELAY_DS: f32 = 20.0;
/// Luna's titles for the team's leader. Names aren't translated.
const LEADER_TITLES: [&str; 7] = [
    "Czar",
    "Boss",
    "Commander",
    "Chief",
    "Kingpin",
    "Director",
    "Overlord",
];

/// A marked place on the map, only used when a nuclear round is set up (Luna's landmarks).
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct Landmark(pub String);

/// The kinds of landmark.
pub const SYND_SPAWN: &str = "landmark-synd-spawn";
pub const DISK_SPAWN: &str = "landmark-disk-spawn";
pub const NUKE_SPAWN: &str = "landmark-nuke-spawn";
pub const CLOSET_SPAWN: &str = "landmark-closet-spawn";
pub const GEAR_CLOSET: &str = "landmark-gear-closet";
pub const SYN_BOMB: &str = "landmark-synbomb";

#[derive(Component, Debug, Clone)]
pub struct NuclearBomb {
    /// Unfolded and bolted down, by the first click.
    pub extended: bool,
    pub anchored: bool,
    pub timing: bool,
    pub time_left: i32,
    /// What's been typed.
    pub code: String,
    /// What has to be typed.
    pub real_code: String,
    /// The right code has been entered.
    pub code_ok: bool,
    pub safety: bool,
    pub disk: Option<Entity>,
    next_second: f32,
    /// It can't be stopped now: when it goes off.
    pub blast_at: Option<f32>,
}

impl NuclearBomb {
    /// `real_code` is Luna's "ADMIN" outside a nuclear round: no keypad can type that.
    pub fn new(real_code: impl Into<String>) -> Self {
        Self {
            extended: false,
            anchored: false,
            timing: false,
            time_left: TIMER_START,
            code: String::new(),
            real_code: real_code.into(),
            code_ok: false,
            safety: true,
            disk: None,
            next_second: 0.0,
            blast_at: None,
        }
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NuclearDisk;

#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Pinpointer {
    pub active: bool,
}

/// One of the team.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NukeAgent;

/// Would like to be on the team (Luna `be_nuke_agent`): those who would are picked first.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct WantsNuke;

/// What a mob's open bomb window shows.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct NukePanelOut(pub Option<NukePanelView>);

/// What's become of the station.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct NukeState {
    pub detonated: bool,
}

/// What can be asked of the bomb's window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NukeOp {
    Disk,
    Key(u8),
    Reset,
    Enter,
    Time(i32),
    Timer,
    Safety,
    Anchor,
}

pub fn spawn_bomb(world: &mut World, pos: TilePos, id: NetId, real_code: &str) -> Entity {
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(BOMB, "0"),
            Dense,
            NuclearBomb::new(real_code),
        ))
        .id()
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

/// A click: the first unfolds and bolts it down, the rest open its window.
pub fn click(world: &mut World, mob: Entity, target: Entity) {
    let Some(mut bomb) = world.get_mut::<NuclearBomb>(target) else {
        return;
    };
    if bomb.extended {
        world
            .entity_mut(mob)
            .insert((PanelViewer(target), NukePanelOut::default()));
        return;
    }
    bomb.extended = true;
    bomb.anchored = true;
    if let Some(mut look) = world.get_mut::<Appearance>(target) {
        look.state = "1".to_owned();
    }
}

/// A press of the window's buttons, by someone at the bomb (Luna's `Topic`).
pub fn apply(world: &mut World, mob: Entity, target: Entity, op: NukeOp) {
    let near = match (world.get::<GridPos>(mob), world.get::<GridPos>(target)) {
        (Some(a), Some(b)) => in_reach(a.0, b.0),
        _ => false,
    };
    let awake = world
        .get::<Vitals>(mob)
        .is_none_or(|v| v.stat == MobStat::Conscious);
    if !near || !awake || world.get::<NuclearBomb>(target).is_none() {
        return;
    }
    if op == NukeOp::Disk {
        swap_disk(world, mob, target);
        return;
    }
    let Some(mut bomb) = world.get_mut::<NuclearBomb>(target) else {
        return;
    };
    // With no disk in it, nothing on it works.
    if bomb.disk.is_none() {
        return;
    }
    match op {
        NukeOp::Disk => {}
        NukeOp::Key(digit) => {
            bomb.code.push_str(&digit.min(9).to_string());
            if bomb.code.len() > 5 {
                bomb.code = "ERROR".to_owned();
            }
        }
        NukeOp::Reset => {
            bomb.code_ok = false;
            bomb.code.clear();
        }
        NukeOp::Enter => {
            if bomb.code == bomb.real_code {
                bomb.code_ok = true;
                bomb.code.clear();
            } else {
                bomb.code = "ERROR".to_owned();
            }
        }
        NukeOp::Time(change) if bomb.code_ok => {
            bomb.time_left = (bomb.time_left + change).clamp(TIMER_MIN, TIMER_MAX);
        }
        NukeOp::Timer if bomb.code_ok && bomb.blast_at.is_none() => {
            bomb.timing = !bomb.timing;
            let state = if bomb.timing { "2" } else { "1" };
            if let Some(mut look) = world.get_mut::<Appearance>(target) {
                look.state = state.to_owned();
            }
        }
        NukeOp::Safety if bomb.code_ok => bomb.safety = !bomb.safety,
        NukeOp::Anchor if bomb.code_ok => bomb.anchored = !bomb.anchored,
        _ => {}
    }
}

/// Take the disk out, onto the floor, or put in the one in hand. Without it the code's forgotten.
fn swap_disk(world: &mut World, mob: Entity, target: Entity) {
    let Some(at) = world.get::<GridPos>(target).map(|p| p.0) else {
        return;
    };
    let inside = world.get::<NuclearBomb>(target).and_then(|b| b.disk);
    if let Some(disk) = inside {
        world.entity_mut(disk).insert(GridPos(at));
        if let Some(mut bomb) = world.get_mut::<NuclearBomb>(target) {
            bomb.disk = None;
            bomb.code_ok = false;
        }
        return;
    }
    let held = world
        .get::<Hands>(mob)
        .and_then(Hands::active_item)
        .filter(|item| world.get::<NuclearDisk>(*item).is_some());
    let Some(disk) = held else {
        return;
    };
    if let Some(mut hands) = world.get_mut::<Hands>(mob) {
        let hand = hands.active;
        *hands.slot_mut(hand) = None;
    }
    world.entity_mut(disk).remove::<HeldBy>();
    if let Some(mut bomb) = world.get_mut::<NuclearBomb>(target) {
        bomb.disk = Some(disk);
    }
}

/// The timer, and what happens when it runs out (Luna's `process`, `explode`).
pub fn run_nuke(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    let mut bombs = world.query::<(Entity, &mut NuclearBomb)>();
    let mut goes_off = false;
    let mut looks: Vec<(Entity, &str)> = Vec::new();
    for (entity, mut bomb) in bombs.iter_mut(world) {
        if let Some(at) = bomb.blast_at {
            if now >= at {
                bomb.blast_at = Some(f32::INFINITY);
                goes_off = goes_off || at.is_finite();
            }
            continue;
        }
        if !bomb.timing || now < bomb.next_second {
            continue;
        }
        bomb.next_second = now + 10.0;
        bomb.time_left -= 1;
        if bomb.time_left > 0 {
            continue;
        }
        if bomb.safety {
            // It just stops, with the safety on.
            bomb.timing = false;
            bomb.time_left = TIMER_MIN;
            looks.push((entity, "1"));
        } else {
            bomb.code_ok = false;
            bomb.blast_at = Some(now + BLAST_DELAY_DS);
            looks.push((entity, "3"));
        }
    }
    for (entity, state) in looks {
        if let Some(mut look) = world.get_mut::<Appearance>(entity) {
            look.state = state.to_owned();
        }
    }
    if goes_off && !world.resource::<NukeState>().detonated {
        world.resource_mut::<NukeState>().detonated = true;
        let nuclear = mode_is_nuclear(world);
        tell_everyone(world, &LocMsg::new("nuke-blast"));
        if nuclear {
            tell_everyone(world, &LocMsg::new("nuke-win-syndicate"));
            for line in agent_lines(world) {
                tell_everyone(world, &line);
            }
        } else {
            tell_everyone(world, &LocMsg::new("nuke-killed-everyone"));
        }
    }
}

fn mode_is_nuclear(world: &World) -> bool {
    world
        .get_resource::<Round>()
        .is_some_and(|round| matches!(round.phase, Phase::Playing { mode: "nuclear" }))
}

fn agent_lines(world: &mut World) -> Vec<LocMsg> {
    let mut agents = world.query_filtered::<(Entity, &DisplayName, Option<&Vitals>), With<NukeAgent>>();
    let mut all: Vec<_> = agents
        .iter(world)
        .map(|(e, name, vitals)| (e, name.0.clone(), vitals.is_some_and(|v| v.stat == MobStat::Dead)))
        .collect();
    all.sort_by_key(|(e, ..)| *e);
    all.into_iter()
        .map(|(_, name, dead)| {
            LocMsg::new(if dead { "nuke-agent-dead" } else { "nuke-agent" }).arg("name", name)
        })
        .collect()
}

/// Where something is, however it's carried.
pub fn tile_of(world: &World, thing: Entity) -> Option<TilePos> {
    let mut at = thing;
    // Nothing is carried more than a few levels deep.
    for _ in 0..8 {
        if let Some(pos) = world.get::<GridPos>(at) {
            return Some(pos.0);
        }
        at = world
            .get::<HeldBy>(at)
            .map(|holder| holder.0)
            .or_else(|| world.get::<WornBy>(at).map(|wearer| wearer.0))
            .or_else(|| world.get::<InStorage>(at).map(|store| store.0))
            .or_else(|| world.get::<InCloset>(at).map(|closet| closet.0))?;
    }
    None
}

/// How the round went, as the pods arrive (Luna's `declare_completion`).
pub fn report(world: &mut World) -> Vec<LocMsg> {
    if !mode_is_nuclear(world) {
        return Vec::new();
    }
    let mut disks = world.query_filtered::<Entity, With<NuclearDisk>>();
    let found: Vec<Entity> = disks.iter(world).collect();
    let away = found
        .into_iter()
        .any(|disk| tile_of(world, disk).is_some_and(|at| in_pod(world, at)));
    let mut lines = vec![LocMsg::new(if away { "nuke-win-crew" } else { "nuke-neutral" })];
    lines.extend(agent_lines(world));
    lines
}

/// Switch a pinpointer on or off.
pub fn use_pinpointer(world: &mut World, mob: Entity, pin: Entity) {
    let Some(mut pinpointer) = world.get_mut::<Pinpointer>(pin) else {
        return;
    };
    pinpointer.active = !pinpointer.active;
    if pinpointer.active {
        let mut disks = world.query_filtered::<Entity, With<NuclearDisk>>();
        let disk = disks.iter(world).next();
        let (mine, theirs) = (tile_of(world, pin), disk.and_then(|d| tile_of(world, d)));
        let deck = match (mine, theirs) {
            (Some(a), Some(b)) if b.z > a.z => "up",
            (Some(a), Some(b)) if b.z < a.z => "down",
            _ => "level",
        };
        tell(world, mob, LocMsg::new("nuke-pin-on").arg("deck", deck));
    } else {
        if let Some(mut look) = world.get_mut::<Appearance>(pin) {
            look.state = String::new();
        }
        tell(world, mob, LocMsg::new("nuke-pin-off"));
    }
}

/// What an active pinpointer shows, by how far the disk is: Luna's `work`.
pub fn pin_state(from: TilePos, to: TilePos) -> &'static str {
    match (from.x - to.x).abs().max((from.y - to.y).abs()) {
        0 => "direct",
        1..=8 => "close",
        9..=16 => "medium",
        _ => "far",
    }
}

/// The way a pinpointer's needle points: `Facing` numbers directions as Luna's icons do.
fn pin_facing(from: TilePos, to: TilePos) -> u8 {
    match ((to.x - from.x).signum(), (to.y - from.y).signum()) {
        (0, 1) => 1,
        (1, 0) => 2,
        (-1, 0) => 3,
        (1, -1) => 4,
        (-1, -1) => 5,
        (1, 1) => 6,
        (-1, 1) => 7,
        _ => 0,
    }
}

pub fn update_pinpointers(world: &mut World) {
    let mut pins = world.query::<(Entity, &Pinpointer)>();
    let active: Vec<Entity> = pins
        .iter(world)
        .filter(|(_, p)| p.active)
        .map(|(e, _)| e)
        .collect();
    if active.is_empty() {
        return;
    }
    let mut disks = world.query_filtered::<Entity, With<NuclearDisk>>();
    let disk = disks.iter(world).next();
    let target = disk.and_then(|d| tile_of(world, d));
    for pin in active {
        let from = tile_of(world, pin);
        let (state, facing) = match (from, target) {
            (Some(from), Some(to)) => (pin_state(from, to), pin_facing(from, to)),
            _ => ("null", 0),
        };
        let mut entity = world.entity_mut(pin);
        if let Some(mut look) = entity.get_mut::<Appearance>()
            && look.state != state
        {
            look.state = state.to_owned();
        }
        if entity.get::<Facing>().is_none_or(|f| f.0 != facing) {
            entity.insert(Facing(facing));
        }
    }
}

/// Show each player the bomb they have open.
#[allow(clippy::type_complexity)]
pub fn update_nuke_views(
    mut viewers: Query<(
        Entity,
        &GridPos,
        Option<&PanelViewer>,
        &mut NukePanelOut,
        Option<&Vitals>,
    )>,
    bombs: Query<(&GridPos, &NetId, &NuclearBomb)>,
    mut commands: Commands,
) {
    for (mob, pos, viewer, mut out, vitals) in &mut viewers {
        let target = viewer.and_then(|v| bombs.get(v.0).ok());
        let view = target.and_then(|(at, id, bomb)| {
            let usable = in_reach(pos.0, at.0) && vitals.is_none_or(|v| v.stat == MobStat::Conscious);
            usable.then(|| NukePanelView {
                target: *id,
                disk: bomb.disk.is_some(),
                code_ok: bomb.code_ok,
                display: bomb.code.clone(),
                timing: bomb.timing,
                time_left: bomb.time_left.max(0) as u32,
                safety: bomb.safety,
                anchored: bomb.anchored,
            })
        });
        if view.is_none() && target.is_some() {
            commands.entity(mob).remove::<PanelViewer>();
        }
        if out.0 != view {
            out.0 = view;
        }
    }
}

fn places(world: &mut World, kind: &str) -> Vec<(TilePos, Entity)> {
    let mut marks = world.query::<(Entity, &Landmark, &GridPos)>();
    let mut found: Vec<(TilePos, Entity)> = marks
        .iter(world)
        .filter(|(_, mark, _)| mark.0 == kind)
        .map(|(e, _, pos)| (pos.0, e))
        .collect();
    found.sort_by_key(|(_, e)| *e);
    found
}

/// Everything someone wears and holds is gone, contents and all.
fn strip(world: &mut World, mob: Entity) {
    let mut things: Vec<Entity> = world
        .get::<Equipment>(mob)
        .map_or_else(Vec::new, |e| e.0.values().copied().collect());
    if let Some(mut equipment) = world.get_mut::<Equipment>(mob) {
        equipment.0.clear();
    }
    if let Some(mut hands) = world.get_mut::<Hands>(mob) {
        things.extend(hands.left.take());
        things.extend(hands.right.take());
    }
    while let Some(thing) = things.pop() {
        if let Some(storage) = world.get::<Storage>(thing) {
            things.extend(storage.items.iter().copied());
        }
        world.entity_mut(thing).despawn();
    }
}

/// What every agent starts in (Luna's `equip_syndicate`).
const OUTFIT: [&str; 5] = [
    "headset-syndicate",
    "syndicate-jumpsuit",
    "black-shoes",
    "swat-gloves",
    "backpack",
];

fn make_agent(world: &mut World, mob: Entity, at: TilePos) {
    strip(world, mob);
    // They've no job after all.
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.retain(|msg| msg.id != "job-assigned");
    }
    world
        .entity_mut(mob)
        .remove::<HasJob>()
        .insert((NukeAgent, GridPos(at)));
    for id in OUTFIT {
        if let Ok(item) = spawn_object(world, &ObjectSpec::new(id), at) {
            wear(world, mob, item);
        }
    }
}

/// How many are on the team, out of how many are playing (Luna's `pre_setup`).
pub fn team_size(players: usize) -> usize {
    (players / 3).clamp(1, MOST_AGENTS)
}

/// Choose the team, send them to their base, and lay out the disk, the bomb and the gear (Luna's
/// `post_setup`). A map without the landmarks gets none of it.
pub fn setup_round(world: &mut World) {
    let spawns = places(world, SYND_SPAWN);
    if spawns.is_empty() {
        return;
    }
    let mut crew = world.query_filtered::<(Entity, Has<WantsNuke>), With<crate::objects::PlayerControlled>>();
    let mut all: Vec<(Entity, bool)> = crew.iter(world).collect();
    all.sort_by_key(|(e, _)| *e);
    let count = team_size(all.len()).min(spawns.len());
    // If anyone would like to be one, it's from among them.
    let wanting: Vec<Entity> = all.iter().filter(|(_, w)| *w).map(|(e, _)| *e).collect();
    let mut candidates = if wanting.is_empty() {
        all.iter().map(|(e, _)| *e).collect()
    } else {
        wanting
    };
    let mut rng = world.resource_mut::<GameRng>();
    let code = rng.range(10_000, 99_999).to_string();
    let title = LEADER_TITLES[rng.range(0, LEADER_TITLES.len() as i32 - 1) as usize];
    let mut team = Vec::new();
    while team.len() < count && !candidates.is_empty() {
        let i = rng.range(0, candidates.len() as i32 - 1) as usize;
        team.push(candidates.remove(i));
    }
    let disk_spot = {
        let disks = places(world, DISK_SPAWN);
        (!disks.is_empty()).then(|| {
            let i = world.resource_mut::<GameRng>().range(0, disks.len() as i32 - 1) as usize;
            disks[i].0
        })
    };
    let agents = team.len();
    for (i, mob) in team.iter().copied().enumerate() {
        make_agent(world, mob, spawns[i].0);
        tell(world, mob, LocMsg::new("nuke-agent-brief"));
        if i == 0 {
            world
                .entity_mut(mob)
                .insert(DisplayName(format!("Syndicate {title}")));
            tell(
                world,
                mob,
                LocMsg::new("nuke-leader-code").arg("code", code.clone()),
            );
            tell(world, mob, LocMsg::new("nuke-guide"));
        }
        tell(world, mob, LocMsg::new("nuke-radio"));
    }
    if let Some(at) = disk_spot {
        let _ = spawn_object(world, &ObjectSpec::new(DISK), at);
    }
    let closet_spot = places(world, CLOSET_SPAWN).first().map(|(at, _)| *at);
    // Luna's map has no place for the bomb itself: it goes beside the team's closet.
    let bomb_spot = places(world, NUKE_SPAWN)
        .first()
        .map(|(at, _)| *at)
        .or_else(|| closet_spot.and_then(|at| free_beside(world, at)));
    if let Some(at) = bomb_spot {
        let id = world.resource_mut::<crate::objects::NetIds>().alloc();
        spawn_bomb(world, at, id, &code);
    }
    if let Some(at) = closet_spot
        && let Ok(closet) = spawn_object(world, &ObjectSpec::new("closet-syndicate-nuclear"), at)
    {
        let mut uplinks = world.query::<(&mut Uplink, &InCloset)>();
        for (mut uplink, inside) in uplinks.iter_mut(world) {
            if inside.0 == closet {
                uplink.crystals = (agents as u8).saturating_mul(CRYSTALS_EACH);
            }
        }
    }
    for (at, _) in places(world, GEAR_CLOSET) {
        let _ = spawn_object(world, &ObjectSpec::new("closet-syndicate-personal"), at);
    }
    let mut marks = world.query::<(&Landmark, &GridPos, Option<&LandmarkStrong>)>();
    let bombs: Vec<(TilePos, bool)> = marks
        .iter(world)
        .filter(|(mark, ..)| mark.0 == SYN_BOMB)
        .map(|(_, pos, strong)| (pos.0, strong.is_some()))
        .collect();
    for (at, strong) in bombs {
        let id = if strong { "c4-large" } else { "c4-small" };
        let _ = spawn_object(world, &ObjectSpec::new(id), at);
    }
}

/// A `SYN_BOMB` that's Luna's "Syndicate-Bomb-Strong".
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LandmarkStrong;

/// A floor tile next to this one with nothing dense on it.
fn free_beside(world: &mut World, at: TilePos) -> Option<TilePos> {
    let mut dense = world.query_filtered::<&GridPos, With<Dense>>();
    let taken: Vec<TilePos> = dense.iter(world).map(|p| p.0).collect();
    let mut maps = world.query::<&rs13_world::MapLayer>();
    let mut open = |tile: TilePos, world: &mut World| {
        maps.iter(world)
            .any(|m| m.0.z == tile.z && m.0.tile(tile).is_some_and(|t| !t.solid && t.id != "space"))
    };
    [(1, 0), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .map(|(dx, dy)| TilePos::new(at.x + dx, at.y + dy, at.z))
        .find(|tile| !taken.contains(tile) && open(*tile, world))
}
