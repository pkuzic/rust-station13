//! Anomalies: the alien artifact, the anomaly analyser and exotic particle harvester, anomaly
//! batteries and the power utilizer (Luna `modules/research/anomaly`). Spec:
//! `docs/reference/anomalies.md`.
//!
//! - **The artifact** has a primary effect and, three times in four, a hidden secondary one. Each
//!   effect has a kind (heal, hurt, stun, teleport, …), a way of working (on touch, as an aura, or
//!   in pulses), a range, a charge time and a trigger that turns it on and off: a bare hand,
//!   reagents, force, energy, heat, or the air around it.
//! - **The analyser** scans what stands east of it and prints a report on what it emits and what
//!   sets it off.
//! - **The harvester** charges an anomaly battery with a copy of the artifact east of it, and can
//!   dump the charge, setting the effect off as it does.
//! - **The utilizer** runs a charged battery's effect where it is carried, for as long as the
//!   charge or its timer lasts.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_simulation::atmos::Gas;
use rs13_world::{
    Appearance, Dense, Ghost, GridPos, Hidden, MapLayer, MobStat, NetId, ObjectSpec, PickListView, Slot,
    TilePos,
};

use crate::actions::{Outbox, in_reach};
use crate::atmos::StationAtmos;
use crate::cells::JOULES_PER_UNIT;
use crate::combat::VisibleEvents;
use crate::equipment::Equipment;
use crate::filing::{PendingPicks, PickListOut};
use crate::hands::{Hands, HeldBy, Item};
use crate::health::{Body, Vitals, Zone};
use crate::machines::MACHINE_TICKS;
use crate::movement::{Bumps, Mob};
use crate::objects::NetIds;
use crate::power::{Apc, Channel, PowerUser, SMES_UNIT, Smes};
use crate::rng::GameRng;
use crate::sparks::SparkShowers;
use crate::time::WorldTime;
use crate::wires::PanelViewer;

pub const ARTIFACT: &str = "alien-artifact";
pub const ANALYSER: &str = "anomaly-analyser";
pub const HARVESTER: &str = "anomaly-harvester";
pub const BATTERY: &str = "anomaly-battery";
pub const UTILIZER: &str = "anomaly-utilizer";
/// A tile of the force field an artifact can throw up around itself.
pub const FIELD: &str = "artifact-field";
const PAPER: &str = "paper";

/// How long a scan takes with the analyser's stock parts, ds (Luna 160, less 20 for each of two
/// scanning modules, 10 for the manipulator and 10 for the laser).
pub const SCAN_DS: f32 = 100.0;
/// An anomaly battery's capacity (Luna `capacity`).
pub const BATTERY_CAPACITY: f32 = 200.0;
/// Charge harvested a machine cycle with stock parts (Luna `rating`: 0.2, plus 0.2 a capacitor
/// and 0.1 each for the manipulator and the laser).
pub const HARVEST_RATE: f32 = 1.0;
/// A dump goes four times as fast.
pub const DUMP_RATE: f32 = 4.0 * HARVEST_RATE;
/// The harvester's draw idle and working, W (Luna `idle_power_usage`, `active_power_usage`).
pub const HARVESTER_IDLE_W: f64 = 50.0;
pub const HARVESTER_ACTIVE_W: f64 = 750.0;
/// The utilizer's timer to start with, in machine cycles (Luna `time = 50`).
pub const UTILIZER_TIME: u32 = 50;
/// Air colder or hotter than this sets off cold and heat triggers, K.
pub const COLD_K: f64 = 225.0;
pub const HOT_K: f64 = 375.0;
/// Moles of a gas on the tile that set off a gas trigger.
pub const GAS_MOLES: f64 = 10.0;
/// What an item needs to hit for to set off a force trigger.
pub const FORCE: f32 = 10.0;
/// Luna's cell drains reach every APC this far away, tiles (`range(200, holder)`).
const APC_RANGE: i32 = 200;

/// What an effect does (Luna's compiled `artifact_effect` types).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    BadFeeling,
    CellCharge,
    CellDrain,
    Cold,
    DnaSwitch,
    Emp,
    ForceField,
    GoodFeeling,
    Heal,
    Heat,
    Hurt,
    Radiate,
    RoboHeal,
    RoboHurt,
    Sleepy,
    Stun,
    Teleport,
}

impl Kind {
    pub const ALL: [Kind; 17] = [
        Kind::BadFeeling,
        Kind::CellCharge,
        Kind::CellDrain,
        Kind::Cold,
        Kind::DnaSwitch,
        Kind::Emp,
        Kind::ForceField,
        Kind::GoodFeeling,
        Kind::Heal,
        Kind::Heat,
        Kind::Hurt,
        Kind::Radiate,
        Kind::RoboHeal,
        Kind::RoboHurt,
        Kind::Sleepy,
        Kind::Stun,
        Kind::Teleport,
    ];
}

/// How an effect works (Luna `EFFECT_TOUCH`, `EFFECT_AURA`, `EFFECT_PULSE`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// On whoever touches it.
    Touch,
    /// On everyone in range, every cycle while on.
    Aura,
    /// On everyone in range, each time it has charged while on.
    Pulse,
}

/// What turns an effect on and off (Luna `TRIGGER_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    Touch,
    Water,
    Acid,
    Volatile,
    Toxin,
    Force,
    Energy,
    Heat,
    Cold,
    Plasma,
    Oxygen,
    CarbonDioxide,
    Nitrogen,
}

impl Trigger {
    pub const ALL: [Trigger; 13] = [
        Trigger::Touch,
        Trigger::Water,
        Trigger::Acid,
        Trigger::Volatile,
        Trigger::Toxin,
        Trigger::Force,
        Trigger::Energy,
        Trigger::Heat,
        Trigger::Cold,
        Trigger::Plasma,
        Trigger::Oxygen,
        Trigger::CarbonDioxide,
        Trigger::Nitrogen,
    ];

    /// What the analyser says about it: something physical, something energetic, or the air.
    fn class(self) -> &'static str {
        match self {
            Self::Touch | Self::Water | Self::Acid | Self::Volatile | Self::Toxin => "physical",
            Self::Force | Self::Energy | Self::Heat | Self::Cold => "energetic",
            _ => "atmospheric",
        }
    }
}

/// One effect of an artifact, or its copy in a battery.
#[derive(Debug, Clone, PartialEq)]
pub struct Effect {
    pub kind: Kind,
    pub mode: Mode,
    pub trigger: Trigger,
    pub range: i32,
    /// Cycles charged towards a pulse, and how many a pulse takes.
    pub charge: u32,
    pub charge_max: u32,
    /// Its energy signature, such as "sigma-417".
    pub id: String,
    /// What the analyser reads off it, 0 to 7 (Luna `effect_type`).
    pub emission: u8,
    pub active: bool,
    /// The kind's own number: the temperature heat and cold work towards (K), how hard a DNA
    /// switch scrambles, or how much an artifact radiates.
    pub strength: f32,
    /// The force field it holds up, and where it was put.
    pub field: Vec<Entity>,
    pub field_at: Option<TilePos>,
}

const SIGNATURES: [&str; 12] = [
    "kappa", "sigma", "antaeres", "beta", "omicron", "iota", "epsilon", "omega", "gamma", "delta", "tau",
    "alpha",
];

fn pick<T: Copy>(rng: &mut GameRng, from: &[T]) -> T {
    from[rng.range(0, from.len() as i32 - 1) as usize]
}

impl Effect {
    /// A new random effect (Luna `artifact_effect/New` and each kind's own `New`).
    pub fn roll(rng: &mut GameRng) -> Self {
        let kind = pick(rng, &Kind::ALL);
        let mut mode = pick(rng, &[Mode::Touch, Mode::Aura, Mode::Pulse]);
        let mut trigger = pick(rng, &Trigger::ALL);
        let id = format!("{}-{}", pick(rng, &SIGNATURES), rng.range(100, 999));
        // Short range and charge twice as often as medium, four times as often as long.
        let tier = rng.range(1, 175);
        let (charge_max, range) = if tier <= 100 {
            (rng.range(3, 20), rng.range(1, 3))
        } else if tier <= 150 {
            (rng.range(15, 40), rng.range(5, 10))
        } else {
            (rng.range(20, 120), rng.range(10, 20))
        };
        let mut strength = 0.0;
        let emission = match kind {
            Kind::BadFeeling | Kind::GoodFeeling => 2,
            Kind::CellCharge | Kind::CellDrain => 3,
            Kind::Emp => {
                mode = Mode::Pulse;
                3
            }
            Kind::Cold | Kind::Heat => {
                strength = if kind == Kind::Cold {
                    rng.range(0, 250)
                } else {
                    rng.range(300, 600)
                } as f32;
                mode = pick(rng, &[Mode::Touch, Mode::Aura]);
                pick(rng, &[5, 6, 7])
            }
            Kind::DnaSwitch => {
                strength = if mode == Mode::Aura {
                    rng.range(5, 30)
                } else {
                    rng.range(25, 95)
                } as f32;
                5
            }
            Kind::ForceField => {
                trigger = Trigger::Touch;
                4
            }
            Kind::Heal | Kind::Hurt => 5,
            Kind::Radiate => {
                strength = rng.range(1, 10) as f32;
                pick(rng, &[4, 5])
            }
            Kind::RoboHeal | Kind::RoboHurt => pick(rng, &[3, 4]),
            Kind::Sleepy | Kind::Stun => pick(rng, &[2, 5]),
            Kind::Teleport => 6,
        };
        Self {
            kind,
            mode,
            trigger,
            range,
            charge: 0,
            charge_max: charge_max as u32,
            id,
            emission,
            active: false,
            strength,
            field: Vec::new(),
            field_at: None,
        }
    }

