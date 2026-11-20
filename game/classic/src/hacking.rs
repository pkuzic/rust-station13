//! Hacking machines (Luna's vending, autolathe and APC wires): a screwdriver opens a machine's
//! panel, and wirecutters or a multitool on it open the same wire panel window airlocks use.
//! Spec: `docs/reference/hacking.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, DisplayName, GridPos, MobStat, NetId, ObjectSpec, TilePos, WirePanelView};

use crate::actions::{Outbox, in_reach};
use crate::autolathe::Autolathe;
use crate::combat::VisibleEvents;
use crate::hands::Hands;
use crate::health::Vitals;
use crate::power::{Apc, PowerUser};
use crate::rng::GameRng;
use crate::time::WorldTime;
use crate::vending::Vending;
use crate::wires::{Electrified, PanelViewer, PendingShocks, WireOp};

/// Wire colours, in panel order (Luna's `vendwires`, `apcwires` and the autolathe's `wires`).
/// These are `panel-wire` selectors in the locales.
pub const VENDING_COLOURS: [&str; 4] = ["violet", "orange", "goldenrod", "green"];
pub const APC_COLOURS: [&str; 4] = ["orange", "dark-red", "white", "yellow"];
pub const LATHE_COLOURS: [&str; 10] = [
    "light-red",
    "dark-red",
    "blue",
    "green",
    "yellow",
    "black",
    "white",
    "grey",
    "orange",
    "pink",
];

/// A vending machine's electrify pulse lasts this long, ds (Luna `seconds_electrified = 30`).
pub const VEND_SHOCK_DS: f32 = 300.0;
/// An autolathe pulse undoes itself after this long, ds (Luna `spawn(100)`).
pub const LATHE_PULSE_DS: f32 = 100.0;
/// A pulsed ID wire unlocks an APC for this long, ds (Luna `spawn(300)`).
pub const APC_UNLOCK_DS: f32 = 300.0;
/// A pulsed main wire shorts an APC for this long, ds (Luna `spawn(1200)`).
pub const APC_SHORT_DS: f32 = 1200.0;
/// A pulsed AI wire turns its light off for this long, ds (Luna `spawn(10)`).
pub const APC_AI_DS: f32 = 10.0;
/// How often a vending machine with its throw wire live may throw something, ds, and the
/// chance each time (Luna `process`, `prob(2)`).
pub const THROW_EVERY_DS: f32 = 20.0;
pub const THROW_CHANCE: f32 = 2.0;
/// How far it looks for someone to throw at (Luna `view(5)`).
pub const THROW_RANGE: i32 = 5;
/// Chance of a shock cutting or pulsing a dangerous wire (Luna `shock(user, 50)`).
pub const WIRE_SHOCK: f32 = 50.0;

/// What a machine's wire does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wire {
    // Vending machines (Luna `WIRE_EXTEND`, `WIRE_SCANID`, `WIRE_SHOCK`, `WIRE_SHOOTINV`).
    Contraband,
    ScanId,
    VendShock,
    Throw,
    // The autolathe (Luna `hack_wire`, `disable_wire`, `shock_wire`, and seven that do nothing).
    Hack,
    Disable,
    LatheShock,
    Dud,
    // APCs (Luna `APC_WIRE_*`).
    ApcIdScan,
    ApcMain1,
    ApcMain2,
    ApcAi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Vending,
    Lathe,
    Apc,
}

impl Kind {
    pub fn colours(self) -> &'static [&'static str] {
        match self {
            Kind::Vending => &VENDING_COLOURS,
            Kind::Lathe => &LATHE_COLOURS,
            Kind::Apc => &APC_COLOURS,
        }
    }

    fn wires(self) -> Vec<Wire> {
        match self {
            Kind::Vending => vec![Wire::Contraband, Wire::ScanId, Wire::VendShock, Wire::Throw],
            Kind::Lathe => {
                let mut w = vec![Wire::Hack, Wire::Disable, Wire::LatheShock];
                w.resize(LATHE_COLOURS.len(), Wire::Dud);
                w
            }
            Kind::Apc => vec![Wire::ApcIdScan, Wire::ApcMain1, Wire::ApcMain2, Wire::ApcAi],
        }
    }
}

fn shuffled(rng: &mut GameRng, mut wires: Vec<Wire>) -> Vec<Wire> {
    for i in (1..wires.len()).rev() {
        let j = rng.range(0, i as i32) as usize;
        wires.swap(i, j);
    }
    wires
}

