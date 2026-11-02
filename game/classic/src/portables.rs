//! Portable air pumps and scrubbers: canister-sized machines with a tank of their own,
//! wheeled up to where the air needs fixing. Spec: `docs/reference/portables.md`.

use bevy_ecs::prelude::*;
use rs13_simulation::atmos::gas::{T20C, TILE_VOLUME};
use rs13_simulation::atmos::{Gas, GasMix};
use rs13_world::{Appearance, Dense, GridPos, NetId, TilePos};

use crate::atmos::StationAtmos;
use crate::equipment::TankGas;
use crate::gas_panel::HeldTank;
use crate::machines::{ALL_SIDES, GasStore, MACHINE_TICKS, PipeMember, Portable, Switch, moles_for};
use crate::time::WorldTime;

/// The portable air pump prototype (Luna `portable_atmospherics/pump`).
pub const PORTABLE_PUMP: &str = "portable-pump";
/// The portable air scrubber prototype (Luna `portable_atmospherics/scrubber`).
pub const PORTABLE_SCRUBBER: &str = "portable-scrubber";
/// Their tanks, L (Luna `volume`).
pub const PUMP_VOLUME: f64 = 1000.0;
pub const SCRUBBER_VOLUME: f64 = 750.0;
/// A pump's target pressure, kPa (Luna `target_pressure`).
pub const PUMP_TARGET: f64 = 100.0;
/// A scrubber's intake, L per cycle (Luna `volume_rate`).
pub const SCRUBBER_RATE: f64 = 1800.0;
/// What a scrubber takes out of the air (Luna: plasma, CO₂, and the trace gases).
pub const SCRUBBED: [Gas; 3] = [Gas::Plasma, Gas::CarbonDioxide, Gas::NitrousOxide];

/// A portable pump: whether it releases its tank into the room (or siphons the room into
/// it), up to `target` kPa.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct PortablePump {
    pub out: bool,
    pub target: f64,
}

/// A portable scrubber.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct PortableScrubber {
    pub rate: f64,
}

pub fn is_portable(proto: &str) -> bool {
    matches!(proto, PORTABLE_PUMP | PORTABLE_SCRUBBER)
}

/// Spawn one, off and empty (Luna `New`: an empty tank at 20 °C).
pub fn spawn_portable(world: &mut World, proto: &str, pos: TilePos, id: NetId) -> Entity {
    let pump = proto == PORTABLE_PUMP;
    let volume = if pump { PUMP_VOLUME } else { SCRUBBER_VOLUME };
    let mut empty = GasMix::default();
    empty.temperature = T20C;
    let mut entity = world.spawn((
        GridPos(pos),
        id,
        Appearance::new(if pump { PORTABLE_PUMP } else { PORTABLE_SCRUBBER }, ""),
        Switch(false),
        PipeMember {
            volume,
            open: false,
            connects: ALL_SIDES,
        },
        GasStore(empty),
        Portable,
        Dense,
    ));
    if pump {
        entity.insert(PortablePump {
            out: false,
            target: PUMP_TARGET,
        });
    } else {
        entity.insert(PortableScrubber { rate: SCRUBBER_RATE });
    }
    entity.id()
}

/// Every machine cycle, each one that is on and not on a port works the gas of the tank
/// held in it, or else the air of its tile (Luna `process`). On a port its tank is part of
/// the pipes instead.
#[allow(clippy::type_complexity)]
pub fn run_portables(
    time: Res<WorldTime>,
    atmos: Option<ResMut<StationAtmos>>,
    mut pumps: Query<
        (
            Entity,
            &GridPos,
            &PortablePump,
            &Switch,
            &PipeMember,
            &mut GasStore,
        ),
        Without<PortableScrubber>,
    >,
    mut scrubbers: Query<(
        Entity,
        &GridPos,
        &PortableScrubber,
        &Switch,
        &PipeMember,
        &mut GasStore,
    )>,
    held: Query<&HeldTank>,
    mut tanks: Query<&mut TankGas>,
) {
    let Some(mut atmos) = atmos else { return };
    if !time.tick.0.is_multiple_of(MACHINE_TICKS) {
        return;
    }
    for (entity, pos, pump, on, member, mut store) in &mut pumps {
        if !on.0 || member.open {
            continue;
        }
        let mut tank = held.get(entity).ok().and_then(|h| tanks.get_mut(h.0).ok());
        if pump.out {
            let (env, volume) = match &tank {
                Some(t) => (t.mix.pressure(t.volume), t.volume),
                None => (atmos.0.pressure_at(pos.0), TILE_VOLUME),
            };
            let moles = moles_for(pump.target - env, volume, store.0.temperature);
            let moved = store.0.remove_moles(moles);
            match tank.as_mut() {
                Some(t) => t.mix.merge(&moved),
                None => atmos.0.add_at(pos.0, &moved),
            }
        } else {
            let delta = pump.target - store.0.pressure(member.volume);
            let moved = match tank.as_mut() {
                Some(t) => {
                    let moles = moles_for(delta, member.volume, t.mix.temperature);
                    t.mix.remove_moles(moles)
                }
                None => {
                    let room = atmos.0.air_at(pos.0).temperature;
                    atmos.0.remove_at(pos.0, moles_for(delta, member.volume, room))
                }
            };
            store.0.merge(&moved);
        }
    }
    for (entity, pos, scrubber, on, member, mut store) in &mut scrubbers {
        if !on.0 || member.open {
            continue;
        }
        let caught = match held.get(entity).ok().and_then(|h| tanks.get_mut(h.0).ok()) {
            // Luna samples min(1, rate / volume × 5) of the tank's gas.
            Some(mut t) => {
                let share = (scrubber.rate / t.volume * 5.0).min(1.0);
                let mut caught = GasMix::default();
                caught.temperature = t.mix.temperature;
                for gas in SCRUBBED {
                    let have = t.mix.get(gas);
                    t.mix.set(gas, have * (1.0 - share));
                    caught.set(gas, have * share);
                }
                caught
            }
            // Of the tile's air: all of it, at Luna's rate.
            None => {
                let share = (scrubber.rate / TILE_VOLUME * 5.0).min(1.0);
                atmos.0.remove_gases_at(pos.0, &SCRUBBED, share)
            }
        };
        store.0.merge(&caught);
    }
}

/// Luna `update_icon`: on or off, and a pump shows its connector while on a port.
pub fn update_portable_looks(
    mut pumps: Query<(&Switch, &PipeMember, &mut Appearance), With<PortablePump>>,
    mut scrubbers: Query<(&Switch, &mut Appearance), (With<PortableScrubber>, Without<PortablePump>)>,
) {
    for (on, member, mut look) in &mut pumps {
        let mut state = format!("psiphon:{}", u8::from(on.0));
        if member.open {
            state.push_str("+siphon-connector");
        }
        if look.state != state {
            look.state = state;
        }
    }
    for (on, mut look) in &mut scrubbers {
        let state = format!("pscrubber:{}", u8::from(on.0));
        if look.state != state {
            look.state = state;
        }
    }
}