    /// A fresh copy, as the harvester makes for a battery.
    fn copy(&self) -> Self {
        Self {
            charge: 0,
            active: false,
            field: Vec::new(),
            field_at: None,
            ..self.clone()
        }
    }
}

/// The alien artifact.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Artifact {
    /// Which of the six looks it has.
    pub look: u8,
    pub primary: Effect,
    pub secondary: Option<Effect>,
    /// Held still by the analyser or the harvester (Luna `being_used`).
    pub busy: bool,
}

/// An anomaly battery: the effect it carries and its charge.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct AnomalyBattery {
    pub effect: Option<Effect>,
    pub charge: f32,
    pub capacity: f32,
}

/// On a battery: the harvester or utilizer it is in.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InDevice(pub Entity);

#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct Analyser {
    /// What it is scanning, and when it finishes, ds.
    pub scanning: Option<(Entity, f32)>,
    pub reports: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HarvestMode {
    #[default]
    Idle,
    Harvesting,
    Dumping,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct Harvester {
    pub battery: Option<Entity>,
    pub mode: HarvestMode,
    pub artifact: Option<Entity>,
}

/// The anomaly power utilizer (Luna `anodevice`). Times are in machine cycles.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Utilizer {
    pub battery: Option<Entity>,
    pub cooldown: u32,
    pub active: bool,
    pub timing: bool,
    pub time: u32,
    /// The timer it was started with: half of it is the cooldown after.
    pub archived: u32,
}

impl Default for Utilizer {
    fn default() -> Self {
        Self {
            battery: None,
            cooldown: 0,
            active: false,
            timing: false,
            time: UTILIZER_TIME,
            archived: UTILIZER_TIME,
        }
    }
}

/// A tile of an artifact's force field.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArtifactField;

pub fn is_machine(proto: &str) -> bool {
    matches!(proto, ARTIFACT | ANALYSER | HARVESTER)
}

/// Whether a click on a thing with this sprite is ours.
pub fn is_clicked(sprite: &str) -> bool {
    is_machine(sprite)
}

pub fn spawn_machine(world: &mut World, proto: &str, pos: TilePos, id: NetId) -> Entity {
    match proto {
        ARTIFACT => {
            let artifact = {
                let mut rng = world.resource_mut::<GameRng>();
                let primary = Effect::roll(&mut rng);
                let secondary = rng.prob(75.0).then(|| Effect::roll(&mut rng));
                Artifact {
                    look: rng.range(0, 5) as u8,
                    primary,
                    secondary,
                    busy: false,
                }
            };
            let state = format!("ano{}0", artifact.look);
            world
                .spawn((
                    GridPos(pos),
                    id,
                    Dense,
                    Appearance::new(ARTIFACT, &state),
                    artifact,
                ))
                .id()
        }
        ANALYSER => world
            .spawn((
                GridPos(pos),
                id,
                Dense,
                Appearance::new(ANALYSER, ""),
                Analyser::default(),
                PowerUser::new(Channel::Equipment, 0.0),
            ))
            .id(),
        _ => world
            .spawn((
                GridPos(pos),
                id,
                Dense,
                Appearance::new(HARVESTER, ""),
                Harvester::default(),
                PowerUser::new(Channel::Equipment, HARVESTER_IDLE_W),
            ))
            .id(),
    }
}

/// The parts a newly spawned battery or utilizer needs.
pub fn equip_item(world: &mut World, item: Entity, proto: &str) {
    match proto {
        BATTERY => {
            world.entity_mut(item).insert(AnomalyBattery {
                effect: None,
                charge: 0.0,
                capacity: BATTERY_CAPACITY,
            });
        }
        UTILIZER => {
            world.entity_mut(item).insert(Utilizer::default());
        }
        _ => {}
    }
}

// --- Small helpers ---

fn at(world: &World, e: Entity) -> Option<TilePos> {
    world.get::<GridPos>(e).map(|p| p.0)
}

/// Where a thing is: its tile, or that of whoever or whatever holds it.
fn where_is(world: &World, thing: Entity) -> Option<TilePos> {
    let mut e = thing;
    for _ in 0..4 {
        if let Some(p) = at(world, e) {
            return Some(p);
        }
        e = world
            .get::<HeldBy>(e)
            .map(|h| h.0)
            .or_else(|| world.get::<InDevice>(e).map(|d| d.0))?;
    }
    None
}

fn tell(world: &mut World, mob: Entity, msg: LocMsg) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(msg);
    }
}

fn around(world: &mut World, tile: TilePos, msg: LocMsg) {
    world.resource_mut::<VisibleEvents>().0.push((tile, msg, None));
}

fn states(world: &mut World, machine: Entity, id: &str) {
    if let Some(tile) = at(world, machine) {
        let name = sprite(world, machine).unwrap_or_default();
        around(world, tile, LocMsg::new(id).arg("item", name));
    }
}

fn sprite(world: &World, e: Entity) -> Option<String> {
    world.get::<Appearance>(e).map(|a| a.sprite.clone())
}

fn powered(world: &World, e: Entity) -> bool {
    world.get::<PowerUser>(e).is_none_or(|p| p.powered)
}

fn awake(world: &World, mob: Entity) -> bool {
    world
        .get::<Vitals>(mob)
        .is_none_or(|v| v.stat == MobStat::Conscious)
}

fn roll(world: &mut World, percent: f32) -> bool {
    world.resource_mut::<GameRng>().prob(percent)
}

fn rand(world: &mut World, low: i32, high: i32) -> f32 {
    world.resource_mut::<GameRng>().range(low, high) as f32
}

/// The tile east of a machine, where it looks for what to work on.
fn pad(tile: TilePos) -> TilePos {
    TilePos::new(tile.x + 1, tile.y, tile.z)
}

fn worn(world: &World, mob: Entity, slot: Slot) -> Option<String> {
    let item = world.get::<Equipment>(mob)?.get(slot)?;
    sprite(world, item)
}

/// From 1 for no protection to 0 for full (Luna `GetAnomalySusceptibility`): gloves and science
/// goggles help a little, and energy armour averaged over the body a lot.
pub fn susceptibility(world: &World, mob: Entity) -> f32 {
    if !crate::genetics::is_person(world, mob) {
        return 1.0;
    }
    let mut protected = 0.0;
    if worn(world, mob, Slot::Gloves).is_some() {
        protected += 0.1;
    }
    if worn(world, mob, Slot::Glasses).as_deref() == Some("science-goggles") {
        protected += 0.1;
    }
    let armour: f32 = Zone::ALL
        .iter()
        .map(|z| crate::combat::armour_on(world, mob, *z, crate::combat::ArmorKind::Energy))
        .sum::<f32>()
        / Zone::ALL.len() as f32;
    protected += armour / 50.0;
    (1.0 - protected).max(0.0)
}

/// Living bodies within `range` tiles of a place (people only if `people`).
fn bodies_near(world: &mut World, here: TilePos, range: i32, people: bool) -> Vec<Entity> {
    let mut q = world.query_filtered::<(Entity, &GridPos, Option<&Vitals>), With<Body>>();
    let found: Vec<Entity> = q
        .iter(world)
        .filter(|(_, p, v)| {
            p.0.z == here.z
                && (p.0.x - here.x).abs() <= range
                && (p.0.y - here.y).abs() <= range
                && v.is_none_or(|v| v.stat != MobStat::Dead)
        })
        .map(|(e, ..)| e)
        .collect();
    if people {
        found
            .into_iter()
            .filter(|m| crate::genetics::is_person(world, *m))
            .collect()
    } else {
        found
    }
}

/// Damage across the whole body; negative heals (Luna's `adjust*Loss`).
fn harm(world: &mut World, mob: Entity, brute: f32, burn: f32, tox: f32, oxy: f32) {
    let Some(mut body) = world.get_mut::<Body>(mob) else {
        return;
    };
    if brute > 0.0 || burn > 0.0 {
        body.take_damage_spread(brute.max(0.0), burn.max(0.0));
    }
    if brute < 0.0 || burn < 0.0 {
        let n = Zone::ALL.len() as f32;
        for zone in Zone::ALL {
            body.heal(zone, (-brute).max(0.0) / n, (-burn).max(0.0) / n);
        }
    }
    body.tox = (body.tox + tox).max(0.0);
    body.oxy = (body.oxy + oxy).max(0.0);
}

fn feed(world: &mut World, mob: Entity, amount: f32) {
    if let Some(mut n) = world.get_mut::<crate::food::Nutrition>(mob) {
        n.level = (n.level + amount).max(0.0);
    }
}

fn unirradiate(world: &mut World, mob: Entity, amount: f32) {
    if let Some(mut r) = world.get_mut::<crate::genetics::Radiation>(mob) {
        r.0 = (r.0 - amount).max(0.0);
    }
}

fn knock_down(world: &mut World, mob: Entity, counts: f32) {
    if counts > 0.0 {
        crate::combat::weaken(world, mob, counts * crate::combat::COUNT_DS);
    }
}