/// Which colour does what in every vending machine and every APC this round (Luna shuffles
/// them once, in `RandomAPCWires`). Each autolathe picks its own.
#[derive(Resource, Debug, Clone)]
pub struct HackLayouts {
    pub vending: Vec<Wire>,
    pub apc: Vec<Wire>,
}

impl FromWorld for HackLayouts {
    fn from_world(world: &mut World) -> Self {
        world.init_resource::<GameRng>();
        let mut rng = world.resource_mut::<GameRng>();
        Self {
            vending: shuffled(&mut rng, Kind::Vending.wires()),
            apc: shuffled(&mut rng, Kind::Apc.wires()),
        }
    }
}

/// A machine's panel and wires, and what they've done to it.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct MachineWires {
    pub kind: Kind,
    pub panel_open: bool,
    /// Colour index → what it does.
    pub layout: Vec<Wire>,
    pub cut: Vec<bool>,
    // Vending machines.
    pub electrified: Electrified,
    pub contraband: bool,
    pub throwing: bool,
    pub scan_id: bool,
    /// The product lock shorted out by an emag (Luna `emagged`, `emag.md`): no access is checked.
    pub emagged: bool,
    pub next_throw_ds: f32,
    // The autolathe.
    pub hacked: bool,
    pub disabled: bool,
    pub shocked: bool,
    // APCs (`shorted` itself is on the `Apc`).
    pub ai_disabled: bool,
    /// Pulses that undo themselves: when, and which wire.
    pub reverts: Vec<(f32, Wire)>,
}

impl MachineWires {
    pub fn new(kind: Kind, layout: Vec<Wire>) -> Self {
        Self {
            kind,
            panel_open: false,
            cut: vec![false; layout.len()],
            layout,
            electrified: Electrified::Off,
            contraband: false,
            throwing: false,
            scan_id: true,
            emagged: false,
            next_throw_ds: 0.0,
            hacked: false,
            disabled: false,
            shocked: false,
            ai_disabled: false,
            reverts: Vec::new(),
        }
    }

    pub fn colour_of(&self, wire: Wire) -> usize {
        self.layout.iter().position(|w| *w == wire).unwrap_or(0)
    }

    pub fn is_cut(&self, wire: Wire) -> bool {
        self.cut.get(self.colour_of(wire)).copied().unwrap_or(false)
    }

    pub fn is_electrified(&self) -> bool {
        self.electrified != Electrified::Off
    }

    /// The panel's lights, as message ids.
    fn lights(&self, apc: Option<&Apc>) -> Vec<String> {
        let on = |b: bool, yes: &str, no: &str| if b { yes } else { no }.to_owned();
        match self.kind {
            // Luna `vending/attack_hand`.
            Kind::Vending => vec![
                on(self.is_electrified(), "light-orange-on", "light-orange-off"),
                on(self.throwing, "light-red-off", "light-red-blinking"),
                on(self.contraband, "light-green-on", "light-green-off"),
                on(self.scan_id, "light-purple-on", "light-yellow-on"),
            ],
            // Luna `autolathe/wires_win`.
            Kind::Lathe => vec![
                on(self.disabled, "light-red-off", "light-red-on"),
                on(self.shocked, "light-green-off", "light-green-on"),
                on(self.hacked, "light-blue-off", "light-blue-on"),
            ],
            // Luna `apc/interact`.
            Kind::Apc => vec![
                on(
                    apc.is_some_and(|a| a.locked),
                    "apc-panel-locked",
                    "apc-panel-unlocked",
                ),
                on(
                    apc.is_some_and(|a| a.shorted),
                    "apc-panel-shorted",
                    "apc-panel-working",
                ),
                on(self.ai_disabled, "panel-ai-light-off", "panel-ai-light-on"),
            ],
        }
    }
}

/// The wire panel a mob has open on a machine (kept apart from `PanelViewer`, which opens the
/// machine's own window).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HackViewer(pub Entity);

fn kind_of(world: &World, machine: Entity) -> Option<Kind> {
    if world.get::<Vending>(machine).is_some() {
        Some(Kind::Vending)
    } else if world.get::<Autolathe>(machine).is_some() {
        Some(Kind::Lathe)
    } else if world.get::<Apc>(machine).is_some() {
        Some(Kind::Apc)
    } else {
        None
    }
}

/// Whether this can be hacked here, from its sprite.
pub fn is_hackable(sprite: &str) -> bool {
    crate::vending::kind(sprite).is_some() || sprite == crate::autolathe::AUTOLATHE || sprite == "apc"
}

