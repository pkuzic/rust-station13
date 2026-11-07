//! Cloning (Luna's `computer/cloning`, `clonepod`, `dna_scannernew`): scan someone, and if they
//! die their ghost can be grown a new body. Spec: `docs/reference/cloning.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, Dense, DisplayName, GridPos, Hidden, NetId, TilePos};

use crate::actions::Outbox;
use crate::closets::InCloset;
use crate::ghosts::GhostOf;
use crate::health::{Body, Zone};
use crate::machines::MACHINE_TICKS;
use crate::movement::Mob;
use crate::power::{Channel, PowerUser};
use crate::rng::GameRng;
use crate::time::WorldTime;

pub const SCANNER: &str = "dna-scanner";
pub const CONSOLE: &str = "cloning-console";
pub const POD: &str = "clone-pod";
/// How far a console reaches for its scanner and pod, in tiles.
pub const LINK_RANGE: i32 = 8;
/// The clone comes out once this healthy (Luna `heal_level`).
pub const HEAL_LEVEL: f32 = 80.0;
/// What a pod draws growing someone, W (Luna `use_power(65000)`), and idle.
pub const GROWING_WATTS: f64 = 65_000.0;
pub const IDLE_WATTS: f64 = 200.0;
/// What a new clone starts with (Luna `growclone`).
pub const START_TOX: f32 = 30.0;
pub const START_OXY: f32 = 40.0;
/// What a pod heals each cycle (Luna: brute 1, toxin 0.1; oxygen kept off by its drug).
pub const HEAL_BRUTE: f32 = 1.0;
pub const HEAL_TOX: f32 = 0.1;
pub const HEAL_OXY: f32 = 1.0;

/// A scanner, and who lies in it.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct DnaScanner {
    pub occupant: Option<Entity>,
    /// Locked from its DNA console: nobody gets out (Luna `locked`).
    pub locked: bool,
    /// A DNA console's beam is on: held shut till it's done.
    pub beam: bool,
}

/// One scanned person (Luna's record): their name, and the body scanned, by which their ghost
/// is found later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloneRecord {
    pub name: DisplayName,
    pub body: Entity,
}

/// Everyone scanned on the station (Luna `geneticsrecords`).
#[derive(Resource, Debug, Default)]
pub struct CloneRecords(pub Vec<CloneRecord>);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CloningConsole;

/// A pod, and who grows in it.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct ClonePod {
    pub occupant: Option<Entity>,
}

/// Ghosts who are getting a body, and the body, for whatever connects players to mobs to
/// follow: `(ghost, body)`.
#[derive(Resource, Debug, Default)]
pub struct Arrivals(pub Vec<(Entity, Entity)>);

pub fn is_cloning_part(proto: &str) -> bool {
    matches!(proto, SCANNER | CONSOLE | POD)
}

pub fn spawn_part(world: &mut World, proto: &str, pos: TilePos, id: NetId) -> Entity {
    let mut entity = world.spawn((GridPos(pos), id, Dense));
    match proto {
        SCANNER => entity.insert((
            Appearance::new(SCANNER, "scanner_0"),
            DnaScanner::default(),
            PowerUser::new(Channel::Equipment, 50.0),
        )),
        POD => entity.insert((
            Appearance::new(POD, "pod_0"),
            ClonePod::default(),
            PowerUser::new(Channel::Equipment, IDLE_WATTS),
        )),
        _ => entity.insert((
            Appearance::new(CONSOLE, "dna"),
            CloningConsole,
            PowerUser::new(Channel::Equipment, 100.0),
        )),
    };
    entity.id()
}

fn tell(world: &mut World, mob: Entity, id: &str) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(LocMsg::new(id));
    }
}

/// What a console finds nearby, nearest first.
fn nearest<T: Component>(world: &mut World, at: TilePos) -> Option<Entity> {
    let mut q = world.query_filtered::<(Entity, &GridPos), With<T>>();
    q.iter(world)
        .filter(|(_, p)| p.0.z == at.z && (p.0.x - at.x).abs().max((p.0.y - at.y).abs()) <= LINK_RANGE)
        .min_by_key(|(e, p)| ((p.0.x - at.x).abs() + (p.0.y - at.y).abs(), e.index()))
        .map(|(e, _)| e)
}