/// One of a numbered set of messages, at random.
fn one_of(world: &mut World, prefix: &str, count: i32) -> LocMsg {
    let n = world.resource_mut::<GameRng>().range(1, count);
    LocMsg::new(format!("{prefix}-{n}"))
}

// --- Effects ---

/// Luna `DoEffectTouch`: on whoever touched it, or for heat and cold, on the air where it is.
fn touch_effect(world: &mut World, e: &mut Effect, user: Entity, here: TilePos) {
    let weak = susceptibility(world, user);
    let person = crate::genetics::is_person(world, user);
    let body = world.get::<Body>(user).is_some();
    match e.kind {
        Kind::Heal if body && roll(world, weak * 100.0) => {
            tell(world, user, LocMsg::new("anomaly-heal-touch"));
            if person {
                let n = Zone::ALL.len() as f32;
                harm(world, user, -25.0 * weak * n, -25.0 * weak * n, 0.0, 0.0);
                feed(world, user, 50.0 * weak);
                unirradiate(world, user, 25.0 * weak);
            }
            harm(
                world,
                user,
                -10.0 * weak,
                -10.0 * weak,
                -25.0 * weak,
                -25.0 * weak,
            );
        }
        Kind::Hurt if body && roll(world, weak * 100.0) => {
            tell(world, user, LocMsg::new("anomaly-hurt-touch"));
            let (brute, burn, tox, oxy) = (
                rand(world, 5, 25),
                rand(world, 5, 25),
                rand(world, 5, 25),
                rand(world, 5, 25),
            );
            harm(world, user, brute * weak, burn * weak, tox * weak, oxy * weak);
            crate::genetics::irradiate(world, user, 25.0 * weak);
            feed(world, user, -50.0 * weak);
            knock_down(world, user, 6.0 * weak);
        }
        Kind::Stun if body && roll(world, weak * 100.0) => {
            tell(world, user, LocMsg::new("anomaly-stun-touch"));
            knock_down(world, user, 45.0 * weak);
        }
        Kind::Teleport if roll(world, weak * 100.0) => {
            if teleport(world, user, 50) {
                tell(world, user, LocMsg::new("anomaly-teleport-touch"));
            }
        }
        Kind::BadFeeling | Kind::GoodFeeling if person => {
            let bad = e.kind == Kind::BadFeeling;
            if roll(world, 50.0) {
                let drastic = roll(world, 75.0);
                feeling(world, user, bad, drastic);
            }
        }
        Kind::Cold | Kind::Heat => {
            let k = rand(world, 5, 50);
            warm_by(world, here, if e.kind == Kind::Cold { -k } else { k });
        }
        Kind::DnaSwitch if person && roll(world, weak * 100.0) => {
            let msg = one_of(world, "anomaly-dna", 7);
            tell(world, user, msg);
            let identity = roll(world, 75.0);
            crate::genetics::scramble(world, user, identity, weak * e.strength);
        }
        Kind::Radiate => crate::genetics::irradiate(world, user, e.strength * 5.0),
        Kind::Sleepy if person && roll(world, weak * 100.0) => {
            let msg = one_of(world, "anomaly-sleepy", 3);
            tell(world, user, msg);
            let blur = rand(world, 1, 3) * weak;
            crate::eyes::blur(world, user, blur);
        }
        _ => {}
    }
}

/// Luna `DoEffectAura`: every cycle while on.
fn aura(world: &mut World, e: &mut Effect, here: TilePos) {
    match e.kind {
        Kind::Heal | Kind::Hurt => {
            let sign = if e.kind == Kind::Heal { -1.0 } else { 1.0 };
            for mob in bodies_near(world, here, e.range, false) {
                let weak = susceptibility(world, mob);
                if roll(world, weak * 100.0) {
                    if roll(world, 10.0) {
                        let id = if sign < 0.0 {
                            "anomaly-heal-aura"
                        } else {
                            "anomaly-hurt-aura"
                        };
                        tell(world, mob, LocMsg::new(id));
                    }
                    let d = sign * weak;
                    harm(world, mob, d, d, d, d);
                }
            }
        }
        Kind::Stun => {
            for mob in bodies_near(world, here, e.range, false) {
                let weak = susceptibility(world, mob);
                if roll(world, 10.0 * weak) {
                    tell(world, mob, LocMsg::new("anomaly-stun-aura"));
                    knock_down(world, mob, 2.0);
                } else if roll(world, 10.0) {
                    tell(world, mob, LocMsg::new("anomaly-numb"));
                }
            }
        }
        Kind::Teleport => {
            for mob in bodies_near(world, here, e.range, false) {
                let weak = susceptibility(world, mob);
                if roll(world, weak * 100.0) && teleport(world, mob, 30) {
                    tell(world, mob, LocMsg::new("anomaly-teleport-wave"));
                }
            }
        }
        Kind::BadFeeling | Kind::GoodFeeling => {
            let bad = e.kind == Kind::BadFeeling;
            for mob in bodies_near(world, here, e.range, true) {
                if roll(world, 5.0) {
                    let drastic = !roll(world, 75.0);
                    feeling(world, mob, bad, drastic);
                }
            }
        }
        Kind::CellCharge => charge_cells(world, here, e.range, 25.0, 25.0),
        Kind::CellDrain => charge_cells(world, here, e.range, -50.0, -100.0),
        Kind::Cold => {
            if temperature(world, here).is_some_and(|t| t > f64::from(e.strength)) {
                let d = -f32::from(u8::from(roll(world, 100.0 / 3.0)));
                warm_by(world, here, d);
            }
        }
        Kind::Heat => {
            if temperature(world, here).is_some_and(|t| t < f64::from(e.strength)) {
                let d = f32::from(u8::from(roll(world, 100.0 / 3.0)));
                warm_by(world, here, d);
            }
        }
        Kind::DnaSwitch => {
            for mob in bodies_near(world, here, e.range, true) {
                let weak = susceptibility(world, mob);
                if roll(world, weak * 100.0) {
                    if roll(world, 30.0) {
                        let msg = one_of(world, "anomaly-dna", 7);
                        tell(world, mob, msg);
                    }
                    let identity = roll(world, 50.0);
                    crate::genetics::scramble(world, mob, identity, weak * e.strength);
                }
            }
        }
        Kind::Radiate => {
            for mob in bodies_near(world, here, e.range, false) {
                crate::genetics::irradiate(world, mob, e.strength);
            }
        }
        Kind::Sleepy => {
            for mob in bodies_near(world, here, e.range, true) {
                let weak = susceptibility(world, mob);
                if roll(world, weak * 100.0) {
                    if roll(world, 10.0) {
                        let msg = one_of(world, "anomaly-sleepy", 3);
                        tell(world, mob, msg);
                    }
                    crate::eyes::blur(world, mob, weak);
                }
            }
        }
        _ => {}
    }
}

/// Luna `DoEffectPulse`: each time it has charged, while on.
fn pulse(world: &mut World, e: &mut Effect, here: TilePos) {
    match e.kind {
        Kind::Heal | Kind::Hurt => {
            let heal = e.kind == Kind::Heal;
            for mob in bodies_near(world, here, e.range, false) {
                let weak = susceptibility(world, mob);
                if roll(world, weak * 100.0) {
                    let id = if heal {
                        "anomaly-heal-pulse"
                    } else {
                        "anomaly-hurt-pulse"
                    };
                    tell(world, mob, LocMsg::new(id));
                    let d = if heal { -5.0 * weak } else { 3.0 * weak };
                    harm(world, mob, d, d, d, d);
                }
            }
        }
        Kind::Stun => {
            for mob in bodies_near(world, here, e.range, false) {
                let weak = susceptibility(world, mob);
                if roll(world, 100.0 * weak) {
                    tell(world, mob, LocMsg::new("anomaly-stun-pulse"));
                    knock_down(world, mob, 4.0 * weak);
                }
            }
        }
        Kind::Teleport => {
            for mob in bodies_near(world, here, e.range, false) {
                let weak = susceptibility(world, mob);
                if roll(world, weak * 100.0) && teleport(world, mob, 15) {
                    tell(world, mob, LocMsg::new("anomaly-teleport-wave"));
                }
            }
        }
        Kind::BadFeeling | Kind::GoodFeeling => {
            let bad = e.kind == Kind::BadFeeling;
            for mob in bodies_near(world, here, e.range, true) {
                if roll(world, 50.0) {
                    let drastic = roll(world, 95.0);
                    feeling(world, mob, bad, drastic);
                }
            }
        }
        Kind::CellCharge => {
            let apc = rand(world, 0, 100);
            charge_cells(world, here, e.range, apc, 250.0);
        }
        Kind::CellDrain => {
            let apc = rand(world, 0, 150);
            charge_cells(world, here, e.range, -apc, -250.0);
        }
        Kind::Emp => {
            let r = e.range;
            crate::explosions::empulse(world, here, r / 2, r);
        }
        Kind::DnaSwitch => {
            for mob in bodies_near(world, here, APC_RANGE, true) {
                let weak = susceptibility(world, mob);
                if roll(world, weak * 100.0) && roll(world, 75.0) {
                    let msg = one_of(world, "anomaly-dna", 7);
                    tell(world, mob, msg);
                }
                if roll(world, 25.0) {
                    let identity = roll(world, 75.0);
                    crate::genetics::scramble(world, mob, identity, weak * e.strength);
                }
            }
        }
        Kind::Radiate => {
            for mob in bodies_near(world, here, e.range, false) {
                crate::genetics::irradiate(world, mob, e.strength * 25.0);
            }
        }
        Kind::Sleepy => {
            for mob in bodies_near(world, here, e.range, true) {
                let weak = susceptibility(world, mob);
                if roll(world, weak * 100.0) {
                    let msg = one_of(world, "anomaly-sleepy", 3);
                    tell(world, mob, msg);
                    let blur = rand(world, 5, 15) * weak;
                    crate::eyes::blur(world, mob, blur);
                }
            }
        }
        _ => {}
    }
}

