//! Sleepers and cryo cells (Luna's `machinery/sleeper`, `sleep_console`, `unary/cryo_cell`): lay
//! someone inside, and treat them from there. Spec: `docs/reference/medbay.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{
    Appearance, Dense, Dir, DisplayName, GridPos, Hidden, MobStat, NetId, SleeperPanelView, TilePos,
};

use crate::actions::{Outbox, in_reach};
use crate::chemistry::{Reagents, is_glass, transfer};
use crate::closets::InCloset;
use crate::combat::VisibleEvents;
use crate::hands::{Hands, HeldBy};
use crate::health::{Body, Vitals, Zone};
use crate::machines::{MACHINE_TICKS, PipeNets, side_bits};
use crate::power::{Channel, PowerUser};
use crate::rng::GameRng;
use crate::time::WorldTime;
use crate::wires::PanelViewer;

pub const SLEEPER: &str = "sleeper";
pub const SLEEPER_CONSOLE: &str = "sleeper-console";
pub const CRYO_CELL: &str = "cryo-cell";
/// How far a console reaches for its sleeper, tiles.
pub const LINK_RANGE: i32 = 3;
/// What a console can inject, and how much at a time (Luna `inject_*`).
pub const INJECTIONS: [(&str, f32); 5] = [
    ("inaprovaline", 30.0),
    ("stoxin", 20.0),
    ("dermaline", 20.0),
    ("bicaridine", 20.0),
    ("dexalin", 20.0),
];
/// A console won't inject past this much in someone (Luna's `< 61`).
pub const INJECT_LIMIT: f32 = 60.0;
/// Below these gas temperatures a cryo cell does more (Luna `T0C`, its 225, and cryoxadone's 170).
pub const FREEZING: f64 = 273.15;
pub const DEEP_COLD: f64 = 225.0;
pub const CRYOXADONE_COLD: f64 = 170.0;
/// It needs this much gas around its occupant, moles (Luna `total_moles() < 10`).
pub const CRYO_MIN_MOLES: f64 = 10.0;
/// Cycles between doses from its beaker (Luna `next_trans == 12`), and what a dose is.
pub const CRYO_DOSE_CYCLES: u32 = 12;
pub const CRYO_DOSE: f32 = 1.0;

/// A sleeper, and who lies in it.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Sleeper {
    pub occupant: Option<Entity>,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SleeperConsole;

/// A cryo cell: who is in it, its beaker, and whether it's on.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CryoCell {
    pub occupant: Option<Entity>,
    pub beaker: Option<Entity>,
    pub on: bool,
    pub dose_in: u32,
}

/// What a mob asked of a sleeper console this tick: which line of `INJECTIONS`.
#[derive(Component, Debug, Default)]
pub struct PendingSleeperOps(pub Vec<(Entity, usize)>);

/// What a mob's open sleeper console window shows.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct SleeperPanelOut(pub Option<SleeperPanelView>);

pub fn is_medbay_part(proto: &str) -> bool {
    matches!(proto, SLEEPER | SLEEPER_CONSOLE | CRYO_CELL)
}

pub fn spawn_part(world: &mut World, proto: &str, pos: TilePos, id: NetId) -> Entity {
    let mut entity = world.spawn((GridPos(pos), id, Dense));
    match proto {
        SLEEPER => entity.insert((Appearance::new(SLEEPER, "sleeper-open"), Sleeper::default())),
        CRYO_CELL => entity.insert((
            Appearance::new(CRYO_CELL, "celltop-p"),
            CryoCell::default(),
            PowerUser::new(Channel::Equipment, 100.0),
        )),
        _ => entity.insert((
            Appearance::new(SLEEPER_CONSOLE, "console"),
            SleeperConsole,
            PowerUser::new(Channel::Equipment, 40.0),
        )),
    };
    entity.id()
}

fn tell(world: &mut World, mob: Entity, id: &str) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(LocMsg::new(id));
    }
}

fn set_look(world: &mut World, entity: Entity, state: &str) {
    if let Some(mut look) = world.get_mut::<Appearance>(entity) {
        look.state = state.to_owned();
    }
}

fn pulled_person(world: &World, mob: Entity) -> Option<Entity> {
    world
        .get::<crate::pulling::Pulling>(mob)
        .map(|p| p.0)
        .filter(|p| world.get::<Body>(*p).is_some())
}

