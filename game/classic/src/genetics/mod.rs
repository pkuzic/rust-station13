//! Genetics (Luna `game/dna.dm`, `machinery/dnaconsole_new.dm`, `obj/injector.dm`): everyone's
//! genes, the DNA modifier console that reads and irradiates whoever lies in its scanner, its
//! buffers and injectors, and the powers and defects the genes give. Spec:
//! `docs/reference/genetics.md`.

pub mod dna;
pub mod powers;

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{
    Appearance, Dense, DisplayName, GeneBuffer, GeneOccupant, GeneOp, GenePage, GeneticsPanelView, GridPos,
    HumanLook, MobStat, NetId, TilePos,
};

use crate::actions::{Outbox, in_reach};
use crate::chemistry::Reagents;
use crate::cloning::DnaScanner;
use crate::health::{Body, Vitals};
use crate::power::{Channel, PowerUser};
use crate::rng::GameRng;
use crate::time::WorldTime;
use crate::wires::PanelViewer;

use dna::{BLOCK_MAX, SE_BLOCKS, UI_BLOCKS};
pub use powers::{Gene, Mutations};

pub const CONSOLE: &str = "genetics-console";
pub const INJECTOR: &str = "dna-injector";
/// The emitter's settings (Luna `radduration` 2 to 20 by twos, `radstrength` 1 to 10).
pub const MIN_DURATION: u8 = 2;
pub const MAX_DURATION: u8 = 20;
pub const MAX_STRENGTH: u8 = 10;
/// The replicator warms up 25 s after the console is built, then rests 20 s after each injector.
pub const INJECTOR_WARMUP_DS: f32 = 250.0;
pub const INJECTOR_REST_DS: f32 = 200.0;
/// Inaprovaline per press, and the most the console tops someone up to.
pub const INAPROVALINE_DOSE: f32 = 10.0;
pub const INAPROVALINE_LIMIT: f32 = 50.0;

/// Someone's genes.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct Genome {
    /// The unique identifier: how they look.
    pub ui: String,
    /// The structural enzymes: their powers and defects.
    pub se: String,
    /// The unique enzymes: who they are.
    pub ue: String,
}

/// How radiated someone is (Luna `radiation`).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct Radiation(pub f32);

/// Which block of the structural enzymes is each gene's, and how far this round's thresholds
/// are shifted (Luna `setupgenetics`, `BLOCKADD`).
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct GeneLayout {
    pub block_add: i32,
    pub blocks: Vec<(Gene, usize)>,
}

impl GeneLayout {
    pub fn new(rng: &mut GameRng) -> Self {
        let block_add = if rng.prob(50.0) { rng.range(-300, 100) } else { 0 };
        // Every block but the monkey's, shuffled out to the genes.
        let mut free: Vec<usize> = (1..=SE_BLOCKS).filter(|b| *b != dna::MONKEY_BLOCK).collect();
        let mut blocks = Vec::new();
        for gene in Gene::ALL {
            let i = rng.range(0, free.len() as i32 - 1) as usize;
            blocks.push((gene, free.remove(i)));
        }
        Self { block_add, blocks }
    }

    pub fn block(&self, gene: Gene) -> usize {
        self.blocks
            .iter()
            .find(|(g, _)| *g == gene)
            .map_or(1, |(_, b)| *b)
    }
}

/// This round's layout, made the first time it's asked for.
pub fn layout(world: &mut World) -> GeneLayout {
    if let Some(layout) = world.get_resource::<GeneLayout>() {
        return layout.clone();
    }
    let layout = GeneLayout::new(&mut world.resource_mut::<GameRng>());
    world.insert_resource(layout.clone());
    layout
}

/// A DNA modifier console and its settings.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct GeneticsConsole {
    pub page: GenePage,
    pub ui_block: u8,
    pub se_block: u8,
    pub sub: u8,
    pub target_digit: u8,
    pub duration: u8,
    pub strength: u8,
    pub buffers: [Option<GeneBuffer>; 3],
    /// When the replicator is ready, ds.
    pub injector_at: f32,
    /// The beam on: when it's done, and what it does then.
    pub working: Option<(f32, GeneOp)>,
}

impl Default for GeneticsConsole {
    fn default() -> Self {
        Self {
            page: GenePage::Main,
            ui_block: 1,
            se_block: 1,
            sub: 1,
            target_digit: 1,
            duration: MIN_DURATION,
            strength: 1,
            buffers: [None, None, None],
            injector_at: INJECTOR_WARMUP_DS,
            working: None,
        }
    }
}

