//! Disposals (Luna `recycling/disposal.dm`): a bin pumps itself up to two atmospheres from
//! the room's air, then flushes what was put in it down a trunk into pipes under the
//! floor; the bundle travels a tile per Luna tick, junctions choosing its way, up and down
//! between decks, until an outlet throws it out, or a gap in the pipes spills it.
//! Spec: `docs/reference/disposals.md`.

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_i18n::LocMsg;
use rs13_simulation::atmos::GasMix;
use rs13_simulation::atmos::gas::ONE_ATMOSPHERE;
use rs13_world::{Appearance, Dense, Dir, GridPos, NetId, ObjectSpec, TilePos};

use crate::actions::ClickOutcome;
use crate::atmos::StationAtmos;
use crate::decks::Decks;
use crate::hands::{Hands, HeldBy};
use crate::machines::MACHINE_TICKS;
use crate::power::{Channel, PowerUser};
use crate::rng::GameRng;
use crate::sound::SoundEvents;
use crate::time::WorldTime;

/// Prototypes.
pub const DISPOSAL_BIN: &str = "disposal-bin";
pub const DISPOSAL_PIPE: &str = "disposal-pipe";
pub const DISPOSAL_OUTLET: &str = "disposal-outlet";

/// The bin's air reservoir, L (Luna `0.125 × CELL_VOLUME`).
pub const BIN_VOLUME: f64 = 312.5;
/// It flushes at this pressure, kPa (Luna `2 × ONE_ATMOSPHERE`), and pumps toward a little
/// more, closing a tenth of the gap each machine cycle.
pub const FLUSH_PRESSURE: f64 = 2.0 * ONE_ATMOSPHERE;
pub const PUMP_TARGET: f64 = 2.1 * ONE_ATMOSPHERE;
/// Machine cycles after the last thing goes in before the handle engages by itself.
pub const AUTO_FLUSH_CYCLES: u8 = 5;
/// Power: always, and more while pumping, W (Luna `use_power(100)`, `+500`).
pub const IDLE_POWER: f64 = 100.0;
pub const PUMP_POWER: f64 = 500.0;
/// From engaging the flush to the bundle leaving, ds (Luna 10 + 5 ticks).
pub const FLUSH_DS: f32 = 15.0;
/// A bundle moves one tile per Luna tick, ds.
pub const STEP_DS: f32 = 1.0;
/// A bundle gives up after this many steps (Luna's loop guard).
pub const MAX_STEPS: u32 = 1000;
/// An outlet opens, buzzes, then throws, ds (Luna `sleep(20)`).
pub const OUTLET_DS: f32 = 20.0;
/// How far an outlet throws things, tiles (Luna throw range 3).
pub const OUTLET_THROW: i32 = 3;

use crate::sound::{DISPOSAL_FLUSH as SOUND_FLUSH, HISS as SOUND_HISS, WARNING_BUZZER as SOUND_BUZZER};

/// Which way gas and things go through a pipe: one of the four, or between decks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Way {
    Flat(Dir),
    Up,
    Down,
}

impl Way {
    fn reverse(self) -> Way {
        match self {
            Way::Flat(d) => Way::Flat(d.opposite()),
            Way::Up => Way::Down,
            Way::Down => Way::Up,
        }
    }

    fn step(self, at: TilePos) -> TilePos {
        match self {
            Way::Flat(d) => at.step(d),
            Way::Up => TilePos::new(at.x, at.y, at.z + 1),
            Way::Down => TilePos::new(at.x, at.y, at.z - 1),
        }
    }
}

fn clockwise(dir: Dir) -> Dir {
    match dir {
        Dir::North => Dir::East,
        Dir::East => Dir::South,
        Dir::South => Dir::West,
        _ => Dir::North,
    }
}

fn counter_clockwise(dir: Dir) -> Dir {
    clockwise(dir).opposite()
}

