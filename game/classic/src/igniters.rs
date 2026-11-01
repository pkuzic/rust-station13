//! Igniters: a floor-mounted spark plug that, while on, keeps touching the air with a hot
//! spark; a wall-mounted sparker that throws one spark when told to; and the wall switch
//! that works every igniter and sparker with its id. Burn chambers and incinerators light
//! their gas with them. Spec: `docs/reference/igniters.md`.

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, GridPos, LightSource, NetId, ObjectSpec, TilePos};

use crate::actions::ClickOutcome;
use crate::atmos::StationAtmos;
use crate::machines::MACHINE_TICKS;
use crate::power::{Channel, PowerUser};
use crate::time::WorldTime;

/// The floor-mounted igniter prototype.
pub const IGNITER: &str = "igniter";
/// The wall-mounted sparker prototype.
pub const SPARKER: &str = "sparker";
/// The wall switch prototype.
pub const IGNITION_SWITCH: &str = "ignition-switch";
/// How hot an igniter's or sparker's spark is, K (Luna `hotspot_expose(6000, 500)`).
pub const SPARK_TEMPERATURE: f64 = 6000.0;
/// A pressed switch stays down this long before it can be pressed again, ds (Luna `sleep(50)`).
pub const SWITCH_COOLDOWN_DS: f32 = 50.0;
/// A sparker sparks at most this often, ds (Luna `last_spark + 50`).
pub const SPARKER_COOLDOWN_DS: f32 = 50.0;
/// How long a sparker shows its spark, ds.
pub const SPARK_SHOWN_DS: f32 = 5.0;
/// Sparks a sparker throws each time it sparks (`sparks.rs`).
pub const SPARKS_THROWN: u8 = 2;
/// The faint light of a powered, connected sparker (Luna `ul_SetLuminosity(2)`).
pub const SPARKER_LIGHT: [u8; 3] = [2, 2, 2];

/// A floor-mounted igniter, and the id its switches share.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct Igniter {
    pub id: String,
    pub on: bool,
}

/// A wall-mounted sparker, and the id its switches share.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Sparker {
    pub id: String,
    /// Its connection undone with a screwdriver (Luna `disable`).
    pub disabled: bool,
    /// When it last sparked, ds.
    pub last_spark: Option<f32>,
}

/// A wall-mounted ignition switch for the igniters and sparkers with its id.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct IgnitionSwitch {
    pub id: String,
    /// While pressed: when it comes back up, ds.
    pub pressed_until: Option<f32>,
}

/// Sparkers told to spark this tick, for `run_igniters` to touch the air with.
#[derive(Resource, Debug, Default)]
pub struct PendingSparks(pub Vec<TilePos>);

/// Luna's map sets `on`, `0` or `1`; it starts on if unset (Luna `on = 1`).
fn var_on(spec: &ObjectSpec) -> bool {
    spec.vars.get("on").is_none_or(|v| v.trim() != "0")
}

fn var_id(spec: &ObjectSpec) -> String {
    spec.vars.get("id").cloned().unwrap_or_default()
}

pub fn spawn_igniter(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    let igniter = Igniter {
        id: var_id(spec),
        on: var_on(spec),
    };
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(IGNITER, igniter_look(&igniter, true)),
            igniter,
            PowerUser::new(Channel::Equipment, 0.0),
        ))
        .id()
}

pub fn spawn_sparker(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(SPARKER, "migniter"),
            Sparker {
                id: var_id(spec),
                disabled: false,
                last_spark: None,
            },
            PowerUser::new(Channel::Equipment, 0.0),
            LightSource(SPARKER_LIGHT),
        ))
        .id()
}

pub fn spawn_ignition_switch(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(IGNITION_SWITCH, "launcherbtt"),
            IgnitionSwitch {
                id: var_id(spec),
                pressed_until: None,
            },
            PowerUser::new(Channel::Equipment, 0.0),
        ))
        .id()
}

/// `igniter1` while on and powered, else `igniter0` (Luna `power_change`).
fn igniter_look(igniter: &Igniter, powered: bool) -> &'static str {
    if igniter.on && powered {
        "igniter1"
    } else {
        "igniter0"
    }
}

/// A sparker's sprite state (Luna `power_change`, `attackby`, `ignite`): `migniter-d`
/// disconnected, `migniter-p` without power, `migniter-spark` just after a spark.
fn sparker_look(sparker: &Sparker, powered: bool, now: f32) -> &'static str {
    if sparker.disabled {
        "migniter-d"
    } else if !powered {
        "migniter-p"
    } else if sparker.last_spark.is_some_and(|t| now < t + SPARK_SHOWN_DS) {
        "migniter-spark"
    } else {
        "migniter"
    }
}

/// What working an igniter, sparker or switch did: for the user, and for everyone who
/// sees it (with `$name` to fill in).
#[derive(Debug, Default)]
pub struct IgniterOutcome {
    pub to_mob: ClickOutcome,
    pub seen: Option<LocMsg>,
}

/// Everything clicking igniters, sparkers and their switches needs.
#[derive(SystemParam)]
pub struct IgniterWork<'w, 's> {
    igniters: Query<'w, 's, (&'static mut Igniter, &'static PowerUser)>,
    sparkers: Query<'w, 's, (&'static mut Sparker, &'static GridPos, &'static PowerUser), Without<Igniter>>,
    switches: Query<
        'w,
        's,
        (&'static mut IgnitionSwitch, &'static PowerUser),
        (Without<Igniter>, Without<Sparker>),
    >,
    sparks: ResMut<'w, PendingSparks>,
    showers: ResMut<'w, crate::sparks::SparkShowers>,
}

impl IgniterWork<'_, '_> {
    pub fn contains(&self, target: Entity) -> bool {
        self.igniters.contains(target) || self.sparkers.contains(target) || self.switches.contains(target)
    }