/// What a DNA injector carries (Luna `dnainjector`): a whole string, or one block of one.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct Injector {
    pub se: bool,
    pub data: String,
    /// The one block it writes, from 1; `None` for the whole string.
    pub block: Option<usize>,
    /// The name it gives, with an identifier saved with its enzymes.
    pub ue: Option<String>,
    pub used: bool,
}

/// What a mob's open DNA console window shows.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct GeneticsPanelOut(pub Option<GeneticsPanelView>);

pub fn spawn_console(world: &mut World, pos: TilePos, id: NetId) -> Entity {
    let ready = world.get_resource::<WorldTime>().map_or(0.0, |t| t.deciseconds()) + INJECTOR_WARMUP_DS;
    world
        .spawn((
            GridPos(pos),
            id,
            Dense,
            Appearance::new(CONSOLE, "scanner"),
            GeneticsConsole {
                injector_at: ready,
                ..Default::default()
            },
            PowerUser::new(Channel::Equipment, 250.0),
        ))
        .id()
}

pub(crate) fn tell(world: &mut World, mob: Entity, msg: LocMsg) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(msg);
    }
}

/// A person, whether or not they chose how they look (a female disguise is drawn as `human-f`).
pub fn is_person(world: &World, mob: Entity) -> bool {
    world.get::<HumanLook>(mob).is_some()
        || world
            .get::<Appearance>(mob)
            .is_some_and(|a| a.sprite.starts_with("human"))
}

/// Someone's genes, made from how they look the first time they're needed (Luna `ready_dna`).
/// Monkeys have them too, with the monkey block on; those who aren't people don't.
pub fn genome(world: &mut World, mob: Entity) -> Option<Genome> {
    if let Some(g) = world.get::<Genome>(mob) {
        return Some(g.clone());
    }
    let look = world.get::<HumanLook>(mob).copied();
    let sprite = world.get::<Appearance>(mob).map(|a| a.sprite.clone());
    let monkey = sprite.as_deref() == Some(crate::monkeys::MONKEY);
    // Every person has genes, whether or not they chose how they look.
    let human = is_person(world, mob);
    if !human && !monkey {
        return None;
    }
    let name = world
        .get::<DisplayName>(mob)
        .map_or_else(String::new, |n| n.0.clone());
    let layout = layout(world);
    let mut rng = world.resource_mut::<GameRng>();
    let genome = Genome {
        ui: dna::identity_of(&look.unwrap_or_default(), layout.block_add, &mut rng),
        se: dna::fresh_enzymes(monkey && !human, &mut rng),
        ue: dna::unique_enzymes(&name),
    };
    world.entity_mut(mob).insert(genome.clone());
    Some(genome)
}

fn set_genome(world: &mut World, mob: Entity, genome: Genome) {
    world.entity_mut(mob).insert(genome);
}

pub fn irradiate(world: &mut World, mob: Entity, amount: f32) {
    let now = world.get::<Radiation>(mob).map_or(0.0, |r| r.0);
    world.entity_mut(mob).insert(Radiation(now + amount));
}

/// Luna `randmutb`: one of the blocks of defects (or a few unused ones) flipped.
pub fn random_bad(world: &mut World, mob: Entity) {
    let layout = layout(world);
    let Some(mut g) = genome(world, mob) else {
        return;
    };
    let pool = [
        1,
        3,
        layout.block(Gene::Fake),
        5,
        layout.block(Gene::Clumsy),
        7,
        9,
        layout.block(Gene::Blind),
        layout.block(Gene::Deaf),
    ];
    flip_one(world, &mut g, &pool);
    set_genome(world, mob, g);
}

/// Luna `randmutg`: one of the great powers' blocks flipped.
pub fn random_good(world: &mut World, mob: Entity) {
    let layout = layout(world);
    let Some(mut g) = genome(world, mob) else {
        return;
    };
    let pool = [Gene::Hulk, Gene::Xray, Gene::ColdResist, Gene::Telekinesis].map(|g| layout.block(g));
    flip_one(world, &mut g, &pool);
    set_genome(world, mob, g);
}

fn flip_one(world: &mut World, g: &mut Genome, pool: &[usize]) {
    let mut rng = world.resource_mut::<GameRng>();
    let n = pool[rng.range(0, pool.len() as i32 - 1) as usize];
    let value = dna::toggled(dna::block(&g.se, n), &mut rng);
    dna::set_block(&mut g.se, n, &dna::hex3(value));
}