fn lay_inside(world: &mut World, mob: Entity, machine: Entity, person: Entity) -> bool {
    let Some(at) = world.get::<GridPos>(machine).copied() else {
        return false;
    };
    world.entity_mut(mob).remove::<crate::pulling::Pulling>();
    world.entity_mut(person).insert((InCloset(machine), Hidden, at));
    let name = |e: Entity| {
        world
            .get::<DisplayName>(e)
            .map(|n| n.0.clone())
            .unwrap_or_default()
    };
    let msg = LocMsg::new("medbay-put-in")
        .arg("name", name(mob))
        .arg("victim", name(person));
    world.resource_mut::<VisibleEvents>().0.push((at.0, msg, None));
    true
}

/// A click on a sleeper: someone pulled is laid inside; else whoever is in comes out.
pub fn click_sleeper(world: &mut World, mob: Entity, sleeper: Entity) {
    let occupant = world.get::<Sleeper>(sleeper).and_then(|s| s.occupant);
    if let Some(person) = pulled_person(world, mob) {
        if occupant.is_some() {
            tell(world, mob, "medbay-occupied");
            return;
        }
        if lay_inside(world, mob, sleeper, person) {
            if let Some(mut s) = world.get_mut::<Sleeper>(sleeper) {
                s.occupant = Some(person);
            }
            set_look(world, sleeper, "sleeper");
            tell(world, person, "sleeper-numb");
        }
    } else if occupant.is_some() {
        leave_sleeper(world, sleeper);
    }
}

fn leave_sleeper(world: &mut World, sleeper: Entity) {
    let Some(person) = world.get::<Sleeper>(sleeper).and_then(|s| s.occupant) else {
        return;
    };
    if let Ok(mut e) = world.get_entity_mut(person) {
        e.remove::<(InCloset, Hidden)>();
    }
    if let Some(mut s) = world.get_mut::<Sleeper>(sleeper) {
        s.occupant = None;
    }
    set_look(world, sleeper, "sleeper-open");
}

fn cryo_look(cell: &CryoCell) -> &'static str {
    match (cell.on, cell.occupant.is_some()) {
        (true, true) => "celltop_1",
        (true, false) => "celltop",
        _ => "celltop-p",
    }
}

/// A click on a cryo cell: someone pulled goes in, a beaker goes in, an empty hand turns it on
/// or off (Luna's window, as clicks).
pub fn click_cryo(world: &mut World, mob: Entity, cell: Entity) {
    let Some(state) = world.get::<CryoCell>(cell).copied() else {
        return;
    };
    if let Some(person) = pulled_person(world, mob) {
        if state.occupant.is_some() {
            tell(world, mob, "medbay-occupied");
            return;
        }
        if lay_inside(world, mob, cell, person)
            && let Some(mut c) = world.get_mut::<CryoCell>(cell)
        {
            c.occupant = Some(person);
        }
    } else if let Some(item) = world.get::<Hands>(mob).and_then(Hands::active_item) {
        let glass = world.get::<Appearance>(item).is_some_and(|a| is_glass(&a.sprite));
        if !glass {
            return;
        }
        if state.beaker.is_some() {
            tell(world, mob, "medbay-beaker-full");
            return;
        }
        if let Some(mut hands) = world.get_mut::<Hands>(mob) {
            let active = hands.active;
            *hands.slot_mut(active) = None;
        }
        world.entity_mut(item).remove::<HeldBy>().insert(InCloset(cell));
        if let Some(mut c) = world.get_mut::<CryoCell>(cell) {
            c.beaker = Some(item);
        }
    } else if let Some(mut c) = world.get_mut::<CryoCell>(cell) {
        c.on = !c.on;
        let id = if c.on { "cryo-on" } else { "cryo-off" };
        tell(world, mob, id);
    }
    if let Some(c) = world.get::<CryoCell>(cell).copied() {
        set_look(world, cell, cryo_look(&c));
    }
}

/// Luna `relaymove`: someone inside walking gets out, if awake. True if this was a medbay machine.
pub fn leave(world: &mut World, mob: Entity, machine: Entity) -> bool {
    let awake = world
        .get::<Vitals>(mob)
        .is_none_or(|v| v.stat == MobStat::Conscious);
    if world.get::<Sleeper>(machine).is_some() {
        if awake {
            leave_sleeper(world, machine);
        }
        return true;
    }
    if world.get::<CryoCell>(machine).is_some() {
        if awake {
            if let Ok(mut e) = world.get_entity_mut(mob) {
                e.remove::<(InCloset, Hidden)>();
            }
            if let Some(mut c) = world.get_mut::<CryoCell>(machine) {
                c.occupant = None;
            }
            if let Some(c) = world.get::<CryoCell>(machine).copied() {
                set_look(world, machine, cryo_look(&c));
            }
        }
        return true;
    }
    false
}