/// A disposal pipe's shape (Luna's icon states).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipeKind {
    /// `pipe-s`: through.
    Straight,
    /// `pipe-c`: a bend.
    Bent,
    /// `pipe-j1`, `pipe-j2` (mirrored), `pipe-y`: junctions.
    Junction1,
    Junction2,
    JunctionY,
    /// `pipe-t`: the end under a bin or outlet.
    Trunk,
    /// `pipe-u`, `pipe-d`: up or down to the next deck.
    Up,
    Down,
    /// `pipe-b`: connects nothing.
    Broken,
}

impl PipeKind {
    pub fn from_state(state: &str) -> Self {
        match state {
            "pipe-c" => PipeKind::Bent,
            "pipe-j1" => PipeKind::Junction1,
            "pipe-j2" => PipeKind::Junction2,
            "pipe-y" => PipeKind::JunctionY,
            "pipe-t" => PipeKind::Trunk,
            "pipe-u" => PipeKind::Up,
            "pipe-d" => PipeKind::Down,
            "pipe-b" => PipeKind::Broken,
            _ => PipeKind::Straight,
        }
    }

    pub fn state(self) -> &'static str {
        match self {
            PipeKind::Straight => "pipe-s",
            PipeKind::Bent => "pipe-c",
            PipeKind::Junction1 => "pipe-j1",
            PipeKind::Junction2 => "pipe-j2",
            PipeKind::JunctionY => "pipe-y",
            PipeKind::Trunk => "pipe-t",
            PipeKind::Up => "pipe-u",
            PipeKind::Down => "pipe-d",
            PipeKind::Broken => "pipe-b",
        }
    }
}

/// A disposal pipe: its shape and the way it faces.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisposalPipe {
    pub kind: PipeKind,
    pub dir: Dir,
}

impl DisposalPipe {
    /// The ways it connects (Luna `dpdir`).
    pub fn ways(&self) -> Vec<Way> {
        let d = self.dir;
        let flat = |ds: &[Dir]| ds.iter().map(|d| Way::Flat(*d)).collect();
        match self.kind {
            PipeKind::Straight => flat(&[d, d.opposite()]),
            PipeKind::Bent => flat(&[d, clockwise(d)]),
            PipeKind::Junction1 => flat(&[d, clockwise(d), d.opposite()]),
            PipeKind::Junction2 => flat(&[d, counter_clockwise(d), d.opposite()]),
            PipeKind::JunctionY => flat(&[d, clockwise(d), counter_clockwise(d)]),
            PipeKind::Trunk => flat(&[d]),
            PipeKind::Up => vec![Way::Flat(d), Way::Up],
            PipeKind::Down => vec![Way::Flat(d), Way::Down],
            PipeKind::Broken => Vec::new(),
        }
    }

    /// Which way a bundle that came in from `from` (the side it entered by) leaves. Luna
    /// `nextdir`: a junction sends everything from a branch out its main way, and what
    /// comes in by the main way down either branch, half the time each; a deck crossing
    /// turns flat to vertical and back; anything else goes out the way it didn't come in.
    pub fn next(&self, from: Way, rng: &mut GameRng) -> Option<Way> {
        let others: Vec<Way> = self.ways().into_iter().filter(|w| *w != from).collect();
        match self.kind {
            PipeKind::Junction1 | PipeKind::Junction2 | PipeKind::JunctionY => {
                if from != Way::Flat(self.dir) {
                    Some(Way::Flat(self.dir))
                } else {
                    others
                        .get(rng.range(0, others.len() as i32 - 1) as usize)
                        .copied()
                }
            }
            _ => others.first().copied(),
        }
    }
}

/// The bin's pump.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pump {
    Off,
    Charging,
    Ready,
}

/// A disposal bin.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct DisposalBin {
    pub pump: Pump,
    /// The flush handle is engaged (Luna `flush`).
    pub handle: bool,
    /// Flushing until this time, ds.
    pub flushing_until: Option<f32>,
    /// Machine cycles until the handle engages by itself, while it holds anything.
    pub countdown: u8,
    pub air: GasMix,
    pub contents: Vec<Entity>,
}