/// Luna `randmuti`: one block of the identifier set anew.
pub fn random_identity(world: &mut World, mob: Entity) {
    let Some(mut g) = genome(world, mob) else {
        return;
    };
    let mut rng = world.resource_mut::<GameRng>();
    let n = rng.range(1, UI_BLOCKS as i32) as usize;
    let value = rng.range(1, BLOCK_MAX);
    dna::set_block(&mut g.ui, n, &dna::hex3(value));
    set_genome(world, mob, g);
}

/// Luna `scramble`: each block, at this chance, set anew; then they change to suit.
pub fn scramble(world: &mut World, mob: Entity, identity: bool, chance: f32) {
    let Some(mut g) = genome(world, mob) else {
        return;
    };
    {
        let mut rng = world.resource_mut::<GameRng>();
        let genes = if identity { &mut g.ui } else { &mut g.se };
        for n in 1..=UI_BLOCKS {
            if rng.prob(chance) {
                let value = rng.range(1, BLOCK_MAX);
                dna::set_block(genes, n, &dna::hex3(value));
            }
        }
    }
    set_genome(world, mob, g);
    if identity {
        powers::update_look(world, mob);
    } else {
        powers::check(world, mob, false);
    }
}

/// The scanner a console works: one beside it, on any side (Luna `New`'s search).
pub fn scanner_of(world: &mut World, console: Entity) -> Option<Entity> {
    let at = world.get::<GridPos>(console)?.0;
    let mut q = world.query_filtered::<(Entity, &GridPos), With<DnaScanner>>();
    let mut found: Vec<(Entity, TilePos)> = q.iter(world).map(|(e, p)| (e, p.0)).collect();
    found.sort_by_key(|(e, _)| *e);
    // North, east, south, west.
    [(0, 1), (1, 0), (0, -1), (-1, 0)].iter().find_map(|(dx, dy)| {
        found
            .iter()
            .find(|(_, p)| p.z == at.z && p.x == at.x + dx && p.y == at.y + dy)
            .map(|(e, _)| *e)
    })
}

fn occupant_of(world: &World, scanner: Option<Entity>) -> Option<Entity> {
    scanner
        .and_then(|s| world.get::<DnaScanner>(s))
        .and_then(|s| s.occupant)
}

/// A click on a DNA console opens its window.
pub fn click_console(world: &mut World, mob: Entity, console: Entity) {
    if !world.get::<PowerUser>(console).is_some_and(|p| p.powered) {
        return;
    }
    world.entity_mut(mob).insert(PanelViewer(console));
}