/// The tools that work a panel.
pub fn takes(tool: Option<&str>) -> bool {
    matches!(tool, Some("screwdriver" | "wirecutters" | "multitool"))
}

/// The machine's wires, fitted the first time anyone looks.
fn ensure(world: &mut World, machine: Entity) -> Option<Kind> {
    let kind = kind_of(world, machine)?;
    if world.get::<MachineWires>(machine).is_none() {
        world.init_resource::<HackLayouts>();
        let layout = match kind {
            Kind::Vending => world.resource::<HackLayouts>().vending.clone(),
            Kind::Apc => world.resource::<HackLayouts>().apc.clone(),
            Kind::Lathe => shuffled(&mut world.resource_mut::<GameRng>(), Kind::Lathe.wires()),
        };
        world.entity_mut(machine).insert(MachineWires::new(kind, layout));
    }
    Some(kind)
}

/// An emag swiped on a vending machine: its product lock shorts out for good.
pub fn emag_vendor(world: &mut World, machine: Entity) {
    if ensure(world, machine).is_some()
        && let Some(mut wires) = world.get_mut::<MachineWires>(machine)
    {
        wires.emagged = true;
    }
}

pub fn wires(world: &World, machine: Entity) -> Option<&MachineWires> {
    world.get::<MachineWires>(machine)
}

fn tell(world: &mut World, mob: Entity, id: &'static str) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(LocMsg::new(id));
    }
}

fn at(world: &World, e: Entity) -> Option<TilePos> {
    world.get::<GridPos>(e).map(|p| p.0)
}

fn powered(world: &World, machine: Entity) -> bool {
    world.get::<PowerUser>(machine).is_none_or(|p| p.powered)
        && world.get::<Apc>(machine).is_none_or(|a| !a.broken)
}

/// A shock from the machine, if it has power (Luna `shock`).
pub fn shock(world: &mut World, mob: Entity, machine: Entity, chance: f32) {
    if powered(world, machine) {
        world.resource_mut::<PendingShocks>().0.push((mob, chance));
    }
}

/// Open the wire panel for a mob.
pub fn show_wires(world: &mut World, mob: Entity, machine: Entity) {
    world
        .entity_mut(mob)
        .remove::<PanelViewer>()
        .insert(HackViewer(machine));
}

/// A screwdriver, wirecutters or a multitool on a machine (Luna `attackby`). The screwdriver
/// opens or shuts the panel; the others open the wire panel when it's open, and otherwise do
/// what an empty hand would.
pub fn use_on(world: &mut World, mob: Entity, machine: Entity, tool: &str) {
    let Some(kind) = ensure(world, machine) else {
        return;
    };
    let open = world.get::<MachineWires>(machine).is_some_and(|w| w.panel_open);
    if tool == "screwdriver" {
        if world.get::<Apc>(machine).is_some_and(|apc| apc.opened) {
            return tell(world, mob, "apc-close-first");
        }
        if world.get::<Apc>(machine).is_some_and(|apc| apc.emagged) {
            return tell(world, mob, "apc-interface-broken");
        }
        if let Some(mut wires) = world.get_mut::<MachineWires>(machine) {
            wires.panel_open = !wires.panel_open;
        }
        if open && world.get::<HackViewer>(mob).is_some_and(|v| v.0 == machine) {
            world.entity_mut(mob).remove::<HackViewer>();
        }
        if let Some(pos) = at(world, machine) {
            world
                .resource_mut::<crate::sound::SoundEvents>()
                .tool_used(Some(tool), pos);
        }
        let id = match (kind, open) {
            (Kind::Apc, false) => "apc-wires-exposed",
            (Kind::Apc, true) => "apc-wires-unexposed",
            (Kind::Lathe, false) => "lathe-hatch-opened",
            (Kind::Lathe, true) => "lathe-hatch-closed",
            (Kind::Vending, false) => "vending-panel-opened",
            (Kind::Vending, true) => "vending-panel-closed",
        };
        update_look(world, machine);
        return tell(world, mob, id);
    }
    if open {
        if kind == Kind::Lathe && world.get::<MachineWires>(machine).is_some_and(|w| w.shocked) {
            shock(world, mob, machine, WIRE_SHOCK);
        }
        return show_wires(world, mob, machine);
    }
    match kind {
        Kind::Vending => touch_vendor(world, mob, machine),
        Kind::Lathe => crate::autolathe::click(world, mob, machine),
        Kind::Apc => tell(world, mob, "apc-hand-full"),
    }
}