/// A disposal outlet: what has arrived and when it goes out.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct DisposalOutlet {
    pub dir: Dir,
    pub arriving: Vec<(Vec<Entity>, f32)>,
}

/// On something inside a bin or travelling the pipes.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InDisposal;

/// A bundle travelling the pipes.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Bundle {
    pub at: TilePos,
    /// The way it last moved; `None` just after it left its bin (Luna's direction 64).
    pub heading: Option<Way>,
    pub contents: Vec<Entity>,
    pub air: GasMix,
    pub next_step: f32,
    pub steps: u32,
}

pub fn spawn_disposal(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    let dir = match spec.dir {
        Some(d @ (Dir::North | Dir::South | Dir::East | Dir::West)) => d,
        _ => Dir::South,
    };
    match spec.id.as_str() {
        DISPOSAL_BIN => world
            .spawn((
                GridPos(pos),
                id,
                Appearance::new(DISPOSAL_BIN, "disposal"),
                DisposalBin {
                    pump: Pump::Charging,
                    handle: false,
                    flushing_until: None,
                    countdown: 0,
                    air: GasMix::default(),
                    contents: Vec::new(),
                },
                PowerUser::new(Channel::Equipment, IDLE_POWER + PUMP_POWER),
                Dense,
            ))
            .id(),
        DISPOSAL_OUTLET => world
            .spawn((
                GridPos(pos),
                id,
                Appearance::new(DISPOSAL_OUTLET, outlet_look(dir, false)),
                DisposalOutlet {
                    dir,
                    arriving: Vec::new(),
                },
                Dense,
            ))
            .id(),
        _ => {
            let pipe = DisposalPipe {
                kind: PipeKind::from_state(spec.state.as_deref().unwrap_or("pipe-s")),
                dir,
            };
            world
                .spawn((
                    GridPos(pos),
                    id,
                    Appearance::new(DISPOSAL_PIPE, pipe_look(&pipe)),
                    pipe,
                ))
                .id()
        }
    }
}

fn dir_name(dir: Dir) -> &'static str {
    match dir {
        Dir::North => "north",
        Dir::East => "east",
        Dir::West => "west",
        _ => "south",
    }
}

/// A pipe's sprite state: its shape and the way it faces.
pub fn pipe_look(pipe: &DisposalPipe) -> String {
    format!("{}-{}", pipe.kind.state(), dir_name(pipe.dir))
}

fn outlet_look(dir: Dir, open: bool) -> String {
    format!(
        "{}-{}",
        if open { "outlet-open" } else { "outlet" },
        dir_name(dir)
    )
}

/// A bin's sprite: `disposal-flush` while flushing, else `disposal` with Luna's overlays.
pub fn bin_look(bin: &DisposalBin, powered: bool) -> String {
    if bin.flushing_until.is_some() {
        return "disposal-flush".to_owned();
    }
    let mut state = "disposal".to_owned();
    if bin.handle {
        state.push_str("+dispover-handle");
    }
    if !powered {
        return state;
    }
    if !bin.contents.is_empty() {
        state.push_str("+dispover-full");
    }
    match bin.pump {
        Pump::Charging => state.push_str("+dispover-charge"),
        Pump::Ready => state.push_str("+dispover-ready"),
        Pump::Off => {}
    }
    state
}

/// Everything clicking a bin needs.
#[derive(SystemParam)]
pub struct DisposalWork<'w, 's> {
    bins: Query<'w, 's, &'static mut DisposalBin>,
}

impl DisposalWork<'_, '_> {
    pub fn contains(&self, target: Entity) -> bool {
        self.bins.contains(target)
    }