/// The buttons pressed on DNA consoles this tick (Luna `scan_consolenew/Topic`).
pub fn console_pick(world: &mut World, mob: Entity, console: Entity, line: usize) {
    let Some(op) = u16::try_from(line).ok().and_then(GeneOp::from_line) else {
        return;
    };
    let (Some(at), Some(place)) = (
        world.get::<GridPos>(mob).map(|p| p.0),
        world.get::<GridPos>(console).map(|p| p.0),
    ) else {
        return;
    };
    if !in_reach(at, place) || !world.get::<PowerUser>(console).is_some_and(|p| p.powered) {
        return;
    }
    let Some(mut c) = world.get::<GeneticsConsole>(console).cloned() else {
        return;
    };
    // While the beam is on, the window only waits.
    if c.working.is_some() {
        return;
    }
    let scanner = scanner_of(world, console);
    let Some(scanner) = scanner else {
        return;
    };
    let occupant = occupant_of(world, Some(scanner));
    let readable = occupant
        .filter(|o| world.get::<crate::changeling::Husked>(*o).is_none() && genome(world, *o).is_some());
    let now = world.resource::<WorldTime>().deciseconds();
    match op {
        GeneOp::Main => c.page = GenePage::Main,
        GeneOp::Identity => c.page = page_or_none(readable, GenePage::Identity),
        GeneOp::Enzymes => c.page = page_or_none(readable, GenePage::Enzymes),
        GeneOp::Buffers => c.page = GenePage::Buffers,
        GeneOp::Emitter => c.page = GenePage::Emitter,
        GeneOp::Lock => {
            if occupant.is_some()
                && let Some(mut s) = world.get_mut::<DnaScanner>(scanner)
            {
                s.locked = !s.locked;
            }
        }
        GeneOp::DurationUp => c.duration = (c.duration + 2).min(MAX_DURATION),
        GeneOp::DurationDown => c.duration = c.duration.saturating_sub(2).max(MIN_DURATION),
        GeneOp::StrengthUp => c.strength = (c.strength + 1).min(MAX_STRENGTH),
        GeneOp::StrengthDown => c.strength = c.strength.saturating_sub(1).max(1),
        GeneOp::Block(forward) => {
            if c.page == GenePage::Identity {
                c.ui_block = wrap(c.ui_block, forward, UI_BLOCKS as u8);
            } else {
                c.se_block = wrap(c.se_block, forward, SE_BLOCKS as u8);
            }
        }
        GeneOp::Sub(forward) => c.sub = wrap(c.sub, forward, 3),
        GeneOp::Target(forward) => {
            c.target_digit = if forward {
                (c.target_digit + 1) % 16
            } else {
                (c.target_digit + 15) % 16
            };
        }
        GeneOp::Select(block, sub) => {
            let most = if c.page == GenePage::Identity {
                UI_BLOCKS
            } else {
                SE_BLOCKS
            };
            if (1..=most as u8).contains(&block) && (1..=3).contains(&sub) {
                if c.page == GenePage::Identity {
                    c.ui_block = block;
                } else {
                    c.se_block = block;
                }
                c.sub = sub;
            }
        }
        GeneOp::Pulse | GeneOp::Irradiate => {
            if readable.is_none() {
                c.page = GenePage::Note("genetics-no-occupant".into());
            } else {
                let op = if op == GeneOp::Irradiate {
                    match c.page {
                        GenePage::Identity => GeneOp::Identity,
                        GenePage::Enzymes => GeneOp::Enzymes,
                        _ => return,
                    }
                } else {
                    GeneOp::Pulse
                };
                c.working = Some((now + f32::from(c.duration) * 10.0, op));
                c.page = GenePage::Working(c.duration);
                lock_for_beam(world, scanner, true);
            }
        }
        GeneOp::Inject => {
            if let Some(person) = readable {
                let had = world
                    .get::<Reagents>(person)
                    .map_or(0.0, |r| r.get("inaprovaline"));
                let now_has = if had < INAPROVALINE_LIMIT {
                    if let Some(mut blood) = world.get_mut::<Reagents>(person) {
                        blood.add("inaprovaline", INAPROVALINE_DOSE);
                    }
                    had + INAPROVALINE_DOSE
                } else {
                    had
                };
                tell(
                    world,
                    mob,
                    LocMsg::new("genetics-inaprovaline").num("value", now_has.round() as i64),
                );
            }
        }
        GeneOp::SaveUi(b) | GeneOp::SaveUiUe(b) | GeneOp::SaveSe(b) => {
            if let Some(person) = readable
                && let Some(g) = genome(world, person)
                && let Some(slot) = c.buffers.get_mut(usize::from(b))
            {
                let owner = world
                    .get::<DisplayName>(person)
                    .map_or_else(String::new, |n| n.0.clone());
                let se = matches!(op, GeneOp::SaveSe(_));
                *slot = Some(GeneBuffer {
                    data: if se { g.se } else { g.ui },
                    se,
                    with_ue: matches!(op, GeneOp::SaveUiUe(_)),
                    owner,
                });
            }
        }
        GeneOp::Clear(b) => {
            if let Some(slot) = c.buffers.get_mut(usize::from(b)) {
                *slot = None;
            }
        }
        GeneOp::Transfer(b) => {
            let (Some(person), Some(Some(buffer))) = (readable, c.buffers.get(usize::from(b)).cloned())
            else {
                return;
            };
            transfer(world, person, &buffer, None, true);
            let extra = world.resource_mut::<GameRng>().range(20, 50) as f32;
            irradiate(world, person, extra);
            c.page = GenePage::Note("genetics-transferred".into());
        }
        GeneOp::Injector(b) | GeneOp::IsoInjector(b, _) => {
            let Some(Some(buffer)) = c.buffers.get(usize::from(b)).cloned() else {
                return;
            };
            if now < c.injector_at {
                c.page = GenePage::Note("genetics-replicator-not-ready".into());
            } else {
                let block = match op {
                    GeneOp::IsoInjector(_, n) => {
                        let n = usize::from(n);
                        if n == 0 || n * 3 > buffer.data.len() {
                            c.page = GenePage::Note("genetics-injector-error".into());
                            world.entity_mut(console).insert(c);
                            return;
                        }
                        Some(n)
                    }
                    _ => None,
                };
                let data = match block {
                    Some(n) => dna::block_text(&buffer.data, n).to_owned(),
                    None => buffer.data.clone(),
                };
                if let Ok(item) =
                    crate::objects::spawn_object(world, &rs13_world::ObjectSpec::new(INJECTOR), place)
                {
                    world.entity_mut(item).insert(Injector {
                        se: buffer.se,
                        data,
                        block,
                        ue: buffer.with_ue.then(|| buffer.owner.clone()),
                        used: false,
                    });
                }
                c.injector_at = now + INJECTOR_REST_DS;
                c.page = GenePage::Note("genetics-injector-made".into());
            }
        }
    }
    world.entity_mut(console).insert(c);
}