/// The sleeper a console works: the nearest close by.
fn sleeper_of(at: TilePos, sleepers: impl Iterator<Item = (Entity, TilePos)>) -> Option<Entity> {
    sleepers
        .filter(|(_, p)| p.z == at.z && (p.x - at.x).abs().max((p.y - at.y).abs()) <= LINK_RANGE)
        .min_by_key(|(e, p)| ((p.x - at.x).abs() + (p.y - at.y).abs(), e.index()))
        .map(|(e, _)| e)
}

/// Injections asked for, and what lying in a sleeper or cryo cell does (Luna `alter_health`,
/// `process_occupant`).
pub fn run_medbay(world: &mut World) {
    let mut askers = world.query::<(Entity, &GridPos, &mut PendingSleeperOps)>();
    let asked: Vec<(Entity, TilePos, Vec<(Entity, usize)>)> = askers
        .iter_mut(world)
        .filter(|(.., ops)| !ops.0.is_empty())
        .map(|(mob, at, mut ops)| (mob, at.0, std::mem::take(&mut ops.0)))
        .collect();
    for (mob, at, ops) in asked {
        for (console, line) in ops {
            inject(world, mob, at, console, line);
        }
    }
    if !world.resource::<WorldTime>().tick.0.is_multiple_of(MACHINE_TICKS) {
        return;
    }
    let mut sleepers = world.query::<&Sleeper>();
    let held: Vec<Entity> = sleepers.iter(world).filter_map(|s| s.occupant).collect();
    for person in held {
        // Luna tops up inaprovaline and eases the lungs of anyone with health left.
        if let Some(mut blood) = world.get_mut::<Reagents>(person)
            && blood.get("inaprovaline") < 5.0
        {
            blood.add("inaprovaline", 5.0);
        }
        if let Some(mut body) = world.get_mut::<Body>(person)
            && body.health() > 0.0
        {
            body.oxy = (body.oxy - if body.oxy >= 10.0 { 1.0 } else { 12.0 }).max(0.0);
        }
    }
    let mut cells = world.query::<(Entity, &GridPos, &CryoCell, &PowerUser)>();
    let running: Vec<(Entity, TilePos, CryoCell)> = cells
        .iter(world)
        .filter(|(_, _, c, power)| c.on && power.powered)
        .map(|(e, p, c, _)| (e, p.0, *c))
        .collect();
    for (entity, at, cell) in running {
        let (temperature, moles) = {
            let nets = world.resource::<PipeNets>();
            match nets
                .net_toward(at.step(Dir::South), side_bits(Dir::North))
                .map(|n| &nets.nets[n])
            {
                Some(net) => (net.mix.temperature, net.mix.total_moles()),
                None => continue,
            }
        };
        if moles < CRYO_MIN_MOLES {
            continue;
        }
        let Some(person) = cell.occupant else { continue };
        if world
            .get::<Vitals>(person)
            .is_some_and(|v| v.stat == MobStat::Dead)
        {
            continue;
        }
        treat(world, person, temperature);
        let due = cell.dose_in == 0;
        if let Some(mut c) = world.get_mut::<CryoCell>(entity) {
            c.dose_in = if due { CRYO_DOSE_CYCLES - 1 } else { c.dose_in - 1 };
        }
        if due && let Some(beaker) = cell.beaker {
            transfer(world, beaker, person, CRYO_DOSE);
        }
    }
}