/// A click on a scanner: someone being pulled is laid inside (Luna `attackby` with a grab);
/// with nobody pulled, whoever is inside comes out.
pub fn click_scanner(world: &mut World, mob: Entity, scanner: Entity) {
    let Some(at) = world.get::<GridPos>(scanner).map(|p| p.0) else {
        return;
    };
    let occupant = world.get::<DnaScanner>(scanner).and_then(|s| s.occupant);
    let pulled = world
        .get::<crate::pulling::Pulling>(mob)
        .map(|p| p.0)
        .filter(|p| world.get::<Body>(*p).is_some());
    if let Some(person) = pulled {
        if occupant.is_some() {
            tell(world, mob, "scanner-occupied");
            return;
        }
        world.entity_mut(mob).remove::<crate::pulling::Pulling>();
        world
            .entity_mut(person)
            .insert((InCloset(scanner), Hidden, GridPos(at)));
        if let Some(mut s) = world.get_mut::<DnaScanner>(scanner) {
            s.occupant = Some(person);
        }
        set_look(world, scanner, "scanner_1");
    } else if let Some(person) = occupant {
        release(world, scanner, person);
    }
}

/// Luna `dna_scannernew/relaymove`: someone inside and awake walks out. True if this was a scanner.
pub fn leave(world: &mut World, mob: Entity, machine: Entity) -> bool {
    if world.get::<DnaScanner>(machine).is_none() {
        return false;
    }
    let awake = world
        .get::<crate::health::Vitals>(mob)
        .is_none_or(|v| v.stat == rs13_world::MobStat::Conscious);
    if awake {
        release(world, machine, mob);
    }
    true
}

fn release(world: &mut World, scanner: Entity, person: Entity) {
    // Luna `go_out`: not while it's locked.
    if world
        .get::<DnaScanner>(scanner)
        .is_some_and(|s| s.locked || s.beam)
    {
        return;
    }
    if let Ok(mut e) = world.get_entity_mut(person) {
        e.remove::<(InCloset, Hidden)>();
    }
    if let Some(mut s) = world.get_mut::<DnaScanner>(scanner) {
        s.occupant = None;
        s.locked = false;
    }
    set_look(world, scanner, "scanner_0");
}

fn set_look(world: &mut World, entity: Entity, state: &str) {
    if let Some(mut look) = world.get_mut::<Appearance>(entity) {
        look.state = state.to_owned();
    }
}

/// A click on a console (Luna's window, as a single button): scan whoever is in the scanner, or
/// with nobody there, start cloning the first scanned person who is a ghost.
pub fn click_console(world: &mut World, mob: Entity, console: Entity) {
    let Some(at) = world.get::<GridPos>(console).map(|p| p.0) else {
        return;
    };
    if !world.get::<PowerUser>(console).is_some_and(|p| p.powered) {
        return;
    }
    let scanner = nearest::<DnaScanner>(world, at);
    let occupant = scanner
        .and_then(|s| world.get::<DnaScanner>(s))
        .and_then(|s| s.occupant);
    if let Some(person) = occupant {
        let id = scan(world, person);
        tell(world, mob, id);
        return;
    }
    let id = start_clone(world, at);
    tell(world, mob, id);
}

/// Luna `scan_mob`: the living and the dead alike, but only people.
fn scan(world: &mut World, person: Entity) -> &'static str {
    let Some(name) = world.get::<DisplayName>(person).cloned() else {
        return "cloning-no-genes";
    };
    if world.get::<crate::monkeys::Monkey>(person).is_some()
        || world.get::<crate::changeling::Husked>(person).is_some()
    {
        return "cloning-no-genes";
    }
    let mut records = world.resource_mut::<CloneRecords>();
    if records.0.iter().any(|r| r.body == person) {
        return "cloning-already-scanned";
    }
    records.0.push(CloneRecord { name, body: person });
    "cloning-scanned"
}

