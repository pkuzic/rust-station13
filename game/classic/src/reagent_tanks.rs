//! Big tanks of something to fill a beaker from (Luna's `reagent_dispensers`): a pepper refiller, a
//! water cooler and a beer keg, and the water and fuel tanks. Spec: `docs/reference/chemistry.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, Dense, GridPos, NetId, TilePos};

use crate::actions::Outbox;
use crate::chemistry::{Reagents, react, transfer};
use crate::extinguishers::WaterTank;
use crate::hands::Hands;
use crate::welding::FuelTank;

pub const PEPPER_TANK: &str = "pepper-tank";
pub const WATER_COOLER: &str = "water-cooler";
pub const BEER_KEG: &str = "beer-keg";
/// How much a click on a water or fuel tank takes (Luna `amount_per_transfer_from_this`).
pub const TANK_TRANSFER: u32 = 10;

/// Something to fill from: how much a click takes.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ReagentTank {
    pub transfer: f32,
}

/// Its look, whether it stands in the way, how much a click takes, and what it starts with.
fn kind(proto: &str) -> Option<(&'static str, bool, f32, &'static str, f32)> {
    Some(match proto {
        PEPPER_TANK => ("peppertank", false, 50.0, "condensedcapsaicin", 1000.0),
        WATER_COOLER => ("water_cooler", true, 5.0, "water", 500.0),
        BEER_KEG => ("beertankTEMP", true, 10.0, "beer", 1000.0),
        _ => return None,
    })
}

pub fn is_tank(proto: &str) -> bool {
    kind(proto).is_some()
}

pub fn spawn_tank(world: &mut World, proto: &str, pos: TilePos, id: NetId) -> Option<Entity> {
    let (look, dense, transfer, reagent, amount) = kind(proto)?;
    let mut tank = world.spawn((
        GridPos(pos),
        id,
        Appearance::new(proto, look),
        ReagentTank { transfer },
        Reagents::with(amount, reagent, amount),
    ));
    if dense {
        tank.insert(Dense);
    }
    Some(tank.id())
}

fn tell(world: &mut World, mob: Entity, msg: LocMsg) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(msg);
    }
}

/// A water or fuel tank keeps its own count, for extinguishers and welders: take from that.
fn draw_count(world: &mut World, tank: Entity, room: f32) -> Option<(&'static str, f32)> {
    let wanted = (TANK_TRANSFER as f32).min(room) as u32;
    if let Some(mut water) = world.get_mut::<WaterTank>(tank) {
        let taken = wanted.min(water.water);
        water.water -= taken;
        return Some(("water", taken as f32));
    }
    let mut fuel = world.get_mut::<FuelTank>(tank)?;
    let taken = wanted.min(fuel.fuel);
    fuel.fuel -= taken;
    Some(("fuel", taken as f32))
}

/// A click with a beaker: it fills (Luna `glass/afterattack`).
pub fn fill(world: &mut World, mob: Entity, tank: Entity) {
    let Some(glass) = world.get::<Hands>(mob).and_then(Hands::active_item) else {
        return;
    };
    let Some(room) = world.get::<Reagents>(glass).map(|r| r.max - r.total()) else {
        return;
    };
    if room <= 0.0 {
        tell(world, mob, LocMsg::new("tank-glass-full"));
        return;
    }
    let moved = if let Some(amount) = world.get::<ReagentTank>(tank).map(|t| t.transfer) {
        transfer(world, tank, glass, amount)
    } else {
        let Some((reagent, taken)) = draw_count(world, tank, room) else {
            return;
        };
        if let Some(mut into) = world.get_mut::<Reagents>(glass) {
            into.add(reagent, taken);
            react(&mut into);
        }
        taken
    };
    if moved > 0.0 {
        tell(
            world,
            mob,
            LocMsg::new("tank-filled").num("value", moved.round() as i64),
        );
    } else {
        tell(world, mob, LocMsg::new("tank-empty"));
    }
}