fn page_or_none(readable: Option<Entity>, page: GenePage) -> GenePage {
    if readable.is_some() {
        page
    } else {
        GenePage::Note("genetics-no-occupant".into())
    }
}

fn wrap(value: u8, forward: bool, most: u8) -> u8 {
    if forward {
        if value >= most { 1 } else { value + 1 }
    } else if value <= 1 {
        most
    } else {
        value - 1
    }
}

/// The scanner holds its occupant while the beam is on (Luna locks it, then puts it back).
fn lock_for_beam(world: &mut World, scanner: Entity, on: bool) {
    if let Some(mut s) = world.get_mut::<DnaScanner>(scanner) {
        s.beam = on;
    }
}

/// A buffer or injector written into someone (Luna's `transfer` and `dnainjector/inject`):
/// a whole identifier, with its name if it carries one, or whole enzymes; or one block.
pub fn transfer(world: &mut World, person: Entity, buffer: &GeneBuffer, block: Option<usize>, scanner: bool) {
    let Some(mut g) = genome(world, person) else {
        return;
    };
    let target = if buffer.se { &mut g.se } else { &mut g.ui };
    match block {
        Some(n) => dna::set_block(target, n, &buffer.data[..3.min(buffer.data.len())]),
        None => {
            if buffer.data.len() == target.len() {
                *target = buffer.data.clone();
            }
        }
    }
    set_genome(world, person, g);
    if buffer.se {
        powers::check_in(world, person, false, scanner);
    } else {
        if buffer.with_ue && !buffer.owner.is_empty() {
            world.entity_mut(person).insert(DisplayName(buffer.owner.clone()));
        }
        powers::update_look(world, person);
    }
}

/// Beams that have run their time (Luna's `sleep(10 * radduration)` and what follows it).
pub fn run_consoles(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    let mut q = world.query::<(Entity, &GeneticsConsole)>();
    let done: Vec<(Entity, GeneOp)> = q
        .iter(world)
        .filter_map(|(e, c)| c.working.filter(|(t, _)| *t <= now).map(|(_, op)| (e, op)))
        .collect();
    for (console, op) in done {
        let Some(mut c) = world.get::<GeneticsConsole>(console).cloned() else {
            continue;
        };
        c.working = None;
        let scanner = scanner_of(world, console);
        if let Some(s) = scanner {
            lock_for_beam(world, s, false);
        }
        let occupant = occupant_of(world, scanner).filter(|o| genome(world, *o).is_some());
        c.page = match op {
            GeneOp::Identity => GenePage::Identity,
            GeneOp::Enzymes => GenePage::Enzymes,
            _ => GenePage::Main,
        };
        if let Some(person) = occupant {
            let (s, d) = (i32::from(c.strength), i32::from(c.duration));
            match op {
                GeneOp::Pulse => pulse(world, person, s, d),
                GeneOp::Identity => {
                    beam_identity(world, person, c.ui_block, c.sub, c.target_digit, s, d);
                }
                GeneOp::Enzymes => {
                    beam_enzymes(world, person, c.se_block, c.sub, s, d, scanner.is_some());
                }
                _ => {}
            }
        }
        world.entity_mut(console).insert(c);
    }
}

/// Luna `genpulse`: mostly a defect or a look changed, now and then a great power. Nothing
/// shows until the genes are next looked at.
fn pulse(world: &mut World, person: Entity, s: i32, d: i32) {
    let (a, b) = {
        let mut rng = world.resource_mut::<GameRng>();
        (rng.prob(95.0), rng.prob(75.0))
    };
    if a {
        if b {
            random_bad(world, person);
        } else {
            random_identity(world, person);
        }
    } else if world.resource_mut::<GameRng>().prob(95.0) {
        random_good(world, person);
    } else {
        random_identity(world, person);
    }
    irradiate(world, person, (s * 3 + d * 3) as f32);
}