    /// Luna `disposal/attackby`: anything in the hand goes into the bin (and it will flush
    /// itself soon after), which everyone around sees. An empty hand works the flush
    /// handle (Luna's window has the handle; not built yet). Nothing happens mid-flush.
    /// The caller checks reach.
    pub fn use_on(
        &mut self,
        target: Entity,
        hands: &mut Hands,
        held: Option<&str>,
        commands: &mut Commands,
    ) -> Option<(ClickOutcome, Option<LocMsg>)> {
        let mut bin = self.bins.get_mut(target).ok()?;
        if bin.flushing_until.is_some() {
            return Some((ClickOutcome::default(), None));
        }
        let Some(item) = hands.active_item() else {
            bin.handle = !bin.handle;
            return Some((ClickOutcome::default(), None));
        };
        let name = held.unwrap_or_default().to_owned();
        *hands.slot_mut(hands.active) = None;
        commands.entity(item).remove::<HeldBy>().insert(InDisposal);
        bin.contents.push(item);
        bin.countdown = AUTO_FLUSH_CYCLES;
        Some((
            ClickOutcome {
                to_mob: Some(
                    LocMsg::new("disposal-placed-self")
                        .arg("item", name.clone())
                        .arg("bin", DISPOSAL_BIN),
                ),
                seen: None,
            },
            Some(
                LocMsg::new("disposal-placed")
                    .arg("item", name)
                    .arg("bin", DISPOSAL_BIN),
            ),
        ))
    }
}

/// Put `things` back on the map at `at`.
fn release(commands: &mut Commands, things: &[Entity], at: TilePos) {
    for thing in things {
        commands.entity(*thing).remove::<InDisposal>().insert(GridPos(at));
    }
}

/// Spill `things` and `air` at `at` (Luna `expel`): they land there, the gas goes into the
/// room with a hiss.
fn spill(
    commands: &mut Commands,
    atmos: Option<&mut StationAtmos>,
    sounds: &mut SoundEvents,
    things: &[Entity],
    air: &GasMix,
    at: TilePos,
) {
    release(commands, things, at);
    if let Some(atmos) = atmos {
        atmos.0.add_at(at, air);
    }
    sounds.play(SOUND_HISS, at, 50.0);
}

/// Every machine cycle a bin counts down to flushing itself and pumps air in from its room;
/// at two atmospheres with the handle engaged it flushes, even without power. Every tick it
/// finishes flushing, launching what it held down its trunk.
#[allow(clippy::too_many_arguments)]
pub fn run_bins(
    time: Res<WorldTime>,
    mut commands: Commands,
    mut atmos: Option<ResMut<StationAtmos>>,
    mut sounds: ResMut<SoundEvents>,
    mut bins: Query<(&GridPos, &mut DisposalBin, &mut PowerUser, &mut Appearance)>,
    pipes: Query<(&GridPos, &DisposalPipe)>,
) {
    let now = time.deciseconds();
    let cycle = time.tick.0.is_multiple_of(MACHINE_TICKS);
    for (pos, mut bin, mut power, mut look) in &mut bins {
        if let Some(until) = bin.flushing_until {
            if now >= until {
                bin.flushing_until = None;
                bin.handle = false;
                if bin.pump == Pump::Ready {
                    bin.pump = Pump::Charging;
                }
                let contents = std::mem::take(&mut bin.contents);
                let air = std::mem::take(&mut bin.air);
                let trunk = pipes
                    .iter()
                    .any(|(p, pipe)| p.0 == pos.0 && pipe.kind == PipeKind::Trunk);
                if trunk {
                    commands.spawn(Bundle {
                        at: pos.0,
                        heading: None,
                        contents,
                        air,
                        next_step: now + STEP_DS,
                        steps: 0,
                    });
                } else {
                    spill(
                        &mut commands,
                        atmos.as_deref_mut(),
                        &mut sounds,
                        &contents,
                        &air,
                        pos.0,
                    );
                }
            }
        } else if cycle {
            if !bin.contents.is_empty() {
                if bin.countdown == 0 {
                    bin.handle = true;
                } else {
                    bin.countdown -= 1;
                }
            }
            if bin.handle && bin.air.pressure(BIN_VOLUME) >= FLUSH_PRESSURE {
                bin.flushing_until = Some(now + FLUSH_DS);
                sounds.play(SOUND_FLUSH, pos.0, 50.0);
            } else if power.powered && bin.pump == Pump::Charging {
                // Close a tenth of the gap to the pump's target from the room's air.
                if let Some(atmos) = atmos.as_deref_mut() {
                    let room = atmos.0.air_at(pos.0);
                    if room.temperature > 0.0 {
                        let gap = PUMP_TARGET - bin.air.pressure(BIN_VOLUME);
                        let moles =
                            0.1 * gap * BIN_VOLUME / (room.temperature * rs13_simulation::atmos::gas::R);
                        if moles > 0.0 {
                            let taken = atmos.0.remove_at(pos.0, moles);
                            bin.air.merge(&taken);
                        }
                    }
                }
                if bin.air.pressure(BIN_VOLUME) >= FLUSH_PRESSURE {
                    bin.pump = Pump::Ready;
                }
            }
        }
        let watts = IDLE_POWER
            + if bin.pump == Pump::Charging {
                PUMP_POWER
            } else {
                0.0
            };
        if power.watts != watts {
            power.watts = watts;
        }
        let state = bin_look(&bin, power.powered);
        if look.state != state {
            look.state = state;
        }
    }
}