/// A vending machine clicked by hand (Luna `attack_hand`): an electrified one shocks, then its
/// window opens if it has power.
pub fn touch_vendor(world: &mut World, mob: Entity, machine: Entity) {
    let near = match (at(world, mob), at(world, machine)) {
        (Some(a), Some(b)) => in_reach(a, b),
        _ => false,
    };
    if !near || !powered(world, machine) {
        return;
    }
    if world
        .get::<MachineWires>(machine)
        .is_some_and(MachineWires::is_electrified)
    {
        shock(world, mob, machine, 100.0);
    }
    world
        .entity_mut(mob)
        .remove::<HackViewer>()
        .insert(PanelViewer(machine));
}

/// The autolathe's sprite with its hatch open (Luna `autolathe_t`).
fn update_look(world: &mut World, machine: Entity) {
    let Some(w) = world.get::<MachineWires>(machine) else {
        return;
    };
    if w.kind != Kind::Lathe {
        return;
    }
    let busy = world
        .get::<Autolathe>(machine)
        .is_some_and(|l| l.making.is_some());
    let state = if busy {
        "autolathe_n"
    } else if w.panel_open {
        "autolathe_t"
    } else {
        crate::autolathe::AUTOLATHE
    };
    if let Some(mut look) = world.get_mut::<Appearance>(machine)
        && look.state != state
    {
        look.state = state.to_owned();
    }
}

/// Luna's `cut` and `mend`. Returns the chance of a shock.
fn cut_or_mend(world: &mut World, machine: Entity, colour: usize) -> f32 {
    let Some(mut w) = world.get_mut::<MachineWires>(machine) else {
        return 0.0;
    };
    let Some(&wire) = w.layout.get(colour) else {
        return 0.0;
    };
    let mending = w.cut[colour];
    w.cut[colour] = !mending;
    let mut chance = 0.0;
    match wire {
        Wire::Contraband => {
            if !mending {
                w.contraband = false;
            }
        }
        // Luna `vending_tg` and the wire datum: a cut ID wire stops the scanner.
        Wire::ScanId => w.scan_id = mending,
        Wire::VendShock => {
            w.electrified = if mending {
                Electrified::Off
            } else {
                Electrified::Indefinitely
            };
        }
        Wire::Throw => w.throwing = !mending,
        // The autolathe's wires flip their state either way (Luna `act == "wire"`).
        Wire::Hack => w.hacked = !w.hacked,
        Wire::Disable => {
            w.disabled = !w.disabled;
            chance = WIRE_SHOCK;
        }
        Wire::LatheShock => {
            w.shocked = !w.shocked;
            chance = WIRE_SHOCK;
        }
        Wire::Dud | Wire::ApcIdScan => {}
        Wire::ApcAi => w.ai_disabled = !mending,
        Wire::ApcMain1 | Wire::ApcMain2 => {
            let whole = !w.is_cut(Wire::ApcMain1) && !w.is_cut(Wire::ApcMain2);
            if !mending || whole {
                chance = WIRE_SHOCK;
                if let Some(mut apc) = world.get_mut::<Apc>(machine) {
                    apc.shorted = !mending;
                }
            }
        }
    }
    chance
}

/// Luna's `pulse`. Returns the chance of a shock.
fn pulse(world: &mut World, machine: Entity, colour: usize, now: f32) -> f32 {
    let Some(mut w) = world.get_mut::<MachineWires>(machine) else {
        return 0.0;
    };
    let Some(&wire) = w.layout.get(colour) else {
        return 0.0;
    };
    let mut chance = 0.0;
    match wire {
        Wire::Contraband => w.contraband = !w.contraband,
        Wire::ScanId => w.scan_id = !w.scan_id,
        Wire::VendShock => {
            if w.electrified == Electrified::Off {
                w.electrified = Electrified::Until(now + VEND_SHOCK_DS);
            }
        }
        Wire::Throw => w.throwing = !w.throwing,
        Wire::Hack | Wire::Disable | Wire::LatheShock => {
            match wire {
                Wire::Hack => w.hacked = !w.hacked,
                Wire::Disable => w.disabled = !w.disabled,
                _ => w.shocked = !w.shocked,
            }
            if wire != Wire::Hack {
                chance = WIRE_SHOCK;
            }
            w.reverts.push((now + LATHE_PULSE_DS, wire));
        }
        Wire::Dud => {}
        Wire::ApcIdScan => {
            w.reverts.push((now + APC_UNLOCK_DS, wire));
            if let Some(mut apc) = world.get_mut::<Apc>(machine) {
                apc.locked = false;
            }
        }
        Wire::ApcMain1 | Wire::ApcMain2 => {
            w.reverts.push((now + APC_SHORT_DS, wire));
            if let Some(mut apc) = world.get_mut::<Apc>(machine) {
                apc.shorted = true;
            }
        }
        Wire::ApcAi => {
            if !w.ai_disabled {
                w.ai_disabled = true;
                w.reverts.push((now + APC_AI_DS, wire));
            }
        }
    }
    chance
}