/// Luna's Clone link: the first record whose person is a ghost, into a free pod.
fn start_clone(world: &mut World, at: TilePos) -> &'static str {
    let Some(pod) = nearest::<ClonePod>(world, at) else {
        return "cloning-no-pod";
    };
    if world.get::<ClonePod>(pod).is_some_and(|p| p.occupant.is_some()) {
        return "cloning-busy";
    }
    let mut ghosts = world.query::<(Entity, &GhostOf)>();
    let ghosts: Vec<(Entity, Entity)> = ghosts.iter(world).map(|(g, of)| (g, of.0)).collect();
    let records = &world.resource::<CloneRecords>().0;
    let found = records.iter().enumerate().find_map(|(index, record)| {
        ghosts
            .iter()
            .find(|(_, body)| *body == record.body)
            .map(|(ghost, _)| (index, *ghost, record.name.clone()))
    });
    let Some((index, ghost, name)) = found else {
        return "cloning-nobody";
    };
    world.resource_mut::<CloneRecords>().0.remove(index);
    grow(world, pod, ghost, name);
    "cloning-started"
}

/// Luna `growclone`: a new, badly hurt body in the pod, with its player already in it.
fn grow(world: &mut World, pod: Entity, ghost: Entity, name: DisplayName) {
    let Some(at) = world.get::<GridPos>(pod).map(|p| p.0) else {
        return;
    };
    let id = world.resource_mut::<crate::objects::NetIds>().alloc();
    let clone = crate::objects::spawn_bare_mob(world, at, id);
    let brute = world.resource_mut::<GameRng>().range(60, 90) as f32;
    if let Some(mut body) = world.get_mut::<Body>(clone) {
        body.take_damage_spread(brute, 0.0);
        body.tox = START_TOX;
        body.oxy = START_OXY;
    }
    world.entity_mut(clone).insert((
        name,
        InCloset(pod),
        Hidden,
        crate::equipment::Equipment::default(),
    ));
    if let Some(mut p) = world.get_mut::<ClonePod>(pod) {
        p.occupant = Some(clone);
    }
    set_look(world, pod, "pod_1");
    if let Some(mut power) = world.get_mut::<PowerUser>(pod) {
        power.watts = GROWING_WATTS;
    }
    world.resource_mut::<Arrivals>().0.push((ghost, clone));
    if let Some(mut outbox) = world.get_mut::<Outbox>(clone) {
        outbox.0.push(LocMsg::new("cloning-begins"));
    }
}

/// Every 2 s pods heal their clones, and let them out once healthy (Luna `clonepod/process`).
/// A power cut leaves them waiting. A scanner gives up someone who has gone.
pub fn run_cloning(world: &mut World) {
    if !world.resource::<WorldTime>().tick.0.is_multiple_of(MACHINE_TICKS) {
        return;
    }
    let mut pods = world.query::<(Entity, &ClonePod, &PowerUser)>();
    let growing: Vec<(Entity, Entity, bool)> = pods
        .iter(world)
        .filter_map(|(pod, p, power)| p.occupant.map(|o| (pod, o, power.powered)))
        .collect();
    for (pod, clone, powered) in growing {
        if world.get::<Mob>(clone).is_none() {
            empty_pod(world, pod);
            continue;
        }
        if !powered {
            continue;
        }
        let Some(mut body) = world.get_mut::<Body>(clone) else {
            continue;
        };
        body.tox = (body.tox - HEAL_TOX).max(0.0);
        body.oxy = (body.oxy - HEAL_OXY).max(0.0);
        let per_zone = HEAL_BRUTE / Zone::ALL.len() as f32;
        for zone in Zone::ALL {
            body.heal(zone, per_zone, 0.0);
        }
        if body.health() >= HEAL_LEVEL {
            world.entity_mut(clone).remove::<(InCloset, Hidden)>();
            empty_pod(world, pod);
            if let Some(mut outbox) = world.get_mut::<Outbox>(clone) {
                outbox.0.push(LocMsg::new("cloning-done"));
            }
        }
    }
}

fn empty_pod(world: &mut World, pod: Entity) {
    if let Some(mut p) = world.get_mut::<ClonePod>(pod) {
        p.occupant = None;
    }
    set_look(world, pod, "pod_0");
    if let Some(mut power) = world.get_mut::<PowerUser>(pod) {
        power.watts = IDLE_WATTS;
    }
}