/// Move every bundle on a tile per Luna tick along its pipes: to the next tile its pipe
/// sends it, onto a pipe there that connects back the way it came, or out at a gap. At a
/// trunk it is handed to the outlet there, or spilled.
#[allow(clippy::too_many_arguments)]
pub fn run_bundles(
    time: Res<WorldTime>,
    mut commands: Commands,
    mut rng: ResMut<GameRng>,
    mut atmos: Option<ResMut<StationAtmos>>,
    mut sounds: ResMut<SoundEvents>,
    decks: Decks,
    mut bundles: Query<(Entity, &mut Bundle)>,
    pipes: Query<(&GridPos, &DisposalPipe)>,
    mut outlets: Query<(&GridPos, &mut DisposalOutlet)>,
) {
    let now = time.deciseconds();
    if bundles.is_empty() {
        return;
    }
    let mut at: HashMap<TilePos, Vec<DisposalPipe>> = HashMap::new();
    for (pos, pipe) in &pipes {
        at.entry(pos.0).or_default().push(*pipe);
    }
    for (entity, mut bundle) in &mut bundles {
        if now < bundle.next_step {
            continue;
        }
        bundle.next_step = now + STEP_DS;
        bundle.steps += 1;
        // The pipe it is in, joined the way it came.
        let from = bundle.heading.map(Way::reverse);
        let here = at.get(&bundle.at).and_then(|ps| {
            ps.iter()
                .find(|p| from.is_none_or(|f| p.ways().contains(&f)))
                .copied()
        });
        let Some(pipe) = here else {
            spill(
                &mut commands,
                atmos.as_deref_mut(),
                &mut sounds,
                &bundle.contents,
                &bundle.air,
                bundle.at,
            );
            commands.entity(entity).despawn();
            continue;
        };
        // At a trunk, arriving: out through the outlet on it, or spilled.
        if pipe.kind == PipeKind::Trunk && bundle.heading.is_some() {
            let contents = std::mem::take(&mut bundle.contents);
            match outlets.iter_mut().find(|(p, _)| p.0 == bundle.at) {
                Some((_, mut outlet)) => {
                    sounds.play(SOUND_BUZZER, bundle.at, 50.0);
                    outlet.arriving.push((contents, now + OUTLET_DS));
                }
                None => spill(
                    &mut commands,
                    atmos.as_deref_mut(),
                    &mut sounds,
                    &contents,
                    &bundle.air,
                    bundle.at,
                ),
            }
            commands.entity(entity).despawn();
            continue;
        }
        let leaving = match from {
            None => pipe.ways().first().copied(),
            Some(f) => pipe.next(f, &mut rng),
        };
        let Some(way) = leaving.filter(|_| bundle.steps < MAX_STEPS) else {
            continue;
        };
        let next = way.step(bundle.at);
        let joins = at
            .get(&next)
            .is_some_and(|ps| ps.iter().any(|p| p.ways().contains(&way.reverse())));
        if joins {
            bundle.at = next;
            bundle.heading = Some(way);
        } else if decks.is_solid(next) || decks.deck(next.z).is_none() {
            // Into a wall: stuck in the last pipe (Luna leaves it there).
            continue;
        } else {
            spill(
                &mut commands,
                atmos.as_deref_mut(),
                &mut sounds,
                &bundle.contents,
                &bundle.air,
                next,
            );
            commands.entity(entity).despawn();
        }
    }
}