/// One of the bad or good feelings: a strong one or a mild one.
fn feeling(world: &mut World, mob: Entity, bad: bool, drastic: bool) {
    let (prefix, count) = match (bad, drastic) {
        (true, true) => ("anomaly-dread", 6),
        (true, false) => ("anomaly-unease", 16),
        (false, true) => ("anomaly-euphoria", 6),
        (false, false) => ("anomaly-ease", 14),
    };
    let msg = one_of(world, prefix, count);
    tell(world, mob, msg);
}

/// Luna's cell charge and drain: every APC's cell within reach, and SMES units in range.
/// Amounts are in Luna's charge units.
fn charge_cells(world: &mut World, here: TilePos, range: i32, apc: f32, smes: f32) {
    let near = |p: TilePos, r: i32| p.z == here.z && (p.x - here.x).abs() <= r && (p.y - here.y).abs() <= r;
    let mut apcs = world.query::<(&GridPos, &mut Apc)>();
    for (p, mut a) in apcs.iter_mut(world) {
        if !near(p.0, APC_RANGE) {
            continue;
        }
        if let Some((charge, capacity)) = a.cell {
            let to = (charge + f64::from(apc) * JOULES_PER_UNIT).clamp(0.0, capacity);
            a.cell = Some((to, capacity));
        }
    }
    let mut units = world.query::<(&GridPos, &mut Smes)>();
    for (p, mut s) in units.iter_mut(world) {
        if near(p.0, range) {
            s.charge = (s.charge + f64::from(smes) * SMES_UNIT).clamp(0.0, s.capacity);
        }
    }
}

fn temperature(world: &World, here: TilePos) -> Option<f64> {
    let atmos = world.get_resource::<StationAtmos>()?;
    let air = atmos.0.air_at(here);
    (!air.is_vacuum()).then_some(air.temperature)
}

/// Warm (or cool) the air where it is by so many kelvin.
fn warm_by(world: &mut World, here: TilePos, kelvin: f32) {
    let Some(mut atmos) = world.get_resource_mut::<StationAtmos>() else {
        return;
    };
    let moles = atmos.0.air_at(here).total_moles();
    if moles <= 0.0 {
        return;
    }
    let mut air = atmos.0.remove_at(here, moles);
    let energy = air.heat_capacity() * f64::from(kelvin);
    air.add_energy(energy);
    atmos.0.add_at(here, &air);
}

/// Off to a random floor within so many tiles, with sparks at both ends. Whether it went.
fn teleport(world: &mut World, mob: Entity, range: i32) -> bool {
    let Some(here) = at(world, mob) else {
        return false;
    };
    let mut layers = world.query::<&MapLayer>();
    let Some(map) = layers.iter(world).find(|l| l.0.z == here.z).map(|l| l.0.clone()) else {
        return false;
    };
    let mut floors = Vec::new();
    for y in here.y - range..=here.y + range {
        for x in here.x - range..=here.x + range {
            let t = TilePos::new(x, y, here.z);
            if t != here
                && map
                    .tile(t)
                    .is_some_and(|d| !d.solid && d.id != "space" && d.id != "open-space")
            {
                floors.push(t);
            }
        }
    }
    if floors.is_empty() {
        return false;
    }
    let to = floors[world.resource_mut::<GameRng>().range(0, floors.len() as i32 - 1) as usize];
    world.resource_mut::<SparkShowers>().0.push((here, 3));
    world.entity_mut(mob).insert(GridPos(to));
    world.resource_mut::<SparkShowers>().0.push((to, 3));
    true
}

/// The sixteen tiles two out from a place, all round.
fn ring(center: TilePos) -> Vec<TilePos> {
    let mut out = Vec::new();
    for dy in -2..=2_i32 {
        for dx in -2..=2_i32 {
            if dx.abs() == 2 || dy.abs() == 2 {
                out.push(TilePos::new(center.x + dx, center.y + dy, center.z));
            }
        }
    }
    out
}

/// Put the force field up round a place, or move it there.
fn place_field(world: &mut World, e: &mut Effect, center: TilePos) {
    if e.field.len() != 16 {
        clear_field(world, e);
        for tile in ring(center) {
            let id = world.resource_mut::<NetIds>().alloc();
            let tile_entity = world
                .spawn((
                    GridPos(tile),
                    id,
                    Dense,
                    Appearance::new(FIELD, "shieldsparkles0"),
                    ArtifactField,
                ))
                .id();
            e.field.push(tile_entity);
        }
    } else if e.field_at != Some(center) {
        for (tile_entity, tile) in e.field.clone().into_iter().zip(ring(center)) {
            world.entity_mut(tile_entity).insert(GridPos(tile));
        }
    }
    e.field_at = Some(center);
}

fn clear_field(world: &mut World, e: &mut Effect) {
    for tile in e.field.drain(..) {
        if let Ok(entity) = world.get_entity_mut(tile) {
            entity.despawn();
        }
    }
    e.field_at = None;
}

/// Which of a holder's effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Which {
    Primary,
    Secondary,
    Battery,
}

fn effect_of(world: &World, holder: Entity, which: Which) -> Option<Effect> {
    match which {
        Which::Primary => world.get::<Artifact>(holder).map(|a| a.primary.clone()),
        Which::Secondary => world.get::<Artifact>(holder).and_then(|a| a.secondary.clone()),
        Which::Battery => world.get::<AnomalyBattery>(holder).and_then(|b| b.effect.clone()),
    }
}

fn set_effect(world: &mut World, holder: Entity, which: Which, e: Effect) {
    match which {
        Which::Primary => {
            if let Some(mut a) = world.get_mut::<Artifact>(holder) {
                a.primary = e;
            }
        }
        Which::Secondary => {
            if let Some(mut a) = world.get_mut::<Artifact>(holder) {
                a.secondary = Some(e);
            }
        }
        Which::Battery => {
            if let Some(mut b) = world.get_mut::<AnomalyBattery>(holder) {
                b.effect = Some(e);
            }
        }
    }
}

/// What shows when an effect turns on or off: the artifact, or the device its battery is in,
/// wherever it is or whoever carries it.
fn shown_as(world: &World, holder: Entity) -> Option<(TilePos, String)> {
    let mut shown = holder;
    let mut e = holder;
    for _ in 0..4 {
        if world.get::<Body>(e).is_none() {
            shown = e;
        }
        if let Some(p) = at(world, e) {
            return Some((p, sprite(world, shown)?));
        }
        e = world
            .get::<InDevice>(e)
            .map(|d| d.0)
            .or_else(|| world.get::<HeldBy>(e).map(|h| h.0))?;
    }
    None
}

/// Luna `ToggleActivate`: on or off, and if `reveal`, a glimmer anyone near can see.
fn toggle(world: &mut World, holder: Entity, which: Which, reveal: bool) {
    let Some(mut e) = effect_of(world, holder, which) else {
        return;
    };
    e.active = !e.active;
    if e.kind == Kind::ForceField {
        if e.field.is_empty() {
            if let Some(center) = where_is(world, holder) {
                place_field(world, &mut e, center);
            }
        } else {
            clear_field(world, &mut e);
        }
    }
    let active = e.active;
    set_effect(world, holder, which, e);
    if reveal {
        if let Some(look) = world.get::<Artifact>(holder).map(|a| a.look) {
            let state = format!("ano{look}{}", u8::from(active));
            world.entity_mut(holder).insert(Appearance::new(ARTIFACT, &state));
        }
        if let Some((tile, name)) = shown_as(world, holder) {
            let msg = if active {
                one_of(world, "anomaly-on", 5)
            } else {
                one_of(world, "anomaly-off", 4)
            };
            around(world, tile, msg.arg("item", name));
        }
    }
}

fn set_active(world: &mut World, holder: Entity, which: Which, active: bool, reveal: bool) {
    if effect_of(world, holder, which).is_some_and(|e| e.active != active) {
        toggle(world, holder, which, reveal);
    }
}

/// Luna `artifact_effect/process`: charge up, and work if on.
fn process_effect(world: &mut World, holder: Entity, which: Which, here: TilePos) {
    let Some(mut e) = effect_of(world, holder, which) else {
        return;
    };
    if e.charge < e.charge_max {
        e.charge += 1;
    }
    if e.active {
        match e.mode {
            Mode::Aura => aura(world, &mut e, here),
            Mode::Pulse if e.charge >= e.charge_max => {
                e.charge = 0;
                pulse(world, &mut e, here);
            }
            _ => {}
        }
    }
    // The effect may have been changed while it worked: keep what it did.
    set_effect(world, holder, which, e);
}

fn touch_with(world: &mut World, holder: Entity, which: Which, user: Entity) {
    let Some(mut e) = effect_of(world, holder, which) else {
        return;
    };
    let Some(here) = where_is(world, holder) else {
        return;
    };
    touch_effect(world, &mut e, user, here);
    set_effect(world, holder, which, e);
}