/// A wire panel button on a machine (Luna `Topic`): in reach, the panel open, and the right
/// tool in hand.
pub fn wire_op(world: &mut World, mob: Entity, op: WireOp) {
    let (machine, colour, pulsing) = match op {
        WireOp::CutOrMend { door, colour } => (door, colour, false),
        WireOp::Pulse { door, colour } => (door, colour, true),
        WireOp::Close => {
            world.entity_mut(mob).remove::<HackViewer>();
            return;
        }
    };
    let Some(w) = world.get::<MachineWires>(machine) else {
        return;
    };
    let (open, cut) = (w.panel_open, w.cut.get(colour).copied());
    let Some(cut) = cut else { return };
    let near = match (at(world, mob), at(world, machine)) {
        (Some(a), Some(b)) => in_reach(a, b),
        _ => false,
    };
    if !open || !near {
        return;
    }
    let tool = world
        .get::<Hands>(mob)
        .and_then(Hands::active_item)
        .and_then(|e| world.get::<Appearance>(e))
        .map(|a| a.sprite.clone());
    let now = world.resource::<WorldTime>().deciseconds();
    let chance = if pulsing {
        if tool.as_deref() != Some("multitool") {
            return tell(world, mob, "wire-need-multitool");
        }
        if cut {
            return tell(world, mob, "wire-pulse-cut");
        }
        pulse(world, machine, colour, now)
    } else {
        if tool.as_deref() != Some("wirecutters") {
            return tell(world, mob, "wire-need-cutters");
        }
        if let Some(pos) = at(world, machine) {
            world
                .resource_mut::<crate::sound::SoundEvents>()
                .tool_used(tool.as_deref(), pos);
        }
        cut_or_mend(world, machine, colour)
    };
    if chance > 0.0 {
        shock(world, mob, machine, chance);
    }
}

/// Timed pulses running out, electrified vending machines going quiet, and broken ones
/// throwing their stock at people (Luna `process`).
pub fn run_hacking(world: &mut World) {
    let now = world.resource::<WorldTime>().deciseconds();
    let mut q = world.query::<(Entity, &MachineWires)>();
    let machines: Vec<Entity> = q.iter(world).map(|(e, _)| e).collect();
    for machine in machines {
        let Some(mut w) = world.get_mut::<MachineWires>(machine) else {
            continue;
        };
        if let Electrified::Until(t) = w.electrified
            && now >= t
        {
            w.electrified = Electrified::Off;
        }
        let due: Vec<Wire> = w
            .reverts
            .iter()
            .filter(|(t, _)| now >= *t)
            .map(|(_, x)| *x)
            .collect();
        w.reverts.retain(|(t, _)| now < *t);
        for wire in due {
            undo_pulse(world, machine, wire);
        }
        update_look(world, machine);
        let Some(w) = world.get::<MachineWires>(machine) else {
            continue;
        };
        if w.kind == Kind::Vending && w.throwing && now >= w.next_throw_ds && powered(world, machine) {
            if let Some(mut w) = world.get_mut::<MachineWires>(machine) {
                w.next_throw_ds = now + THROW_EVERY_DS;
            }
            if world.resource_mut::<GameRng>().prob(THROW_CHANCE) {
                throw_item(world, machine);
            }
        }
    }
}

fn undo_pulse(world: &mut World, machine: Entity, wire: Wire) {
    let Some(mut w) = world.get_mut::<MachineWires>(machine) else {
        return;
    };
    match wire {
        Wire::Hack => w.hacked = !w.hacked,
        Wire::Disable => w.disabled = !w.disabled,
        Wire::LatheShock => w.shocked = !w.shocked,
        Wire::ApcAi => {
            if !w.is_cut(Wire::ApcAi) {
                w.ai_disabled = false;
            }
        }
        Wire::ApcMain1 | Wire::ApcMain2 => {
            // Only while both main wires are whole: a cut one keeps it shorted.
            let whole = !w.is_cut(Wire::ApcMain1) && !w.is_cut(Wire::ApcMain2);
            if whole && let Some(mut apc) = world.get_mut::<Apc>(machine) {
                apc.shorted = false;
            }
        }
        Wire::ApcIdScan => {
            if let Some(mut apc) = world.get_mut::<Apc>(machine) {
                apc.locked = true;
            }
        }
        _ => {}
    }
}