/// Luna `unipulse`: the chosen digit pushed towards the one aimed at, most times; else a defect
/// or another look.
fn beam_identity(world: &mut World, person: Entity, block: u8, sub: u8, target: u8, s: i32, d: i32) {
    let hit = world.resource_mut::<GameRng>().prob(80.0 + d as f32 / 2.0);
    if hit {
        let Some(mut g) = genome(world, person) else {
            return;
        };
        let digit = dna::miniscramble_target(target, s, d, &mut world.resource_mut::<GameRng>());
        dna::set_digit(&mut g.ui, usize::from(block), usize::from(sub), digit);
        set_genome(world, person, g);
        powers::update_look(world, person);
        irradiate(world, person, (s + d) as f32);
    } else {
        if world.resource_mut::<GameRng>().prob(20.0 + s as f32) {
            random_bad(world, person);
            powers::check_in(world, person, false, true);
        } else {
            random_identity(world, person);
            powers::update_look(world, person);
        }
        irradiate(world, person, (s * 2 + d) as f32);
    }
}

/// Luna `strucpulse`: the chosen digit scrambled, now and then on the block beside it instead;
/// rarely, a defect or another look.
fn beam_enzymes(world: &mut World, person: Entity, block: u8, sub: u8, s: i32, d: i32, scanner: bool) {
    let hit = world.resource_mut::<GameRng>().prob(95.0 + d as f32 / 2.0);
    if hit {
        let Some(mut g) = genome(world, person) else {
            return;
        };
        let mut n = usize::from(block);
        if world.resource_mut::<GameRng>().prob(5.0) {
            let half = SE_BLOCKS as f32 / 2.0;
            let nf = n as f32;
            if n > 1 && nf < half {
                n += 1;
            } else if nf > half && n < SE_BLOCKS {
                n -= 1;
            }
        }
        let old = dna::digit(&g.se, n, usize::from(sub));
        let digit = dna::miniscramble(old, s, d, &mut world.resource_mut::<GameRng>());
        dna::set_digit(&mut g.se, n, usize::from(sub), digit);
        set_genome(world, person, g);
        powers::check_in(world, person, false, scanner);
        irradiate(world, person, (s + d) as f32);
    } else {
        if world.resource_mut::<GameRng>().prob(80.0 - d as f32) {
            random_bad(world, person);
            powers::check_in(world, person, false, scanner);
        } else {
            random_identity(world, person);
            powers::update_look(world, person);
        }
        irradiate(world, person, (s * 2 + d) as f32);
    }
}

/// Show each player the DNA console they have open.
pub fn update_views(world: &mut World) {
    let mut q = world.query::<(Entity, &GridPos, &PanelViewer)>();
    let viewers: Vec<(Entity, TilePos, Entity)> = q.iter(world).map(|(m, p, v)| (m, p.0, v.0)).collect();
    for (mob, at, console) in viewers {
        let Some(c) = world.get::<GeneticsConsole>(console).cloned() else {
            continue;
        };
        let (Some(place), Some(id)) = (
            world.get::<GridPos>(console).map(|p| p.0),
            world.get::<NetId>(console).copied(),
        ) else {
            continue;
        };
        let awake = world
            .get::<Vitals>(mob)
            .is_none_or(|v| v.stat == MobStat::Conscious);
        let powered = world.get::<PowerUser>(console).is_some_and(|p| p.powered);
        if !in_reach(at, place) || !awake || !powered {
            world.entity_mut(mob).remove::<PanelViewer>();
            if let Some(mut out) = world.get_mut::<GeneticsPanelOut>(mob) {
                out.0 = None;
            }
            continue;
        }
        let scanner = scanner_of(world, console);
        let locked = scanner
            .and_then(|s| world.get::<DnaScanner>(s))
            .is_some_and(|s| s.locked);
        let occupant = occupant_of(world, scanner).and_then(|person| {
            let ruined = world.get::<crate::changeling::Husked>(person).is_some();
            let g = genome(world, person)?;
            Some(GeneOccupant {
                stat: world.get::<Vitals>(person).map_or(MobStat::Conscious, |v| v.stat),
                health: world.get::<Body>(person).map_or(0, |b| b.health().round() as i16),
                radiation: world.get::<Radiation>(person).map_or(0, |r| r.0.round() as u16),
                ue: g.ue.to_uppercase(),
                ui: g.ui,
                se: g.se,
                ruined,
            })
        });
        let view = GeneticsPanelView {
            target: id,
            no_scanner: scanner.is_none(),
            occupant,
            page: c.page.clone(),
            locked,
            ui_block: c.ui_block,
            se_block: c.se_block,
            sub: c.sub,
            target_digit: c.target_digit,
            duration: c.duration,
            strength: c.strength,
            buffers: c.buffers.clone(),
        };
        match world.get_mut::<GeneticsPanelOut>(mob) {
            Some(mut out) => {
                if out.0.as_ref() != Some(&view) {
                    out.0 = Some(view);
                }
            }
            None => {
                world.entity_mut(mob).insert(GeneticsPanelOut(Some(view)));
            }
        }
    }
}