    /// Luna `igniter/attack_hand` flips an igniter, powered or not. Luna
    /// `sparker/attackby`: a screwdriver undoes or redoes a sparker's connection, which
    /// everyone around sees; anything else does nothing. Luna
    /// `ignition_switch/attack_hand`: a powered switch that isn't already down goes down
    /// for five seconds, flips every igniter with its id and tells every sparker with its
    /// id to spark. Anything in the hand works the switch as a hand. The caller checks
    /// reach.
    pub fn use_on(&mut self, target: Entity, held: Option<&str>, now_ds: f32) -> Option<IgniterOutcome> {
        if let Ok((mut igniter, _)) = self.igniters.get_mut(target) {
            igniter.on = !igniter.on;
            return Some(IgniterOutcome::default());
        }
        if let Ok((mut sparker, ..)) = self.sparkers.get_mut(target) {
            if held != Some("screwdriver") {
                return Some(IgniterOutcome::default());
            }
            sparker.disabled = !sparker.disabled;
            let (to_mob, seen) = if sparker.disabled {
                ("sparker-disabled-self", "sparker-disabled")
            } else {
                ("sparker-reconnected-self", "sparker-reconnected")
            };
            return Some(IgniterOutcome {
                to_mob: ClickOutcome {
                    to_mob: Some(LocMsg::new(to_mob).arg("item", SPARKER)),
                    seen: None,
                },
                seen: Some(LocMsg::new(seen).arg("item", SPARKER)),
            });
        }
        let (mut switch, power) = self.switches.get_mut(target).ok()?;
        if !power.powered || switch.pressed_until.is_some() {
            return Some(IgniterOutcome::default());
        }
        switch.pressed_until = Some(now_ds + SWITCH_COOLDOWN_DS);
        let id = switch.id.clone();
        for (mut sparker, pos, power) in &mut self.sparkers {
            let ready = sparker
                .last_spark
                .is_none_or(|t| now_ds >= t + SPARKER_COOLDOWN_DS);
            if sparker.id == id && power.powered && !sparker.disabled && ready {
                sparker.last_spark = Some(now_ds);
                self.sparks.0.push(pos.0);
                // Luna `spark_spread.set_up(2, 1, src)`: two sparks, straight lines.
                self.showers.0.push((pos.0, SPARKS_THROWN));
            }
        }
        for (mut igniter, _) in &mut self.igniters {
            if igniter.id == id {
                igniter.on = !igniter.on;
            }
        }
        Some(IgniterOutcome::default())
    }
}

/// Every machine cycle a powered igniter that is on touches the air on its tile with its
/// spark (Luna `igniter/process`); sparkers told to spark do so at once (Luna `ignite`).
/// Pressed switches come back up when their time is over.
pub fn run_igniters(
    time: Res<WorldTime>,
    atmos: Option<ResMut<StationAtmos>>,
    mut sparks: ResMut<PendingSparks>,
    igniters: Query<(&Igniter, &GridPos, &PowerUser)>,
    mut switches: Query<&mut IgnitionSwitch>,
) {
    let now = time.deciseconds();
    for mut switch in &mut switches {
        if switch.pressed_until.is_some_and(|t| now >= t) {
            switch.pressed_until = None;
        }
    }
    let sparked = std::mem::take(&mut sparks.0);
    let Some(mut atmos) = atmos else { return };
    for at in sparked {
        atmos.0.expose(at, SPARK_TEMPERATURE);
    }
    if !time.tick.0.is_multiple_of(MACHINE_TICKS) {
        return;
    }
    for (igniter, pos, power) in &igniters {
        if igniter.on && power.powered {
            atmos.0.expose(pos.0, SPARK_TEMPERATURE);
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn update_igniter_looks(
    time: Res<WorldTime>,
    mut commands: Commands,
    mut igniters: Query<(&Igniter, &PowerUser, &mut Appearance), (Without<IgnitionSwitch>, Without<Sparker>)>,
    mut sparkers: Query<
        (Entity, &Sparker, &PowerUser, &mut Appearance, Has<LightSource>),
        (Without<Igniter>, Without<IgnitionSwitch>),
    >,
    mut switches: Query<(&IgnitionSwitch, &mut Appearance), (Without<Igniter>, Without<Sparker>)>,
) {
    let now = time.deciseconds();
    for (igniter, power, mut look) in &mut igniters {
        let state = igniter_look(igniter, power.powered);
        if look.state != state {
            look.state = state.to_owned();
        }
    }
    for (entity, sparker, power, mut look, lit) in &mut sparkers {
        let state = sparker_look(sparker, power.powered, now);
        if look.state != state {
            look.state = state.to_owned();
        }
        // A pilot light while powered and connected (Luna `power_change`).
        let glows = power.powered && !sparker.disabled;
        if glows && !lit {
            commands.entity(entity).insert(LightSource(SPARKER_LIGHT));
        } else if !glows && lit {
            commands.entity(entity).remove::<LightSource>();
        }
    }
    for (switch, mut look) in &mut switches {
        let state = if switch.pressed_until.is_some() {
            "launcheract"
        } else {
            "launcherbtt"
        };
        if look.state != state {
            look.state = state.to_owned();
        }
    }
}
