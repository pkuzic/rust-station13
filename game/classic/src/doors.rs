//! Airlocks. Spec: `docs/reference/airlock.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, Dense, Dir, GridPos, MapLayer, Opaque, TilePos};

use crate::access::{IdCard, allowed};
use crate::actions::Outbox;
use crate::hands::Item;
use crate::movement::Mob;
use crate::time::WorldTime;
use crate::wires::Wiring;

/// From the start of opening until passable.
pub const OPEN_DS: f32 = 6.0;
/// From the start of closing until mobs and items on the tile are pushed out.
pub const CLOSE_PUSH_DS: f32 = 4.0;
/// From the start of closing until fully closed.
pub const CLOSE_DS: f32 = 6.0;
/// From fully open until the single autoclose check.
pub const AUTOCLOSE_DS: f32 = 150.0;
/// Minimum time between two bumps by the same mob.
pub const BUMP_COOLDOWN_DS: f32 = 5.0;
/// How long the "deny" animation shows. Luna flicks `door_deny`; its length comes from
/// the sprite's frame delays, so this is an estimate until the sprites are imported.
pub const DENY_DS: f32 = 3.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DoorState {
    Closed,
    Opening {
        done_at: f32,
    },
    Open {
        autoclose_at: Option<f32>,
    },
    Closing {
        push_at: f32,
        done_at: f32,
        pushed: bool,
    },
}

impl DoorState {
    /// Sprite state name, as in BYOND's `icon_state`.
    pub fn name(self) -> &'static str {
        match self {
            DoorState::Closed => "closed",
            DoorState::Opening { .. } => "opening",
            DoorState::Open { .. } => "open",
            DoorState::Closing { .. } => "closing",
        }
    }
}

#[derive(Component, Debug, Clone, PartialEq)]
pub struct Airlock {
    pub state: DoorState,
    /// Bolted doors refuse to open or close.
    pub bolted: bool,
    /// Stands in for the powernet until it exists.
    pub powered: bool,
    /// Access ids, any one of which opens the door. Empty: anyone.
    pub req_access: Vec<String>,
    /// Until when the "deny" animation shows.
    pub deny_until: Option<f32>,
    /// Powered opens and closes since the power system last billed them.
    pub operations: u32,
    /// The maintenance panel is open (Luna `p_open`).
    pub panel_open: bool,
    /// Main or backup power is up inside the door (Luna `arePowerSystemsOn`, less the
    /// area power). Set by `wires::update_wiring`.
    pub systems_on: bool,
    /// The ID scanner wire is whole. Without it the door recognises no card.
    pub id_scan: bool,
    /// Next to an area whose air alarm seal is on. Set by `alarms::apply_seals`.
    pub in_seal: bool,
    /// The seal still has to close and bolt this door.
    pub seal_pending: bool,
    /// The bolts were dropped by the seal (Luna `air_locked`), so lifting the seal
    /// raises them again.
    pub air_locked: bool,
    /// Welded shut (Luna `welded`): it opens for nothing until welded open again.
    pub welded: bool,
}

impl Default for Airlock {
    fn default() -> Self {
        Self {
            state: DoorState::Closed,
            bolted: false,
            powered: true,
            req_access: Vec::new(),
            deny_until: None,
            operations: 0,
            panel_open: false,
            systems_on: true,
            id_scan: true,
            in_seal: false,
            seal_pending: false,
            air_locked: false,
            welded: false,
        }
    }
}

impl Airlock {
    /// Area power and the door's own power systems.
    pub fn has_power(&self) -> bool {
        self.powered && self.systems_on
    }

    /// Luna's door `attackby`, also reached by hand and by bumping: ignored while
    /// operating; with access, toggle; without, a closed door shows "deny".
    pub fn use_by(&mut self, now: f32, card: Option<&IdCard>) {
        if matches!(self.state, DoorState::Opening { .. } | DoorState::Closing { .. }) {
            return;
        }
        // A cut ID scanner wire: no card counts (Luna `requiresID`).
        let card = if self.id_scan { card } else { None };
        if allowed(&self.req_access, card) {
            self.toggle(now);
        } else if self.is_dense() {
            self.deny_until = Some(now + DENY_DS);
        }
    }

    /// Open a closed door or close an open one. Returns whether anything started.
    pub fn toggle(&mut self, now: f32) -> bool {
        if self.bolted || self.welded || !self.has_power() {
            return false;
        }
        match self.state {
            DoorState::Closed => {
                self.state = DoorState::Opening {
                    done_at: now + OPEN_DS,
                }
            }
            DoorState::Open { .. } => self.start_closing(now),
            // Operating: ignored.
            DoorState::Opening { .. } | DoorState::Closing { .. } => return false,
        }
        self.operations += 1;
        true
    }

    /// Luna's crowbar: an unpowered closed door can be prised open, and an open door can be
    /// pulled shut at any time, unless it is bolted. Uses no power.
    pub fn force(&mut self, now: f32) -> bool {
        if self.bolted || self.welded {
            return false;
        }
        match self.state {
            DoorState::Closed if !self.has_power() => {
                self.state = DoorState::Opening {
                    done_at: now + OPEN_DS,
                };
                true
            }
            DoorState::Open { .. } => {
                self.start_closing(now);
                true
            }
            _ => false,
        }
    }