/// Close the window of anyone no longer looking at a DNA console.
pub fn clear_closed_views(
    mut out: Query<(&mut GeneticsPanelOut, Option<&PanelViewer>)>,
    consoles: Query<(), With<GeneticsConsole>>,
) {
    for (mut out, viewer) in &mut out {
        if out.0.is_some() && !viewer.is_some_and(|v| consoles.contains(v.0)) {
            out.0 = None;
        }
    }
}

/// How long injecting a person takes, ds (Luna's `spawn(40)` on `equip_e`).
pub const INJECT_DS: f32 = 40.0;

/// Luna's ready-made injectors: their prototype, the gene whose block they write (or the monkey
/// block), and the three digits. `FED` and `FA0` and `BD6` are on; `708` and `508` off.
pub const PREFABS: [(&str, Option<Gene>, &str); 20] = [
    ("dna-injector-hulk", Some(Gene::Hulk), "FED"),
    ("dna-injector-anti-hulk", Some(Gene::Hulk), "708"),
    ("dna-injector-xray", Some(Gene::Xray), "FED"),
    ("dna-injector-anti-xray", Some(Gene::Xray), "708"),
    ("dna-injector-cough", Some(Gene::Cough), "BD6"),
    ("dna-injector-anti-cough", Some(Gene::Cough), "708"),
    ("dna-injector-clumsy", Some(Gene::Clumsy), "FA0"),
    ("dna-injector-anti-clumsy", Some(Gene::Clumsy), "708"),
    ("dna-injector-stutter", Some(Gene::Nervous), "FA0"),
    ("dna-injector-anti-stutter", Some(Gene::Nervous), "708"),
    ("dna-injector-fire", Some(Gene::ColdResist), "FED"),
    ("dna-injector-anti-fire", Some(Gene::ColdResist), "708"),
    ("dna-injector-blind", Some(Gene::Blind), "FA0"),
    ("dna-injector-anti-blind", Some(Gene::Blind), "708"),
    ("dna-injector-tele", Some(Gene::Telekinesis), "FED"),
    ("dna-injector-anti-tele", Some(Gene::Telekinesis), "708"),
    ("dna-injector-deaf", Some(Gene::Deaf), "FA0"),
    ("dna-injector-anti-deaf", Some(Gene::Deaf), "708"),
    ("dna-injector-h2m", None, "FA0"),
    ("dna-injector-m2h", None, "508"),
];

pub fn is_injector(proto: &str) -> bool {
    proto == INJECTOR || PREFABS.iter().any(|(p, ..)| *p == proto)
}

/// Someone injecting a person: done at this time, if both stay put and it's still in hand.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct GeneInjecting {
    pub target: Entity,
    pub injector: Entity,
    pub done_at: f32,
    pub from: TilePos,
    pub target_from: TilePos,
}

/// What an injector carries, a ready-made one's worked out for this round's blocks.
fn contents(world: &mut World, injector: Entity) -> Option<Injector> {
    if let Some(i) = world.get::<Injector>(injector) {
        return Some(i.clone());
    }
    let proto = world.get::<Appearance>(injector)?.sprite.clone();
    let (_, gene, digits) = PREFABS.iter().find(|(p, ..)| *p == proto)?;
    let block = match gene {
        Some(gene) => layout(world).block(*gene),
        None => dna::MONKEY_BLOCK,
    };
    let made = Injector {
        se: true,
        data: (*digits).to_owned(),
        block: Some(block),
        ue: None,
        used: false,
    };
    world.entity_mut(injector).insert(made.clone());
    Some(made)
}