// --- The artifact ---

fn artifact_triggers(world: &World, artifact: Entity) -> Option<(Trigger, Option<Trigger>)> {
    let a = world.get::<Artifact>(artifact)?;
    Some((a.primary.trigger, a.secondary.as_ref().map(|s| s.trigger)))
}

/// Something that sets off triggers of these kinds: the primary always, a secondary one time in
/// four.
fn set_off(world: &mut World, artifact: Entity, kinds: &[Trigger]) {
    let Some((primary, secondary)) = artifact_triggers(world, artifact) else {
        return;
    };
    if kinds.contains(&primary) {
        toggle(world, artifact, Which::Primary, true);
    }
    if secondary.is_some_and(|s| kinds.contains(&s)) && roll(world, 25.0) {
        toggle(world, artifact, Which::Secondary, false);
    }
}

fn has_gloves(world: &World, mob: Entity) -> bool {
    worn(world, mob, Slot::Gloves).is_some()
}

/// Luna `artifact/attack_hand`.
fn touch(world: &mut World, mob: Entity, artifact: Entity) {
    if has_gloves(world, mob) {
        tell(
            world,
            mob,
            LocMsg::new("anomaly-touch-gloved").arg("item", ARTIFACT),
        );
        return;
    }
    let Some(a) = world.get::<Artifact>(artifact).cloned() else {
        return;
    };
    if a.primary.trigger == Trigger::Touch {
        tell(world, mob, LocMsg::new("anomaly-touch").arg("item", ARTIFACT));
        toggle(world, artifact, Which::Primary, true);
    } else {
        tell(
            world,
            mob,
            LocMsg::new("anomaly-touch-nothing").arg("item", ARTIFACT),
        );
    }
    if a.secondary.as_ref().is_some_and(|s| s.trigger == Trigger::Touch) && roll(world, 25.0) {
        toggle(world, artifact, Which::Secondary, false);
    }
    if a.primary.mode == Mode::Touch {
        touch_with(world, artifact, Which::Primary, mob);
    }
    if effect_of(world, artifact, Which::Secondary).is_some_and(|s| s.active && s.mode == Mode::Touch) {
        touch_with(world, artifact, Which::Secondary, mob);
    }
}

/// Luna `artifact/Bumped`, by someone walking into it bare-handed.
fn bumped(world: &mut World, mob: Entity, artifact: Entity) {
    if !crate::genetics::is_person(world, mob) || has_gloves(world, mob) {
        return;
    }
    let Some(a) = world.get::<Artifact>(artifact).cloned() else {
        return;
    };
    let mut warn = false;
    if a.primary.trigger == Trigger::Touch && roll(world, 50.0) {
        toggle(world, artifact, Which::Primary, true);
        warn = true;
    }
    if a.secondary.as_ref().is_some_and(|s| s.trigger == Trigger::Touch) && roll(world, 25.0) {
        toggle(world, artifact, Which::Secondary, false);
        warn = true;
    }
    if a.primary.mode == Mode::Touch && roll(world, 50.0) {
        touch_with(world, artifact, Which::Primary, mob);
        warn = true;
    }
    if effect_of(world, artifact, Which::Secondary).is_some_and(|s| s.active && s.mode == Mode::Touch)
        && roll(world, 50.0)
    {
        touch_with(world, artifact, Which::Secondary, mob);
        warn = true;
    }
    if warn {
        tell(
            world,
            mob,
            LocMsg::new("anomaly-touch-accident").arg("item", ARTIFACT),
        );
    }
}

/// What an item does to an artifact's triggers (Luna `artifact/attackby`).
fn used_on_artifact(world: &mut World, artifact: Entity, item: Entity) {
    if let Some(r) = world.get::<crate::chemistry::Reagents>(item) {
        let any = |ids: &[&str]| ids.iter().any(|id| r.get(id) >= 1.0);
        let kind = if any(&["hydrogen", "water"]) {
            Some(Trigger::Water)
        } else if any(&["acid", "pacid", "diethylamine"]) {
            Some(Trigger::Acid)
        } else if any(&["plasma", "thermite"]) {
            Some(Trigger::Volatile)
        } else if any(&["toxin", "cyanide", "amanitin", "neurotoxin"]) {
            Some(Trigger::Toxin)
        } else {
            None
        };
        if let Some(kind) = kind {
            set_off(world, artifact, &[kind]);
        }
        return;
    }
    let proto = sprite(world, item).unwrap_or_default();
    let energetic = world.get::<crate::weapons::Baton>(item).is_some_and(|b| b.on) || proto == "multitool";
    let hot = world.get::<crate::welding::Welder>(item).is_some_and(|w| w.lit)
        || world.get::<crate::smoking::Lighter>(item).is_some_and(|l| l.lit)
        || matches!(
            world.get::<crate::smoking::Match>(item),
            Some(crate::smoking::Match::Lit(_))
        );
    if energetic {
        set_off(world, artifact, &[Trigger::Energy]);
    } else if hot {
        set_off(world, artifact, &[Trigger::Heat]);
    } else {
        // A hard enough blow sets off the primary; any blow may set off a secondary.
        let force = world.get::<Item>(item).map_or(0.0, |i| i.force);
        let Some((primary, secondary)) = artifact_triggers(world, artifact) else {
            return;
        };
        if primary == Trigger::Force && force >= FORCE {
            toggle(world, artifact, Which::Primary, true);
        }
        if secondary == Some(Trigger::Force) && roll(world, 25.0) {
            toggle(world, artifact, Which::Secondary, false);
        }
    }
}

/// A shot that hit an artifact standing in its way (Luna `artifact/bullet_act`).
pub fn shot_into(world: &mut World, tile: TilePos, bullet: bool) {
    let mut q = world.query_filtered::<(Entity, &GridPos), With<Artifact>>();
    let hit: Vec<Entity> = q
        .iter(world)
        .filter(|(_, p)| p.0 == tile)
        .map(|(e, _)| e)
        .collect();
    let kind = if bullet { Trigger::Force } else { Trigger::Energy };
    for artifact in hit {
        set_off(world, artifact, &[kind]);
    }
}

/// An artifact caught in a blast (Luna `artifact/ex_act`). Whether it's gone.
pub fn blasted(world: &mut World, artifact: Entity, severity: u8) -> bool {
    let gone = match severity {
        1 => true,
        2 => roll(world, 50.0),
        _ => false,
    };
    if gone {
        for which in [Which::Primary, Which::Secondary] {
            if let Some(mut e) = effect_of(world, artifact, which) {
                clear_field(world, &mut e);
            }
        }
        world.entity_mut(artifact).despawn();
    } else {
        set_off(world, artifact, &[Trigger::Force, Trigger::Heat]);
    }
    gone
}

/// Turn air triggers on or off with the air around the artifact.
fn air_triggers(world: &mut World, artifact: Entity, here: TilePos) {
    let Some(atmos) = world.get_resource::<StationAtmos>() else {
        return;
    };
    let air = atmos.0.air_at(here);
    let holds = |trigger: Trigger| match trigger {
        Trigger::Cold => Some(air.temperature < COLD_K),
        Trigger::Heat => Some(air.temperature > HOT_K),
        Trigger::Plasma => Some(air.get(Gas::Plasma) >= GAS_MOLES),
        Trigger::Oxygen => Some(air.get(Gas::Oxygen) >= GAS_MOLES),
        Trigger::CarbonDioxide => Some(air.get(Gas::CarbonDioxide) >= GAS_MOLES),
        Trigger::Nitrogen => Some(air.get(Gas::Nitrogen) >= GAS_MOLES),
        _ => None,
    };
    let Some((primary, secondary)) = artifact_triggers(world, artifact) else {
        return;
    };
    let (p, s) = (holds(primary), secondary.and_then(holds));
    if let Some(on) = p {
        set_active(world, artifact, Which::Primary, on, true);
    }
    if let Some(on) = s {
        set_active(world, artifact, Which::Secondary, on, false);
    }
}

// --- Clicks ---

/// A click on the artifact, the analyser or the harvester, in reach, with whatever is in hand.
pub fn click(world: &mut World, mob: Entity, target: Entity, held: Option<Entity>) {
    if !awake(world, mob) {
        return;
    }
    let (Some(here), Some(there)) = (at(world, mob), at(world, target)) else {
        return;
    };
    if !in_reach(here, there) {
        return;
    }
    match sprite(world, target).as_deref() {
        Some(ARTIFACT) => match held {
            None => touch(world, mob, target),
            Some(item) => used_on_artifact(world, target, item),
        },
        Some(ANALYSER) => {
            if powered(world, target) {
                open(world, mob, target);
            }
        }
        Some(HARVESTER) => {
            if let Some(item) = held.filter(|i| world.get::<AnomalyBattery>(*i).is_some()) {
                insert_battery(world, mob, target, item);
            } else if powered(world, target) {
                open(world, mob, target);
            }
        }
        _ => {}
    }
}

fn open(world: &mut World, mob: Entity, target: Entity) {
    world
        .entity_mut(mob)
        .insert((PanelViewer(target), PickListOut::default()));
}

/// The utilizer used in the hand, or a battery used on it.
pub fn use_utilizer(world: &mut World, mob: Entity, utilizer: Entity, held: Option<Entity>) {
    if !awake(world, mob) {
        return;
    }
    match held.filter(|i| *i != utilizer) {
        Some(item) if world.get::<AnomalyBattery>(item).is_some() => {
            insert_battery(world, mob, utilizer, item)
        }
        Some(_) => {}
        None => open(world, mob, utilizer),
    }
}

