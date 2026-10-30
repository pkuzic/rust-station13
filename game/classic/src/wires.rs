//! Airlock maintenance panels and wires. Spec: `docs/reference/wires.md`.
//!
//! A screwdriver opens the panel. With the panel open, using the door shows the wire
//! panel: eleven coloured wires, each cut or mended with wirecutters or pulsed with a
//! multitool. Which colour does what is shuffled once per round, as in Luna.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, DisplayName, GridPos, MobStat, NetId, WirePanelView};

use crate::actions::{Outbox, in_reach};
use crate::combat::VisibleEvents;
use crate::doors::Airlock;
use crate::equipment::{Equipment, Wearable};
use crate::hands::Hands;
use crate::health::{Body, Vitals};
use crate::rng::GameRng;
use crate::time::WorldTime;

/// Wire colours, in the order the panel lists them (Luna's `attack_hand`). These are also
/// message selectors: `wire-colour` in the locales.
pub const WIRE_COLOURS: [&str; 11] = [
    "orange", "dark-red", "white", "yellow", "red", "blue", "green", "grey", "black", "pink", "brown",
];

/// What a wire does (Luna `AIRLOCK_WIRE_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireFn {
    IdScan,
    MainPower1,
    MainPower2,
    Bolts,
    BackupPower1,
    BackupPower2,
    OpenDoor,
    AiControl,
    Electrify,
    Crush,
    Light,
}

impl WireFn {
    const ALL: [WireFn; 11] = [
        WireFn::IdScan,
        WireFn::MainPower1,
        WireFn::MainPower2,
        WireFn::Bolts,
        WireFn::BackupPower1,
        WireFn::BackupPower2,
        WireFn::OpenDoor,
        WireFn::AiControl,
        WireFn::Electrify,
        WireFn::Crush,
        WireFn::Light,
    ];
}

/// Main power comes back this long after a pulse or a cut is mended (Luna: 60 s).
pub const MAIN_POWER_LOST_DS: f32 = 600.0;
/// Backup power holds the door this long after main power is lost (Luna: 10 s).
pub const BACKUP_DELAY_DS: f32 = 100.0;
/// A pulsed backup wire trips its breaker for this long (Luna: 60 s).
pub const BACKUP_POWER_LOST_DS: f32 = 600.0;
/// A pulsed electrify wire charges the door for this long (Luna: 30 s).
pub const ELECTRIFY_PULSE_DS: f32 = 300.0;
/// Chance of a shock when cutting or mending a power wire (Luna: 50%).
pub const POWER_WIRE_SHOCK: f32 = 50.0;
/// Chance of a shock touching an electrified door by hand or walking into it (Luna: 100%).
pub const HAND_SHOCK: f32 = 100.0;
/// Chance of a shock using a tool on an electrified door (Luna: 75%).
pub const TOOL_SHOCK: f32 = 75.0;

/// Colour index → function, the same for every airlock this round.
#[derive(Resource, Debug, Clone)]
pub struct WireLayout(pub [WireFn; 11]);

impl FromWorld for WireLayout {
    fn from_world(world: &mut World) -> Self {
        world.init_resource::<GameRng>();
        let mut rng = world.resource_mut::<GameRng>();
        // Fisher–Yates over the functions.
        let mut layout = WireFn::ALL;
        for i in (1..layout.len()).rev() {
            let j = rng.range(0, i as i32) as usize;
            layout.swap(i, j);
        }
        Self(layout)
    }
}