fn name_of(world: &World, who: Entity) -> String {
    world
        .get::<DisplayName>(who)
        .map_or_else(String::new, |n| n.0.clone())
}

/// An injector used on someone (Luna `dnainjector/attack`): a person, themself included, after
/// four seconds of struggle; a monkey at once.
pub fn use_injector(world: &mut World, mob: Entity, injector: Entity, target: Entity) {
    if contents(world, injector).is_none_or(|i| i.used) {
        return;
    }
    let (Some(from), Some(target_from)) = (
        world.get::<GridPos>(mob).map(|p| p.0),
        world.get::<GridPos>(target).map(|p| p.0),
    ) else {
        return;
    };
    if is_person(world, target) {
        let now = world.resource::<WorldTime>().deciseconds();
        world.entity_mut(mob).insert(GeneInjecting {
            target,
            injector,
            done_at: now + INJECT_DS,
            from,
            target_from,
        });
        let msg = LocMsg::new("injector-trying")
            .arg("name", name_of(world, mob))
            .arg("victim", name_of(world, target));
        world
            .resource_mut::<crate::combat::VisibleEvents>()
            .0
            .push((from, msg, None));
    } else if world
        .get::<Appearance>(target)
        .is_some_and(crate::monkey_round::is_monkey)
    {
        let msg = LocMsg::new("injector-injected")
            .arg("name", name_of(world, mob))
            .arg("victim", name_of(world, target));
        world
            .resource_mut::<crate::combat::VisibleEvents>()
            .0
            .push((from, msg, None));
        tell(
            world,
            mob,
            LocMsg::new("injector-you-inject").arg("victim", name_of(world, target)),
        );
        inject(world, injector, target);
    }
}

/// Luna `dnainjector/inject`: written in, a dose of radiation, and the injector is used up.
pub fn inject(world: &mut World, injector: Entity, target: Entity) {
    let Some(i) = contents(world, injector) else {
        return;
    };
    let rads = world.resource_mut::<GameRng>().range(20, 40) as f32;
    irradiate(world, target, rads);
    let buffer = GeneBuffer {
        data: i.data.clone(),
        se: i.se,
        with_ue: i.ue.is_some(),
        owner: i.ue.clone().unwrap_or_default(),
    };
    transfer(world, target, &buffer, i.block, false);
    world.entity_mut(injector).despawn();
}

/// Injections that have taken their time.
pub fn run_injections(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    let mut q = world.query::<(Entity, &GeneInjecting, &GridPos, &crate::hands::Hands)>();
    let jobs: Vec<(Entity, GeneInjecting, TilePos, Option<Entity>)> = q
        .iter(world)
        .map(|(e, j, p, h)| (e, *j, p.0, h.active_item()))
        .collect();
    for (mob, job, at, held) in jobs {
        let target_at = world.get::<GridPos>(job.target).map(|p| p.0);
        let cuffed = world
            .get::<crate::equipment::Equipment>(mob)
            .is_some_and(|e| e.get(rs13_world::Slot::Cuffs).is_some());
        let awake = world
            .get::<Vitals>(mob)
            .is_none_or(|v| v.stat == MobStat::Conscious);
        if at != job.from
            || target_at != Some(job.target_from)
            || held != Some(job.injector)
            || cuffed
            || !awake
        {
            world.entity_mut(mob).remove::<GeneInjecting>();
            continue;
        }
        if now < job.done_at {
            continue;
        }
        world.entity_mut(mob).remove::<GeneInjecting>();
        if let Some(mut hands) = world.get_mut::<crate::hands::Hands>(mob) {
            let active = hands.active;
            hands.slot_mut(active).take();
        }
        inject(world, job.injector, job.target);
    }
}

/// Buttons pressed on DNA consoles (sent as picks; the wizard's things take theirs first).
pub fn run_picks(world: &mut World) {
    let mut q = world.query::<(Entity, &mut crate::filing::PendingPicks)>();
    let picks: Vec<(Entity, Vec<(Entity, usize)>)> = q
        .iter_mut(world)
        .filter(|(_, p)| !p.0.is_empty())
        .map(|(mob, mut p)| (mob, std::mem::take(&mut p.0)))
        .collect();
    for (mob, list) in picks {
        let awake = world
            .get::<Vitals>(mob)
            .is_none_or(|v| v.stat == MobStat::Conscious);
        for (target, line) in list {
            if awake && world.get::<GeneticsConsole>(target).is_some() {
                console_pick(world, mob, target, line);
            }
        }
    }
}