/// A battery into a harvester or a utilizer.
fn insert_battery(world: &mut World, mob: Entity, device: Entity, battery: Entity) {
    let taken = world
        .get::<Harvester>(device)
        .map(|h| h.battery)
        .or_else(|| world.get::<Utilizer>(device).map(|u| u.battery));
    match taken {
        Some(Some(_)) => {
            tell(
                world,
                mob,
                LocMsg::new("anomaly-battery-full-slot")
                    .arg("item", sprite(world, device).unwrap_or_default()),
            );
        }
        Some(None) => {
            if let Some(mut hands) = world.get_mut::<Hands>(mob) {
                for hand in [rs13_world::Hand::Left, rs13_world::Hand::Right] {
                    if hands.get(hand) == Some(battery) {
                        *hands.slot_mut(hand) = None;
                    }
                }
            }
            world
                .entity_mut(battery)
                .remove::<(HeldBy, GridPos)>()
                .insert(InDevice(device));
            if let Some(mut h) = world.get_mut::<Harvester>(device) {
                h.battery = Some(battery);
            }
            if let Some(mut u) = world.get_mut::<Utilizer>(device) {
                u.battery = Some(battery);
            }
            tell(
                world,
                mob,
                LocMsg::new("anomaly-battery-inserted")
                    .arg("item", sprite(world, device).unwrap_or_default()),
            );
            refresh_sprites(world, device);
        }
        None => {}
    }
}

/// The battery out onto the floor where the device is.
fn eject(world: &mut World, device: Entity) {
    let battery = world
        .get::<Harvester>(device)
        .and_then(|h| h.battery)
        .or_else(|| world.get::<Utilizer>(device).and_then(|u| u.battery));
    let Some(battery) = battery else {
        return;
    };
    if let Some(mut h) = world.get_mut::<Harvester>(device) {
        h.battery = None;
    }
    if let Some(mut u) = world.get_mut::<Utilizer>(device) {
        u.battery = None;
    }
    world.entity_mut(battery).remove::<InDevice>();
    if let Some(tile) = where_is(world, device) {
        world.entity_mut(battery).insert(GridPos(tile));
    }
    refresh_sprites(world, device);
}

/// A battery's and a utilizer's look follow the charge (Luna `UpdateSprite`).
fn refresh_sprites(world: &mut World, device: Entity) {
    let battery = world
        .get::<Utilizer>(device)
        .map(|u| u.battery)
        .or_else(|| world.get::<Harvester>(device).map(|h| h.battery))
        .flatten();
    let level =
        |b: &AnomalyBattery| (((b.charge / b.capacity * 100.0).min(100.0) / 25.0).round() * 25.0) as u32;
    if let Some(b) = battery
        && let Some(lvl) = world.get::<AnomalyBattery>(b).map(level)
    {
        let state = format!("anobattery{lvl}");
        if world.get::<Appearance>(b).is_some_and(|a| a.state != state) {
            world.entity_mut(b).insert(Appearance::new(BATTERY, &state));
        }
    }
    if world.get::<Utilizer>(device).is_some() {
        let state = match battery.and_then(|b| world.get::<AnomalyBattery>(b)) {
            Some(b) => format!("anodev{}", level(b)),
            None => "anodev".to_owned(),
        };
        if world.get::<Appearance>(device).is_some_and(|a| a.state != state) {
            world.entity_mut(device).insert(Appearance::new(UTILIZER, &state));
        }
    }
}

// --- The lists ---

/// What a line of a list does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Line {
    Info,
    BeginScan,
    HaltScan,
    Harvest,
    Halt,
    Eject,
    Drain,
    Start,
    StartTimed,
    Shutdown,
    Time(i32),
}

fn battery_of(world: &World, device: Entity) -> Option<(Entity, AnomalyBattery)> {
    let b = world
        .get::<Harvester>(device)
        .and_then(|h| h.battery)
        .or_else(|| world.get::<Utilizer>(device).and_then(|u| u.battery))?;
    Some((b, world.get::<AnomalyBattery>(b)?.clone()))
}

fn percent(b: &AnomalyBattery) -> String {
    format!("{:.0}", b.charge / b.capacity * 100.0)
}

fn lines(world: &World, target: Entity) -> Vec<(String, Line)> {
    let msg = |id: &str| format!("msg:{id}");
    let val = |id: &str, v: String| format!("msg:{id}:{v}");
    let mut out = Vec::new();
    if let Some(a) = world.get::<Analyser>(target) {
        if a.scanning.is_some() {
            out.push((msg("anomaly-row-scanning"), Line::Info));
            out.push((msg("anomaly-row-halt-scan"), Line::HaltScan));
        } else {
            out.push((msg("anomaly-row-ready"), Line::Info));
            out.push((msg("anomaly-row-begin-scan"), Line::BeginScan));
        }
    } else if let Some(h) = world.get::<Harvester>(target) {
        let battery = battery_of(world, target);
        match (h.mode, battery) {
            (HarvestMode::Harvesting, Some((_, b))) => {
                out.push((val("anomaly-row-harvesting", percent(&b)), Line::Info));
                out.push((msg("anomaly-row-halt"), Line::Halt));
            }
            (HarvestMode::Dumping, Some((_, b))) => {
                out.push((val("anomaly-row-dumping", percent(&b)), Line::Info));
                out.push((msg("anomaly-row-halt"), Line::Halt));
            }
            (_, Some((_, b))) => {
                let charge = format!("{:.0}/{:.0} ({}%)", b.charge, b.capacity, percent(&b));
                out.push((val("anomaly-row-charge", charge), Line::Info));
                let id = b.effect.as_ref().map_or("???".to_owned(), |e| e.id.clone());
                out.push((val("anomaly-row-signature", id), Line::Info));
                out.push((msg("anomaly-row-eject"), Line::Eject));
                out.push((msg("anomaly-row-drain"), Line::Drain));
                out.push((msg("anomaly-row-harvest"), Line::Harvest));
            }
            (_, None) => out.push((msg("anomaly-row-no-battery"), Line::Info)),
        }
    } else if let Some(u) = world.get::<Utilizer>(target) {
        match battery_of(world, target) {
            Some((_, b)) => {
                let status = if u.cooldown > 0 {
                    "anomaly-util-cooldown"
                } else if u.active && u.timing {
                    "anomaly-util-active-timed"
                } else if u.active {
                    "anomaly-util-active"
                } else {
                    "anomaly-util-ready"
                };
                out.push((msg(status), Line::Info));
                let id = b.effect.as_ref().map_or("NA".to_owned(), |e| e.id.clone());
                out.push((val("anomaly-util-id", id), Line::Info));
                out.push((
                    val("anomaly-util-power", format!("{:.0}/{:.0}", b.charge, b.capacity)),
                    Line::Info,
                ));
                out.push((
                    val("anomaly-util-time", format!("{:.1}", u.time as f32 / 10.0)),
                    Line::Info,
                ));
                for (id, step) in [
                    ("anomaly-util-time-down-more", -100),
                    ("anomaly-util-time-down", -10),
                    ("anomaly-util-time-up", 10),
                    ("anomaly-util-time-up-more", 100),
                ] {
                    out.push((msg(id), Line::Time(step)));
                }
                if u.cooldown == 0 {
                    if u.active {
                        out.push((msg("anomaly-util-shutdown"), Line::Shutdown));
                    } else {
                        out.push((msg("anomaly-util-start"), Line::Start));
                        out.push((msg("anomaly-util-start-timed"), Line::StartTimed));
                    }
                }
                out.push((msg("anomaly-row-eject"), Line::Eject));
            }
            None => out.push((msg("anomaly-util-insert"), Line::Info)),
        }
    }
    out
}

/// Whether the filing cabinets should leave this viewer's list alone.
pub fn keeps_own_list(sprite: &str) -> bool {
    matches!(sprite, ANALYSER | HARVESTER | UTILIZER)
}

/// Whether someone can still work the thing: a machine in reach and powered, or a utilizer held.
fn can_work(world: &World, mob: Entity, target: Entity) -> bool {
    if world.get::<Utilizer>(target).is_some() {
        return world.get::<HeldBy>(target).is_some_and(|h| h.0 == mob);
    }
    let near = at(world, mob)
        .zip(at(world, target))
        .is_some_and(|(a, b)| in_reach(a, b));
    near && powered(world, target)
}