impl WireLayout {
    pub fn colour_of(&self, function: WireFn) -> usize {
        self.0.iter().position(|f| *f == function).unwrap_or(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Electrified {
    Off,
    /// Until this world time (ds).
    Until(f32),
    /// Until the electrify wire is mended.
    Indefinitely,
}

/// The wiring inside one airlock.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Wiring {
    /// By colour index.
    pub cut: [bool; 11],
    /// Main power comes back when this reaches 0 and both main wires are whole.
    pub main_lost_ds: f32,
    /// The same for backup power.
    pub backup_lost_ds: f32,
    pub electrified: Electrified,
    /// Luna's `forcecrush`: shown on the panel; crushing is not modelled yet.
    pub crush: bool,
    /// Luna's `safetylight`: a bolted door shows the bolted sprite.
    pub bolt_light: bool,
}

impl Default for Wiring {
    fn default() -> Self {
        Self {
            cut: [false; 11],
            main_lost_ds: 0.0,
            backup_lost_ds: 0.0,
            electrified: Electrified::Off,
            crush: false,
            bolt_light: true,
        }
    }
}

impl Wiring {
    pub fn is_cut(&self, layout: &WireLayout, function: WireFn) -> bool {
        self.cut[layout.colour_of(function)]
    }

    fn main_whole(&self, layout: &WireLayout) -> bool {
        !self.is_cut(layout, WireFn::MainPower1) && !self.is_cut(layout, WireFn::MainPower2)
    }

    fn backup_whole(&self, layout: &WireLayout) -> bool {
        !self.is_cut(layout, WireFn::BackupPower1) && !self.is_cut(layout, WireFn::BackupPower2)
    }

    /// Luna `arePowerSystemsOn`, less the area power: main or backup is up.
    pub fn systems_on(&self) -> bool {
        self.main_lost_ds <= 0.0 || self.backup_lost_ds <= 0.0
    }

    pub fn is_electrified(&self) -> bool {
        self.electrified != Electrified::Off
    }

    fn lose_main(&mut self) {
        if self.main_lost_ds <= 0.0 {
            self.main_lost_ds = MAIN_POWER_LOST_DS;
            self.backup_lost_ds = self.backup_lost_ds.max(BACKUP_DELAY_DS);
        }
    }

    fn lose_backup(&mut self) {
        self.backup_lost_ds = self.backup_lost_ds.max(BACKUP_POWER_LOST_DS);
    }

    /// Luna `pulse`. Luna's `switch` only matches the first of each pair of power wires
    /// (`if (A || B)` is `if (A)` in DM); both wires of a pair work the same here.
    pub fn pulse(&mut self, function: WireFn, door: &mut Airlock, now: f32) {
        match function {
            WireFn::IdScan => {
                if door.has_power() {
                    door.deny_until = Some(now + crate::doors::DENY_DS);
                }
            }
            WireFn::MainPower1 | WireFn::MainPower2 => self.lose_main(),
            WireFn::BackupPower1 | WireFn::BackupPower2 => self.lose_backup(),
            WireFn::Bolts => {
                if !door.bolted {
                    door.bolted = true;
                } else if door.has_power() {
                    door.bolted = false;
                }
            }
            WireFn::OpenDoor => {
                if !door.id_scan || door.req_access.is_empty() {
                    door.toggle(now);
                }
            }
            WireFn::Electrify => {
                if self.electrified == Electrified::Off {
                    self.electrified = Electrified::Until(now + ELECTRIFY_PULSE_DS);
                }
            }
            WireFn::Crush => self.crush = !self.crush,
            WireFn::Light => self.bolt_light = !self.bolt_light,
            // There is no AI yet: the pulse only blinks a light Luna shows for a second.
            WireFn::AiControl => {}
        }
    }

    /// Luna `cut`. Returns the chance of a shock.
    pub fn cut(&mut self, layout: &WireLayout, colour: usize, door: &mut Airlock) -> f32 {
        self.cut[colour] = true;
        match layout.0[colour] {
            WireFn::MainPower1 | WireFn::MainPower2 => {
                self.lose_main();
                POWER_WIRE_SHOCK
            }
            WireFn::BackupPower1 | WireFn::BackupPower2 => {
                self.lose_backup();
                POWER_WIRE_SHOCK
            }
            WireFn::Bolts => {
                door.bolted = true;
                0.0
            }
            WireFn::Electrify => {
                self.electrified = Electrified::Indefinitely;
                0.0
            }
            _ => 0.0,
        }
    }

    /// Luna `mend`. Returns the chance of a shock.
    pub fn mend(&mut self, layout: &WireLayout, colour: usize) -> f32 {
        self.cut[colour] = false;
        match layout.0[colour] {
            WireFn::MainPower1 | WireFn::MainPower2 if self.main_whole(layout) => {
                self.main_lost_ds = 0.0;
                POWER_WIRE_SHOCK
            }
            WireFn::BackupPower1 | WireFn::BackupPower2 if self.backup_whole(layout) => {
                self.backup_lost_ds = 0.0;
                POWER_WIRE_SHOCK
            }
            WireFn::Electrify => {
                if self.electrified == Electrified::Indefinitely {
                    self.electrified = Electrified::Off;
                }
                0.0
            }
            _ => 0.0,
        }
    }

    /// The panel's indicator lights, as message ids (Luna `attack_hand`).
    pub fn lights(&self, layout: &WireLayout, door: &Airlock) -> Vec<String> {
        let ai_allowed = !self.is_cut(layout, WireFn::AiControl);
        [
            if door.bolted {
                "panel-bolts-down"
            } else {
                "panel-bolts-up"
            },
            if door.has_power() {
                "panel-test-light-on"
            } else {
                "panel-test-light-off"
            },
            if ai_allowed {
                "panel-ai-light-on"
            } else {
                "panel-ai-light-off"
            },
            if self.is_electrified() {
                "panel-safety-flashing"
            } else {
                "panel-safety-on"
            },
            if self.crush {
                "panel-hydraulics-red"
            } else {
                "panel-hydraulics-green"
            },
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    }
}

/// Count down the power breakers, expire electrification, and tell the door whether
/// it has power and whether its ID scanner works. Runs before `update_doors`.
pub fn update_wiring(
    time: Res<WorldTime>,
    layout: Res<WireLayout>,
    mut doors: Query<(&mut Wiring, &mut Airlock)>,
) {
    let now = time.deciseconds();
    let dt = crate::time::TICK_DS;
    for (mut wiring, mut door) in &mut doors {
        // Luna counts down only while the pair of wires is whole.
        if wiring.main_lost_ds > 0.0 && wiring.main_whole(&layout) {
            wiring.main_lost_ds = (wiring.main_lost_ds - dt).max(0.0);
        }
        if wiring.backup_lost_ds > 0.0 && wiring.backup_whole(&layout) {
            wiring.backup_lost_ds = (wiring.backup_lost_ds - dt).max(0.0);
        }
        if let Electrified::Until(at) = wiring.electrified
            && now >= at
        {
            wiring.electrified = Electrified::Off;
        }
        let systems_on = wiring.systems_on();
        let id_scan = !wiring.is_cut(&layout, WireFn::IdScan);
        if door.systems_on != systems_on {
            door.systems_on = systems_on;
        }
        if door.id_scan != id_scan {
            door.id_scan = id_scan;
        }
    }
}

/// A shock waiting to be rolled: who, and the chance in percent.
#[derive(Resource, Debug, Default)]
pub struct PendingShocks(pub Vec<(Entity, f32)>);

/// Luna `Electrocute`: burn damage of the lower of two rolls of 20–80, times the gloves'
/// conductivity. Insulated gloves stop it.
pub fn shock_damage(rng: &mut GameRng, siemens: f32) -> f32 {
    let roll = rng.range(20, 80).min(rng.range(20, 80));
    roll as f32 * siemens
}

pub fn resolve_shocks(
    mut shocks: ResMut<PendingShocks>,
    mut rng: ResMut<GameRng>,
    mut visible: ResMut<VisibleEvents>,
    mut victims: Query<(
        &GridPos,
        &mut Body,
        &mut Outbox,
        Option<&Equipment>,
        Option<&DisplayName>,
    )>,
    wearables: Query<&Wearable>,
) {
    for (victim, chance) in shocks.0.drain(..) {
        let Ok((pos, mut body, mut outbox, equipment, name)) = victims.get_mut(victim) else {
            continue;
        };
        if !rng.prob(chance) {
            continue;
        }
        let siemens = equipment
            .and_then(|e| e.get(rs13_world::Slot::Gloves))
            .and_then(|g| wearables.get(g).ok())
            .map_or(1.0, |w| w.siemens);
        if siemens <= 0.0 {
            continue;
        }
        let burn = shock_damage(&mut rng, siemens);
        body.take_damage_spread(0.0, burn);
        outbox.0.push(LocMsg::new("shocked"));
        let name = name.map(|n| n.0.clone()).unwrap_or_default();
        visible
            .0
            .push((pos.0, LocMsg::new("shocked-seen").arg("name", name), Some(victim)));
    }
}

/// The machine window a mob has open: a door's wire panel or an air alarm (Luna
/// `usr.machine`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelViewer(pub Entity);

/// What the mob's open wire panel shows, for the server to send when it changes.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct WirePanelOut(pub Option<WirePanelView>);

/// A request from the wire panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireOp {
    /// Wirecutters on a wire: cut it, or mend it if cut.
    CutOrMend {
        door: Entity,
        colour: usize,
    },
    /// Multitool on a wire.
    Pulse {
        door: Entity,
        colour: usize,
    },
    Close,
}

#[derive(Component, Debug, Default)]
pub struct PendingWires(pub Vec<WireOp>);

/// Apply wire panel requests. Luna `Topic`: the panel must be open and in reach, and the
/// right tool in the active hand.
pub fn process_wire_ops(
    time: Res<WorldTime>,
    layout: Res<WireLayout>,
    mut shocks: ResMut<PendingShocks>,
    mut mobs: Query<(
        Entity,
        &GridPos,
        &Hands,
        &mut PendingWires,
        &mut Outbox,
        Option<&Vitals>,
    )>,
    mut doors: Query<(&GridPos, &mut Airlock, &mut Wiring)>,
    appearances: Query<&Appearance>,
    mut commands: Commands,
) {
    let now = time.deciseconds();
    for (mob, pos, hands, mut pending, mut outbox, vitals) in &mut mobs {
        if vitals.is_some_and(|v| v.stat != MobStat::Conscious) {
            pending.0.clear();
            continue;
        }
        let tool = hands
            .active_item()
            .and_then(|e| appearances.get(e).ok())
            .map(|a| a.sprite.as_str());
        for op in pending.0.drain(..) {
            let (door, colour, pulse) = match op {
                WireOp::Close => {
                    commands.entity(mob).remove::<PanelViewer>();
                    continue;
                }
                WireOp::CutOrMend { door, colour } => (door, colour, false),
                WireOp::Pulse { door, colour } => (door, colour, true),
            };
            let Ok((door_pos, mut airlock, mut wiring)) = doors.get_mut(door) else {
                continue;
            };
            if !airlock.panel_open || !in_reach(pos.0, door_pos.0) || colour >= WIRE_COLOURS.len() {
                continue;
            }
            let chance = if pulse {
                if tool != Some("multitool") {
                    outbox.0.push(LocMsg::new("wire-need-multitool"));
                    continue;
                }
                if wiring.cut[colour] {
                    outbox.0.push(LocMsg::new("wire-pulse-cut"));
                    continue;
                }
                wiring.pulse(layout.0[colour], &mut airlock, now);
                0.0
            } else {
                if tool != Some("wirecutters") {
                    outbox.0.push(LocMsg::new("wire-need-cutters"));
                    continue;
                }
                if wiring.cut[colour] {
                    wiring.mend(&layout, colour)
                } else {
                    wiring.cut(&layout, colour, &mut airlock)
                }
            };
            // A dead door cannot shock anyone.
            if chance > 0.0 && airlock.powered {
                shocks.0.push((mob, chance));
            }
        }
    }
}

/// Build each open wire panel's view; close it once its viewer can no longer use it.
pub fn update_panel_views(
    layout: Res<WireLayout>,
    mut viewers: Query<(
        Entity,
        &GridPos,
        Option<&PanelViewer>,
        &mut WirePanelOut,
        Option<&Vitals>,
    )>,
    doors: Query<(&GridPos, &Airlock, &Wiring, &NetId)>,
    mut commands: Commands,
) {
    for (mob, pos, viewer, mut out, vitals) in &mut viewers {
        let view = viewer.and_then(|v| {
            let (door_pos, airlock, wiring, id) = doors.get(v.0).ok()?;
            let usable = airlock.panel_open
                && in_reach(pos.0, door_pos.0)
                && vitals.is_none_or(|v| v.stat == MobStat::Conscious);
            usable.then(|| WirePanelView {
                target: *id,
                wires: WIRE_COLOURS
                    .iter()
                    .zip(wiring.cut)
                    .map(|(colour, cut)| ((*colour).to_owned(), cut))
                    .collect(),
                lights: wiring.lights(&layout, airlock),
            })
        });
        // Close the window when it can no longer be used; leave other panels alone.
        if view.is_none() && viewer.is_some_and(|v| doors.contains(v.0)) {
            commands.entity(mob).remove::<PanelViewer>();
        }
        if out.0 != view {
            out.0 = view;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> WireLayout {
        WireLayout(WireFn::ALL)
    }

    #[test]
    fn cutting_main_power_falls_back_to_backup_for_ten_seconds() {
        let layout = layout();
        let mut wiring = Wiring::default();
        let mut door = Airlock::default();
        assert!(wiring.systems_on());
        let main = layout.colour_of(WireFn::MainPower1);
        assert_eq!(wiring.cut(&layout, main, &mut door), POWER_WIRE_SHOCK);
        // Main is gone and backup is tripped for 10 s.
        assert!(!wiring.systems_on());
        assert_eq!(wiring.backup_lost_ds, BACKUP_DELAY_DS);
        wiring.backup_lost_ds = 0.0;
        assert!(wiring.systems_on(), "backup takes over");
    }

    #[test]
    fn mending_needs_both_wires_of_a_pair() {
        let layout = layout();
        let mut wiring = Wiring::default();
        let mut door = Airlock::default();
        let (a, b) = (
            layout.colour_of(WireFn::MainPower1),
            layout.colour_of(WireFn::MainPower2),
        );
        wiring.cut(&layout, a, &mut door);
        wiring.cut(&layout, b, &mut door);
        assert_eq!(wiring.mend(&layout, a), 0.0);
        assert!(wiring.main_lost_ds > 0.0);
        assert_eq!(wiring.mend(&layout, b), POWER_WIRE_SHOCK);
        assert_eq!(wiring.main_lost_ds, 0.0);
    }

    #[test]
    fn bolts_drop_on_cut_and_rise_on_pulse_only_with_power() {
        let layout = layout();
        let mut wiring = Wiring::default();
        let mut door = Airlock::default();
        wiring.pulse(WireFn::Bolts, &mut door, 0.0);
        assert!(door.bolted);
        door.powered = false;
        wiring.pulse(WireFn::Bolts, &mut door, 0.0);
        assert!(door.bolted, "no power, no raising");
        door.powered = true;
        wiring.pulse(WireFn::Bolts, &mut door, 0.0);
        assert!(!door.bolted);
        wiring.cut(&layout, layout.colour_of(WireFn::Bolts), &mut door);
        assert!(door.bolted);
    }

    #[test]
    fn the_layout_is_a_shuffle() {
        let mut world = World::new();
        let layout = WireLayout::from_world(&mut world);
        for function in WireFn::ALL {
            assert_eq!(layout.0.iter().filter(|f| **f == function).count(), 1);
        }
    }

    #[test]
    fn shock_rolls_stay_in_luna_range() {
        let mut rng = GameRng::seeded(7);
        for _ in 0..200 {
            let d = shock_damage(&mut rng, 1.0);
            assert!((20.0..=80.0).contains(&d));
        }
        assert_eq!(shock_damage(&mut rng, 0.0), 0.0);
    }
}