    fn start_closing(&mut self, now: f32) {
        self.state = DoorState::Closing {
            push_at: now + CLOSE_PUSH_DS,
            done_at: now + CLOSE_DS,
            pushed: false,
        };
    }

    /// Only a fully open door can be walked through.
    pub fn is_dense(&self) -> bool {
        !matches!(self.state, DoorState::Open { .. })
    }

    /// Luna changes opacity at the end of each animation: a door blocks sight while
    /// closed or opening, and stays see-through while closing.
    pub fn is_opaque(&self) -> bool {
        matches!(self.state, DoorState::Closed | DoorState::Opening { .. })
    }
}

/// Advance door timers, push things out of closing doors, and keep `Dense` and
/// `Appearance` in step with the door state.
pub fn update_doors(
    time: Res<WorldTime>,
    map: Single<&MapLayer>,
    mut doors: Query<(Entity, &GridPos, &mut Airlock, &mut Appearance, Option<&Wiring>), Without<Mob>>,
    mut occupants: Query<
        (&mut GridPos, Has<Mob>, Option<&mut Outbox>),
        (Or<(With<Mob>, With<Item>)>, Without<Airlock>),
    >,
    dense: Query<&GridPos, (With<Dense>, Without<Airlock>, Without<Mob>, Without<Item>)>,
    mut commands: Commands,
) {
    let now = time.deciseconds();
    for (entity, door_pos, mut door, mut appearance, wiring) in &mut doors {
        match door.state {
            DoorState::Opening { done_at } if now >= done_at => {
                door.state = DoorState::Open {
                    autoclose_at: Some(now + AUTOCLOSE_DS),
                };
            }
            DoorState::Open {
                autoclose_at: Some(at),
            } if now >= at => {
                // Luna checks once; a bolted or unpowered door stays open for good.
                door.state = DoorState::Open { autoclose_at: None };
                if !door.bolted && door.has_power() {
                    door.start_closing(now);
                    door.operations += 1;
                }
            }
            DoorState::Closing {
                push_at,
                done_at,
                pushed,
            } => {
                if !pushed && now >= push_at {
                    push_out(door_pos.0, &map.0, &mut occupants, &dense);
                    door.state = DoorState::Closing {
                        push_at,
                        done_at,
                        pushed: true,
                    };
                }
                if now >= done_at {
                    door.state = DoorState::Closed;
                }
            }
            _ => {}
        }

        let mut entity = commands.entity(entity);
        if door.is_dense() {
            entity.insert(Dense);
        } else {
            entity.remove::<Dense>();
        }
        if door.is_opaque() {
            entity.insert(Opaque);
        } else {
            entity.remove::<Opaque>();
        }
        if door.deny_until.is_some_and(|until| now >= until) {
            door.deny_until = None;
        }
        let bolt_light = wiring.is_none_or(|w| w.bolt_light);
        let state = door_look(&door, bolt_light);
        if appearance.state != state {
            appearance.state = state;
        }
    }
}

/// Luna's airlock `update_icon`: bolted doors show the bolted sprite while the bolt light
/// works, and an open panel shows on a closed door (`+panel` overlay) and in the
/// animations (`o_` states). An open door shows no panel. A welded door shows its weld
/// (`+welded`).
pub fn door_look(door: &Airlock, bolt_light: bool) -> String {
    let base = match door.state {
        DoorState::Closed if door.deny_until.is_some() => "deny",
        DoorState::Closed if door.bolted && bolt_light => "locked",
        DoorState::Opening { .. } if door.panel_open => "o_opening",
        DoorState::Closing { .. } if door.panel_open => "o_closing",
        state => state.name(),
    };
    let mut look = base.to_owned();
    if door.state == DoorState::Closed {
        if door.panel_open {
            look.push_str("+panel");
        }
        if door.welded {
            look.push_str("+welded");
        }
    }
    look
}

/// Move every mob and item on `door` to the first free tile in the order N, S, E, W.
/// (Luna picks a random one; deterministic order keeps tests and replays stable.)
fn push_out(
    door: TilePos,
    map: &rs13_world::TileMap,
    occupants: &mut Query<
        (&mut GridPos, Has<Mob>, Option<&mut Outbox>),
        (Or<(With<Mob>, With<Item>)>, Without<Airlock>),
    >,
    dense: &Query<&GridPos, (With<Dense>, Without<Airlock>, Without<Mob>, Without<Item>)>,
) {
    let mob_tiles: Vec<TilePos> = occupants
        .iter()
        .filter(|(_, is_mob, _)| *is_mob)
        .map(|(p, ..)| p.0)
        .collect();
    let free = |p: TilePos| !map.is_solid(p) && !mob_tiles.contains(&p) && !dense.iter().any(|d| d.0 == p);
    let Some(target) = [Dir::North, Dir::South, Dir::East, Dir::West]
        .into_iter()
        .map(|d| door.step(d))
        .find(|p| free(*p))
    else {
        return;
    };
    for (mut pos, is_mob, outbox) in occupants.iter_mut() {
        if pos.0 != door {
            continue;
        }
        pos.0 = target;
        if is_mob && let Some(mut outbox) = outbox {
            outbox.0.push(LocMsg::new("airlock-pushed"));
        }
    }
}