/// Picks in our lists; the rest are left for the other lists.
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
            if sprite(world, target).as_deref().is_some_and(keeps_own_list) {
                if can_work(world, mob, target) && awake(world, mob) {
                    let chosen = lines(world, target).get(line).map(|(_, l)| *l);
                    if let Some(chosen) = chosen {
                        do_line(world, target, chosen);
                    }
                }
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

fn do_line(world: &mut World, target: Entity, line: Line) {
    match line {
        Line::Info => {}
        Line::BeginScan => begin_scan(world, target),
        Line::HaltScan => halt_scan(world, target),
        Line::Harvest => begin_harvest(world, target),
        Line::Halt => halt_harvest(world, target),
        Line::Drain => drain(world, target),
        Line::Eject => {
            if world.get::<Harvester>(target).is_some() {
                halt_harvest(world, target);
            } else {
                shutdown(world, target);
            }
            eject(world, target);
        }
        Line::Start | Line::StartTimed => {
            let Some(mut u) = world.get::<Utilizer>(target).copied() else {
                return;
            };
            if u.cooldown > 0 || u.active {
                return;
            }
            let Some((battery, _)) = battery_of(world, target).filter(|(_, b)| b.effect.is_some()) else {
                return;
            };
            u.active = true;
            if line == Line::StartTimed {
                u.timing = true;
                u.archived = u.time;
            }
            world.entity_mut(target).insert(u);
            set_active(world, battery, Which::Battery, true, true);
        }
        Line::Shutdown => shutdown(world, target),
        Line::Time(step) => {
            let capacity = battery_of(world, target).map_or(BATTERY_CAPACITY, |(_, b)| b.capacity) as i32;
            if let Some(mut u) = world.get_mut::<Utilizer>(target) {
                u.time = (u.time as i32 + step).clamp(0, capacity) as u32;
            }
        }
    }
}

// --- The analyser ---

/// What the analyser would scan on its pad: an artifact if there is one, or the first other thing.
fn on_pad(world: &mut World, tile: TilePos) -> Option<Entity> {
    let mut q = world.query_filtered::<(Entity, &GridPos, Has<Artifact>), (
        Without<Hidden>,
        Without<Mob>,
        Without<Ghost>,
        Without<ArtifactField>,
    )>();
    let mut here: Vec<(Entity, bool)> = q
        .iter(world)
        .filter(|(_, p, _)| p.0 == tile)
        .map(|(e, _, art)| (e, art))
        .collect();
    here.sort_by_key(|(e, art)| (!*art, *e));
    here.first().map(|(e, _)| *e)
}

fn set_busy(world: &mut World, artifact: Entity, busy: bool) {
    if let Some(mut a) = world.get_mut::<Artifact>(artifact) {
        a.busy = busy;
    }
}

fn begin_scan(world: &mut World, analyser: Entity) {
    let Some(tile) = at(world, analyser) else {
        return;
    };
    if world
        .get::<Analyser>(analyser)
        .is_some_and(|a| a.scanning.is_some())
    {
        return;
    }
    let Some(target) = on_pad(world, pad(tile)) else {
        states(world, analyser, "anomaly-says-no-target");
        return;
    };
    if world.get::<Artifact>(target).is_some_and(|a| a.busy) {
        states(world, analyser, "anomaly-says-interference");
        return;
    }
    set_busy(world, target, true);
    let until = world.resource::<WorldTime>().deciseconds() + SCAN_DS;
    if let Some(mut a) = world.get_mut::<Analyser>(analyser) {
        a.scanning = Some((target, until));
    }
    states(world, analyser, "anomaly-says-scan-begun");
}

fn halt_scan(world: &mut World, analyser: Entity) {
    let Some(target) = world
        .get_mut::<Analyser>(analyser)
        .and_then(|mut a| a.scanning.take())
    else {
        return;
    };
    set_busy(world, target.0, false);
    states(world, analyser, "anomaly-says-scan-halted");
}

/// The words for what an effect gives off, how, and what sets it off.
fn describe(e: &Effect, secondary: bool) -> String {
    let emission = if e.emission <= 7 { e.emission } else { 0 };
    let emission = if emission == 0 && secondary {
        "anomaly-emission-0-aux".to_owned()
    } else {
        format!("anomaly-emission-{emission}")
    };
    let how = match e.emission {
        1 => "ambient",
        2 => "bursts",
        _ => "throughout",
    };
    let trigger = if secondary {
        format!("anomaly-trigger-{}-aux", e.trigger.class())
    } else {
        format!("anomaly-trigger-{}", e.trigger.class())
    };
    format!("[msg:anomaly-report-detected] [msg:{emission}], [msg:anomaly-delivery-{how}] [msg:{trigger}]")
}

/// Luna `get_scan_info`.
fn report(world: &mut World, target: Entity) -> String {
    if let Some(a) = world.get::<Artifact>(target).cloned() {
        let mut out = format!("[msg:anomaly-report-device]\n{}", describe(&a.primary, false));
        if let Some(s) = a.secondary.filter(|s| s.active) {
            let n = world.resource_mut::<GameRng>().range(1, 3);
            out.push_str(&format!(
                "\n\n[msg:anomaly-report-aux-{n}] {}",
                describe(&s, true)
            ));
        }
        return out;
    }
    let proto = sprite(world, target).unwrap_or_default();
    if proto == crate::supermatter::SUPERMATTER {
        "[msg:anomaly-report-supermatter]".to_owned()
    } else {
        format!("[noun:{proto}] - [msg:anomaly-report-mundane]")
    }
}

fn finish_scans(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    let mut q = world.query::<(Entity, &GridPos, &Analyser)>();
    let done: Vec<(Entity, TilePos, Entity)> = q
        .iter(world)
        .filter_map(|(e, p, a)| {
            a.scanning
                .filter(|(_, until)| now >= *until)
                .map(|(t, _)| (e, p.0, t))
        })
        .collect();
    for (analyser, tile, target) in done {
        let n = {
            let Some(mut a) = world.get_mut::<Analyser>(analyser) else {
                continue;
            };
            a.scanning = None;
            a.reports += 1;
            a.reports
        };
        let results = if at(world, target) == Some(pad(tile)) {
            report(world, target)
        } else {
            "[msg:anomaly-report-lost]".to_owned()
        };
        states(world, analyser, "anomaly-says-scan-complete");
        let mut sheet = ObjectSpec::new(PAPER);
        sheet
            .vars
            .insert("name".to_owned(), format!("[msg:anomaly-report-name] #{n}"));
        sheet.vars.insert(
            "info".to_owned(),
            format!("[b][msg:anomaly-report-title] #{n}[/b]\n\n{results}"),
        );
        // Paper is always a known prototype.
        let _ = crate::objects::spawn_object(world, &sheet, tile);
        if world.get_entity(target).is_ok() {
            set_busy(world, target, false);
        }
    }
}

// --- The harvester ---

fn set_watts(world: &mut World, machine: Entity, watts: f64) {
    if let Some(mut p) = world.get_mut::<PowerUser>(machine) {
        p.watts = watts;
    }
}

/// Luna `harvester/Topic` `harvest`.
fn begin_harvest(world: &mut World, harvester: Entity) {
    let Some(tile) = at(world, harvester) else {
        return;
    };
    if world
        .get::<Harvester>(harvester)
        .is_some_and(|h| h.mode != HarvestMode::Idle)
    {
        return;
    }
    let pad = pad(tile);
    let mut q = world.query_filtered::<
        (Entity, &GridPos, Has<Artifact>, Has<Mob>),
        (Without<Hidden>, Without<Ghost>, Without<ArtifactField>),
    >();
    let here: Vec<(Entity, bool, bool)> = q
        .iter(world)
        .filter(|(_, p, ..)| p.0 == pad)
        .map(|(e, _, a, m)| (e, a, m))
        .collect();
    let artifacts: Vec<Entity> = here.iter().filter(|(_, a, _)| *a).map(|(e, ..)| *e).collect();
    let mundane = here.iter().any(|(_, a, _)| !*a);
    let Some(&artifact) = artifacts.first() else {
        states(world, harvester, "anomaly-says-no-signature");
        return;
    };
    if world.get::<Artifact>(artifact).is_some_and(|a| a.busy) {
        states(world, harvester, "anomaly-says-interference");
        return;
    }
    if artifacts.len() > 1 || mundane {
        states(world, harvester, "anomaly-says-isolating");
        return;
    }
    let Some((battery, b)) = battery_of(world, harvester) else {
        states(world, harvester, "anomaly-says-no-battery");
        return;
    };
    let Some(source) = world.get::<Artifact>(artifact).map(|a| a.primary.clone()) else {
        return;
    };
    // An empty battery forgets what it held.
    let held = b.effect.filter(|_| b.charge > 0.0);
    if held
        .as_ref()
        .is_some_and(|e| e.id != source.id || e.kind != source.kind)
    {
        states(world, harvester, "anomaly-says-incompatible");
        return;
    }
    if let Some(mut stored) = world.get_mut::<AnomalyBattery>(battery)
        && held.is_none()
    {
        stored.effect = Some(source.copy());
        stored.charge = 0.0;
    }
    set_busy(world, artifact, true);
    if let Some(mut h) = world.get_mut::<Harvester>(harvester) {
        h.mode = HarvestMode::Harvesting;
        h.artifact = Some(artifact);
    }
    set_watts(world, harvester, HARVESTER_ACTIVE_W);
    states(world, harvester, "anomaly-says-harvesting");
}

fn release(world: &mut World, harvester: Entity) {
    let artifact = world.get_mut::<Harvester>(harvester).and_then(|mut h| {
        h.mode = HarvestMode::Idle;
        h.artifact.take()
    });
    if let Some(artifact) = artifact
        && world.get_entity(artifact).is_ok()
    {
        set_busy(world, artifact, false);
    }
    set_watts(world, harvester, HARVESTER_IDLE_W);
}

fn halt_harvest(world: &mut World, harvester: Entity) {
    let Some(h) = world.get::<Harvester>(harvester).copied() else {
        return;
    };
    if h.mode == HarvestMode::Idle {
        return;
    }
    if h.mode == HarvestMode::Dumping
        && let Some((battery, _)) = battery_of(world, harvester)
    {
        set_active(world, battery, Which::Battery, false, true);
    }
    release(world, harvester);
    states(world, harvester, "anomaly-says-interrupted");
}

fn drain(world: &mut World, harvester: Entity) {
    if world
        .get::<Harvester>(harvester)
        .is_some_and(|h| h.mode != HarvestMode::Idle)
    {
        return;
    }
    match battery_of(world, harvester) {
        Some((battery, b)) if b.effect.is_some() && b.charge > 0.0 => {
            set_active(world, battery, Which::Battery, true, false);
            if let Some(mut h) = world.get_mut::<Harvester>(harvester) {
                h.mode = HarvestMode::Dumping;
            }
            set_watts(world, harvester, HARVESTER_ACTIVE_W);
            states(world, harvester, "anomaly-says-dumping");
        }
        Some(_) => states(world, harvester, "anomaly-says-already-drained"),
        None => states(world, harvester, "anomaly-says-no-battery-dump"),
    }
}

fn run_harvesters(world: &mut World) {
    let mut q = world.query::<(Entity, &GridPos, &Harvester, Option<&PowerUser>)>();
    let all: Vec<(Entity, TilePos, Harvester, bool)> = q
        .iter(world)
        .map(|(e, p, h, power)| (e, p.0, *h, power.is_none_or(|p| p.powered)))
        .collect();
    for (harvester, tile, h, powered) in all {
        let look = if powered { "" } else { "nopower" };
        if world
            .get::<Appearance>(harvester)
            .is_some_and(|a| a.state != look)
        {
            world
                .entity_mut(harvester)
                .insert(Appearance::new(HARVESTER, look));
        }
        if !powered {
            continue;
        }
        let Some((battery, b)) = battery_of(world, harvester) else {
            if h.mode != HarvestMode::Idle {
                release(world, harvester);
            }
            continue;
        };
        match h.mode {
            HarvestMode::Idle => {}
            HarvestMode::Harvesting => {
                let full = {
                    let Some(mut stored) = world.get_mut::<AnomalyBattery>(battery) else {
                        continue;
                    };
                    stored.charge = (stored.charge + HARVEST_RATE).min(stored.capacity);
                    stored.charge >= stored.capacity
                };
                if full {
                    release(world, harvester);
                    states(world, harvester, "anomaly-says-battery-full");
                }
            }
            HarvestMode::Dumping => {
                if let Some(mut stored) = world.get_mut::<AnomalyBattery>(battery) {
                    stored.charge -= DUMP_RATE;
                }
                if b.effect.is_some() {
                    process_effect(world, battery, Which::Battery, tile);
                    // A touch effect works on whoever has the harvester's list open beside it.
                    if b.effect.as_ref().is_some_and(|e| e.mode == Mode::Touch) {
                        let mut viewers = world.query::<(Entity, &PanelViewer, &GridPos)>();
                        let near: Vec<Entity> = viewers
                            .iter(world)
                            .filter(|(_, v, p)| v.0 == harvester && in_reach(p.0, tile))
                            .map(|(m, ..)| m)
                            .collect();
                        for mob in near {
                            touch_with(world, battery, Which::Battery, mob);
                        }
                    }
                }
                if world
                    .get::<AnomalyBattery>(battery)
                    .is_some_and(|s| s.charge <= 0.0)
                {
                    if let Some(mut stored) = world.get_mut::<AnomalyBattery>(battery) {
                        stored.charge = 0.0;
                    }
                    set_active(world, battery, Which::Battery, false, true);
                    release(world, harvester);
                    states(world, harvester, "anomaly-says-dump-complete");
                }
            }
        }
        refresh_sprites(world, harvester);
    }
}

// --- The utilizer ---

fn shutdown(world: &mut World, utilizer: Entity) {
    let Some(mut u) = world.get::<Utilizer>(utilizer).copied() else {
        return;
    };
    if !u.active {
        return;
    }
    u.active = false;
    u.timing = false;
    u.cooldown = u.archived / 2;
    world.entity_mut(utilizer).insert(u);
    if let Some(tile) = where_is(world, utilizer) {
        around(
            world,
            tile,
            LocMsg::new("anomaly-util-buzzes").arg("item", UTILIZER),
        );
    }
    if let Some((battery, _)) = battery_of(world, utilizer) {
        set_active(world, battery, Which::Battery, false, false);
    }
}

fn run_utilizers(world: &mut World) {
    let mut q = world.query_filtered::<Entity, With<Utilizer>>();
    let all: Vec<Entity> = q.iter(world).collect();
    for utilizer in all {
        let Some(mut u) = world.get::<Utilizer>(utilizer).copied() else {
            continue;
        };
        let Some(here) = where_is(world, utilizer) else {
            continue;
        };
        if u.cooldown > 0 {
            u.cooldown -= 1;
            world.entity_mut(utilizer).insert(u);
            if u.cooldown == 0 {
                around(
                    world,
                    here,
                    LocMsg::new("anomaly-util-chimes").arg("item", UTILIZER),
                );
            }
            continue;
        }
        if !u.active {
            continue;
        }
        let Some((battery, b)) = battery_of(world, utilizer).filter(|(_, b)| b.effect.is_some()) else {
            shutdown(world, utilizer);
            continue;
        };
        set_active(world, battery, Which::Battery, true, true);
        process_effect(world, battery, Which::Battery, here);
        if b.effect.as_ref().is_some_and(|e| e.mode == Mode::Touch)
            && let Some(holder) = world.get::<HeldBy>(utilizer).map(|h| h.0)
        {
            touch_with(world, battery, Which::Battery, holder);
        }
        let empty = world.get_mut::<AnomalyBattery>(battery).is_some_and(|mut s| {
            s.charge = (s.charge - 1.0).max(0.0);
            s.charge <= 0.0
        });
        let timed_out = u.timing && {
            u.time = u.time.saturating_sub(1);
            world.entity_mut(utilizer).insert(u);
            u.time == 0
        };
        if empty || timed_out {
            shutdown(world, utilizer);
        }
        refresh_sprites(world, utilizer);
    }
}

// --- Every tick ---

/// Fields follow what holds them up: the artifact as it's pulled, a battery as it's carried.
fn follow_fields(world: &mut World) {
    let mut q = world.query::<(Entity, &Artifact)>();
    let mut holders: Vec<(Entity, Which)> = Vec::new();
    for (e, a) in q.iter(world) {
        if !a.primary.field.is_empty() {
            holders.push((e, Which::Primary));
        }
        if a.secondary.as_ref().is_some_and(|s| !s.field.is_empty()) {
            holders.push((e, Which::Secondary));
        }
    }
    let mut q = world.query::<(Entity, &AnomalyBattery)>();
    holders.extend(
        q.iter(world)
            .filter(|(_, b)| b.effect.as_ref().is_some_and(|e| !e.field.is_empty()))
            .map(|(e, _)| (e, Which::Battery)),
    );
    for (holder, which) in holders {
        let (Some(center), Some(mut e)) = (where_is(world, holder), effect_of(world, holder, which)) else {
            continue;
        };
        if e.field_at != Some(center) {
            place_field(world, &mut e, center);
            set_effect(world, holder, which, e);
        }
    }
}

/// Every tick: bumps, finished scans and moving fields; every machine cycle, the artifacts' effects
/// and air triggers, the harvesters and the utilizers.
pub fn run_anomalies(world: &mut World) {
    // Walking into an artifact (the teleporters take the bumps after).
    let bumps = world.resource::<Bumps>().0.clone();
    for (mob, tile) in bumps {
        let mut q = world.query_filtered::<(Entity, &GridPos), With<Artifact>>();
        let hit: Vec<Entity> = q
            .iter(world)
            .filter(|(_, p)| p.0 == tile)
            .map(|(e, _)| e)
            .collect();
        for artifact in hit {
            bumped(world, mob, artifact);
        }
    }
    finish_scans(world);
    follow_fields(world);
    if !world.resource::<WorldTime>().tick.0.is_multiple_of(MACHINE_TICKS) {
        return;
    }
    let mut q = world.query_filtered::<(Entity, &GridPos), With<Artifact>>();
    let artifacts: Vec<(Entity, TilePos)> = q.iter(world).map(|(e, p)| (e, p.0)).collect();
    for (artifact, here) in artifacts {
        process_effect(world, artifact, Which::Primary, here);
        process_effect(world, artifact, Which::Secondary, here);
        air_triggers(world, artifact, here);
    }
    let mut q = world.query::<(Entity, &Analyser, Option<&PowerUser>)>();
    let looks: Vec<(Entity, bool)> = q
        .iter(world)
        .map(|(e, _, p)| (e, p.is_none_or(|p| p.powered)))
        .collect();
    for (analyser, on) in looks {
        let look = if on { "" } else { "nopower" };
        if world.get::<Appearance>(analyser).is_some_and(|a| a.state != look) {
            world.entity_mut(analyser).insert(Appearance::new(ANALYSER, look));
        }
    }
    run_harvesters(world);
    run_utilizers(world);
}

/// Keep each open list in step, and close it once out of reach.
pub fn update_views(world: &mut World) {
    let mut viewers = world.query::<(Entity, &PanelViewer)>();
    let all: Vec<(Entity, Entity)> = viewers.iter(world).map(|(m, v)| (m, v.0)).collect();
    for (mob, target) in all {
        let Some(title) = sprite(world, target).filter(|s| keeps_own_list(s)) else {
            continue;
        };
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