/// Luna `throw_item`: the first product in stock flies at someone living nearby, give or take
/// a tile.
pub fn throw_item(world: &mut World, machine: Entity) -> bool {
    let Some(from) = at(world, machine) else {
        return false;
    };
    let mut mobs = world.query::<(Entity, &GridPos, &Vitals)>();
    let target = mobs
        .iter(world)
        .filter(|(_, p, v)| {
            v.stat != MobStat::Dead
                && p.0.z == from.z
                && (p.0.x - from.x).abs() <= THROW_RANGE
                && (p.0.y - from.y).abs() <= THROW_RANGE
        })
        .map(|(e, p, _)| (e, p.0))
        .next();
    let Some((target, mut to)) = target else {
        return false;
    };
    let item = {
        let Some(mut vending) = world.get_mut::<Vending>(machine) else {
            return false;
        };
        let Some(line) = vending.stock.iter().position(|n| *n > 0) else {
            return false;
        };
        vending.stock[line] -= 1;
        crate::vending::product(vending.kind, line)
    };
    let Some(item) = item else { return false };
    {
        let mut rng = world.resource_mut::<GameRng>();
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            if rng.prob(15.0) {
                to = TilePos::new(to.x + dx, to.y + dy, to.z);
            }
        }
    }
    let Ok(thing) = crate::objects::spawn_object(world, &ObjectSpec::new(item), from) else {
        return false;
    };
    let now = world.resource::<WorldTime>().deciseconds();
    let path: Vec<TilePos> = crate::speech::line(from, to).skip(1).collect();
    world.entity_mut(thing).insert(crate::grenades::Flying {
        path,
        next_ds: now + crate::grenades::STEP_DS,
    });
    let machine_id = world
        .get::<Appearance>(machine)
        .map(|a| a.sprite.clone())
        .unwrap_or_default();
    let name = world
        .get::<DisplayName>(target)
        .map(|n| n.0.clone())
        .unwrap_or_default();
    let msg = LocMsg::new("vending-launches")
        .arg("machine", machine_id)
        .arg("item", item)
        .arg("name", name);
    world.resource_mut::<VisibleEvents>().0.push((from, msg, None));
    true
}

/// Keep each open machine wire panel in step, and close it once it can't be used.
pub fn update_hack_views(world: &mut World) {
    let mut q = world.query_filtered::<Entity, With<HackViewer>>();
    let viewers: Vec<Entity> = q.iter(world).collect();
    for mob in viewers {
        let view = panel_view(world, mob);
        if view.is_none() {
            world.entity_mut(mob).remove::<HackViewer>();
        }
        if let Some(mut out) = world.get_mut::<crate::wires::WirePanelOut>(mob)
            && out.0 != view
        {
            out.0 = view;
        }
    }
}

/// The wire panel a mob has open on a machine, if they can still use it.
pub fn panel_view(world: &World, mob: Entity) -> Option<WirePanelView> {
    let machine = world.get::<HackViewer>(mob)?.0;
    let w = world.get::<MachineWires>(machine)?;
    let near = in_reach(at(world, mob)?, at(world, machine)?);
    let awake = world
        .get::<Vitals>(mob)
        .is_none_or(|v| v.stat == MobStat::Conscious);
    if !w.panel_open || !near || !awake {
        return None;
    }
    Some(WirePanelView {
        target: *world.get::<NetId>(machine)?,
        wires: w
            .kind
            .colours()
            .iter()
            .zip(&w.cut)
            .map(|(c, cut)| ((*c).to_owned(), *cut))
            .collect(),
        lights: w.lights(world.get::<Apc>(machine)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_are_shuffles() {
        let mut world = World::new();
        let l = HackLayouts::from_world(&mut world);
        for wire in Kind::Vending.wires() {
            assert_eq!(l.vending.iter().filter(|w| **w == wire).count(), 1);
        }
        for wire in Kind::Apc.wires() {
            assert_eq!(l.apc.iter().filter(|w| **w == wire).count(), 1);
        }
        assert_eq!(Kind::Lathe.wires().len(), LATHE_COLOURS.len());
    }
}