/// An outlet opens and buzzes when something arrives, then throws it out the way it faces,
/// up to three tiles, as far as nothing stops it (Luna `disposaloutlet/expel`).
pub fn run_outlets(
    time: Res<WorldTime>,
    mut commands: Commands,
    mut sounds: ResMut<SoundEvents>,
    decks: Decks,
    blockers: Query<&GridPos, With<Dense>>,
    mut outlets: Query<(&GridPos, &mut DisposalOutlet, &mut Appearance)>,
) {
    let now = time.deciseconds();
    for (pos, mut outlet, mut look) in &mut outlets {
        let open = !outlet.arriving.is_empty();
        let state = outlet_look(outlet.dir, open);
        if look.state != state {
            look.state = state;
        }
        let (due, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut outlet.arriving)
            .into_iter()
            .partition(|(_, at)| now >= *at);
        outlet.arriving = waiting;
        for (things, _) in due {
            sounds.play(SOUND_HISS, pos.0, 50.0);
            let mut land = pos.0;
            for _ in 0..OUTLET_THROW {
                let next = land.step(outlet.dir);
                if decks.is_solid(next) || blockers.iter().any(|b| b.0 == next) {
                    break;
                }
                land = next;
            }
            release(&mut commands, &things, land);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pipe(kind: PipeKind, dir: Dir) -> DisposalPipe {
        DisposalPipe { kind, dir }
    }

    #[test]
    fn pipes_join_the_ways_luna_draws_them() {
        assert_eq!(
            pipe(PipeKind::Straight, Dir::North).ways(),
            [Way::Flat(Dir::North), Way::Flat(Dir::South)]
        );
        assert_eq!(
            pipe(PipeKind::Bent, Dir::North).ways(),
            [Way::Flat(Dir::North), Way::Flat(Dir::East)]
        );
        assert_eq!(pipe(PipeKind::Trunk, Dir::West).ways(), [Way::Flat(Dir::West)]);
        assert_eq!(
            pipe(PipeKind::Down, Dir::East).ways(),
            [Way::Flat(Dir::East), Way::Down]
        );
    }

    #[test]
    fn a_junction_sends_branches_out_its_main_way_and_splits_the_main_way() {
        let mut rng = GameRng::default();
        let j = pipe(PipeKind::Junction1, Dir::North);
        // In from the east branch: out north.
        assert_eq!(
            j.next(Way::Flat(Dir::East), &mut rng),
            Some(Way::Flat(Dir::North))
        );
        // In from the north: east or south.
        let mut seen = std::collections::HashSet::new();
        for _ in 0..50 {
            seen.insert(j.next(Way::Flat(Dir::North), &mut rng));
        }
        assert_eq!(seen.len(), 2, "{seen:?}");
        assert!(!seen.contains(&Some(Way::Flat(Dir::North))));
    }

    #[test]
    fn the_bin_shows_its_handle_contents_and_pump() {
        let mut bin = DisposalBin {
            pump: Pump::Charging,
            handle: true,
            flushing_until: None,
            countdown: 0,
            air: GasMix::default(),
            contents: vec![Entity::PLACEHOLDER],
        };
        assert_eq!(
            bin_look(&bin, true),
            "disposal+dispover-handle+dispover-full+dispover-charge"
        );
        assert_eq!(bin_look(&bin, false), "disposal+dispover-handle");
        bin.flushing_until = Some(1.0);
        assert_eq!(bin_look(&bin, true), "disposal-flush");
    }
}