/// The cold's work on someone in a cryo cell.
fn treat(world: &mut World, person: Entity, temperature: f64) {
    if temperature >= FREEZING {
        return;
    }
    let lucky: Vec<(Zone, bool, bool)> = if temperature < DEEP_COLD {
        let mut rng = world.resource_mut::<GameRng>();
        Zone::ALL
            .into_iter()
            .map(|z| (z, rng.prob(5.0), rng.prob(5.0)))
            .collect()
    } else {
        Vec::new()
    };
    let tox_chance = world.resource_mut::<GameRng>().prob(5.0);
    let has_cryoxadone = world
        .get::<Reagents>(person)
        .is_some_and(|r| r.get("cryoxadone") > 0.0);
    let Some(mut body) = world.get_mut::<Body>(person) else {
        return;
    };
    body.oxy = (body.oxy - 1.0).max(0.0);
    for (zone, brute, burn) in lucky {
        body.heal(zone, if brute { 1.0 } else { 0.0 }, if burn { 1.0 } else { 0.0 });
    }
    if temperature < DEEP_COLD && tox_chance {
        body.tox = (body.tox - 1.0).max(0.0);
    }
    // Cryoxadone only works this cold (Luna `bodytemperature < 170`).
    if has_cryoxadone && temperature < CRYOXADONE_COLD {
        body.oxy = (body.oxy - 3.0).max(0.0);
        body.tox = (body.tox - 3.0).max(0.0);
        for zone in Zone::ALL {
            body.heal(zone, 3.0 / Zone::ALL.len() as f32, 3.0 / Zone::ALL.len() as f32);
        }
    }
}

/// Luna `sleep_console/Topic`: in crit only inaprovaline, and nothing for the dying.
fn inject(world: &mut World, mob: Entity, at: TilePos, console: Entity, line: usize) {
    let Some(place) = world.get::<GridPos>(console).map(|p| p.0) else {
        return;
    };
    if !in_reach(at, place) || !world.get::<PowerUser>(console).is_some_and(|p| p.powered) {
        return;
    }
    let mut sleepers = world.query::<(Entity, &GridPos, &Sleeper)>();
    let found: Vec<(Entity, TilePos)> = sleepers.iter(world).map(|(e, p, _)| (e, p.0)).collect();
    let Some(sleeper) = sleeper_of(place, found.into_iter()) else {
        return;
    };
    let Some(person) = world.get::<Sleeper>(sleeper).and_then(|s| s.occupant) else {
        return;
    };
    let Some(&(reagent, amount)) = INJECTIONS.get(line) else {
        return;
    };
    let health = world.get::<Body>(person).map_or(0.0, Body::health);
    if health <= crate::health::DEATH_HEALTH {
        tell(world, mob, "sleeper-cant-metabolise");
        return;
    }
    if health <= 0.0 && reagent != "inaprovaline" {
        tell(world, mob, "sleeper-unstable");
        return;
    }
    if let Some(mut blood) = world.get_mut::<Reagents>(person)
        && blood.get(reagent) + amount <= INJECT_LIMIT
    {
        blood.add(reagent, amount);
    }
}

/// A click on a sleeper console opens its window.
pub fn click_console(world: &mut World, mob: Entity, console: Entity) {
    world.entity_mut(mob).insert(PanelViewer(console));
}

/// Show each player the sleeper console they have open.
#[allow(clippy::type_complexity)]
pub fn update_sleeper_views(
    mut viewers: Query<(
        Entity,
        &GridPos,
        Option<&PanelViewer>,
        &mut SleeperPanelOut,
        Option<&Vitals>,
    )>,
    consoles: Query<(&GridPos, &NetId), With<SleeperConsole>>,
    sleepers: Query<(Entity, &GridPos, &Sleeper)>,
    people: Query<(&Body, &Vitals, &Reagents, &DisplayName)>,
    mut commands: Commands,
) {
    for (mob, pos, viewer, mut out, vitals) in &mut viewers {
        let target = viewer.and_then(|v| consoles.get(v.0).ok());
        let view = target.and_then(|(at, id)| {
            let usable = in_reach(pos.0, at.0) && vitals.is_none_or(|v| v.stat == MobStat::Conscious);
            if !usable {
                return None;
            }
            let sleeper = sleeper_of(at.0, sleepers.iter().map(|(e, p, _)| (e, p.0)));
            let occupant = sleeper
                .and_then(|s| sleepers.get(s).ok())
                .and_then(|(.., s)| s.occupant);
            let person = occupant.and_then(|p| people.get(p).ok());
            Some(SleeperPanelView {
                target: *id,
                occupant: person.map(|(_, _, _, n)| n.0.clone()),
                stat: person.map_or(MobStat::Conscious, |(_, v, ..)| v.stat),
                health: person.map_or(0, |(b, ..)| b.health().round() as i16),
                oxy: person.map_or(0, |(b, ..)| b.oxy.round() as i16),
                tox: person.map_or(0, |(b, ..)| b.tox.round() as i16),
                reagents: INJECTIONS
                    .iter()
                    .map(|(r, _)| {
                        (
                            (*r).to_owned(),
                            person.map_or(0, |(_, _, blood, _)| blood.get(r).round() as u16),
                        )
                    })
                    .collect(),
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
