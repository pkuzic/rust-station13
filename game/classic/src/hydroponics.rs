//! Plant pots (Luna's `plantpot`): plant a seed, keep it watered, and harvest what it grows.
//! Spec: `docs/reference/hydroponics.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, Dense, GridPos, NetId, ObjectSpec, TilePos};

use crate::actions::Outbox;
use crate::chemistry::Reagents;
use crate::hands::{Hands, HeldBy};
use crate::machines::MACHINE_TICKS;
use crate::objects::spawn_object;
use crate::rng::GameRng;
use crate::time::WorldTime;

pub const PLANT_POT: &str = "plant-pot";
/// What a pot holds, and the water it starts with (Luna `plantpot/New`).
pub const POT_SIZE: f32 = 360.0;
pub const POT_WATER: f32 = 200.0;
/// How much a plant grows a cycle. Luna's is 1, which takes up to 43 minutes to first ripen: too
/// long to see in most rounds.
pub const GROWTH_PER_CYCLE: i32 = 3;
/// What a plant drinks a cycle, so a pot needs watering in the end, and too much soon goes.
pub const DRINKS: f32 = 0.5;

/// A kind of plant (Luna's `datum/plant`).
#[derive(Debug, PartialEq)]
pub struct Plant {
    pub id: &'static str,
    pub crop: &'static str,
    pub seed: &'static str,
    pub health: i32,
    /// Cycles until it's grown, and until it first has something to pick.
    pub grow: i32,
    pub ripe: i32,
    pub crop_size: u32,
    pub harvests: u32,
    /// The chance, out of 100, of it not suffering from thirst in a cycle.
    pub endurance: i32,
    /// How many stages of growth it has sprites for.
    pub stages: u8,
}

pub const PLANTS: [Plant; 8] = [
    Plant {
        id: "tomato",
        crop: "tomato",
        seed: "seed-tomato",
        health: 20,
        grow: 375,
        ripe: 550,
        crop_size: 2,
        harvests: 3,
        endurance: 3,
        stages: 3,
    },
    Plant {
        id: "apple",
        crop: "apple",
        seed: "seed-apple",
        health: 40,
        grow: 1000,
        ripe: 1300,
        crop_size: 2,
        harvests: 10,
        endurance: 5,
        stages: 3,
    },
    Plant {
        id: "banana",
        crop: "banana",
        seed: "seed-banana",
        health: 15,
        grow: 600,
        ripe: 800,
        crop_size: 3,
        harvests: 4,
        endurance: 3,
        stages: 3,
    },
    Plant {
        id: "lemon",
        crop: "lemon",
        seed: "seed-lemon",
        health: 30,
        grow: 500,
        ripe: 650,
        crop_size: 2,
        harvests: 3,
        endurance: 3,
        stages: 3,
    },
    Plant {
        id: "lime",
        crop: "lime",
        seed: "seed-lime",
        health: 30,
        grow: 150,
        ripe: 500,
        crop_size: 2,
        harvests: 3,
        endurance: 3,
        stages: 3,
    },
    Plant {
        id: "orange",
        crop: "orange",
        seed: "seed-orange",
        health: 20,
        grow: 300,
        ripe: 500,
        crop_size: 1,
        harvests: 3,
        endurance: 3,
        stages: 3,
    },
    Plant {
        id: "carrot",
        crop: "carrot",
        seed: "seed-carrot",
        health: 20,
        grow: 250,
        ripe: 500,
        crop_size: 3,
        harvests: 1,
        endurance: 5,
        stages: 2,
    },
    Plant {
        id: "potato",
        crop: "potato",
        seed: "seed-potato",
        health: 40,
        grow: 400,
        ripe: 800,
        crop_size: 2,
        harvests: 1,
        endurance: 10,
        stages: 2,
    },
];

/// What's growing in a pot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Growing {
    pub plant: usize,
    pub growth: i32,
    pub health: i32,
    pub harvests: u32,
    pub dead: bool,
}

#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct PlantPot {
    pub growing: Option<Growing>,
}

pub fn spawn_pot(world: &mut World, pos: TilePos, id: NetId) -> Entity {
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(PLANT_POT, "pot-empty"),
            Dense,
            PlantPot::default(),
            Reagents::with(POT_SIZE, "water", POT_WATER),
        ))
        .id()
}

fn tell(world: &mut World, mob: Entity, msg: LocMsg) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(msg);
    }
}

/// Luna `update_icon`: the pot, with the plant at its stage over it.
fn look(growing: Option<Growing>) -> String {
    let Some(growing) = growing else {
        return "pot-empty".to_owned();
    };
    let plant = &PLANTS[growing.plant];
    if growing.dead {
        return "pot-empty".to_owned();
    }
    let stage = if growing.growth >= plant.ripe {
        plant.stages
    } else if growing.growth >= plant.grow {
        2
    } else {
        1
    };
    format!("pot-empty+{}-G{stage}", plant.id)
}

fn set_look(world: &mut World, pot: Entity) {
    let growing = world.get::<PlantPot>(pot).and_then(|p| p.growing);
    if let Some(mut appearance) = world.get_mut::<Appearance>(pot) {
        appearance.state = look(growing);
    }
}

/// A click: a seed goes in an empty pot; a ripe plant is picked; a dead one is cleared away (Luna
/// `attackby`, `attack_hand`).
pub fn click(world: &mut World, mob: Entity, pot: Entity) {
    let Some(state) = world.get::<PlantPot>(pot).copied() else {
        return;
    };
    let held = world.get::<Hands>(mob).and_then(Hands::active_item);
    if let Some(item) = held {
        let proto = world
            .get::<Appearance>(item)
            .map(|a| a.sprite.clone())
            .unwrap_or_default();
        let Some(line) = PLANTS.iter().position(|p| p.seed == proto) else {
            return;
        };
        if state.growing.is_some() {
            tell(world, mob, LocMsg::new("pot-occupied"));
            return;
        }
        if let Some(mut hands) = world.get_mut::<Hands>(mob) {
            let active = hands.active;
            *hands.slot_mut(active) = None;
        }
        world.entity_mut(item).remove::<HeldBy>();
        world.entity_mut(item).despawn();
        let plant = &PLANTS[line];
        if let Some(mut p) = world.get_mut::<PlantPot>(pot) {
            p.growing = Some(Growing {
                plant: line,
                growth: 0,
                health: plant.health,
                harvests: plant.harvests,
                dead: false,
            });
        }
        tell(world, mob, LocMsg::new("pot-planted"));
        set_look(world, pot);
        return;
    }
    let Some(growing) = state.growing else {
        return;
    };
    let plant = &PLANTS[growing.plant];
    if growing.dead {
        if let Some(mut p) = world.get_mut::<PlantPot>(pot) {
            p.growing = None;
        }
        tell(world, mob, LocMsg::new("pot-cleared"));
    } else if growing.growth >= plant.ripe {
        let Some(at) = world.get::<GridPos>(mob).map(|p| p.0) else {
            return;
        };
        for _ in 0..plant.crop_size {
            let _ = spawn_object(world, &ObjectSpec::new(plant.crop), at);
        }
        let left = growing.harvests.saturating_sub(1);
        if let Some(mut p) = world.get_mut::<PlantPot>(pot) {
            p.growing = (left > 0).then_some(Growing {
                growth: plant.grow,
                harvests: left,
                ..growing
            });
        }
        tell(
            world,
            mob,
            LocMsg::new("pot-harvested").num("value", i64::from(plant.crop_size)),
        );
    } else {
        tell(world, mob, LocMsg::new("pot-not-ready"));
    }
    set_look(world, pot);
}

/// What someone looking at a pot learns.
pub fn examine(world: &World, pot: Entity) -> Option<LocMsg> {
    let state = world.get::<PlantPot>(pot)?;
    let water = world.get::<Reagents>(pot).map_or(0.0, |r| r.get("water"));
    let id = match state.growing {
        None => "examine-pot-empty",
        Some(g) if g.dead => "examine-pot-dead",
        Some(g) if g.growth >= PLANTS[g.plant].ripe => "examine-pot-ripe",
        Some(_) => "examine-pot-growing",
    };
    Some(LocMsg::new(id).num("value", water.round() as i64))
}

/// Each cycle a watered plant grows, and a thirsty one suffers (Luna `plantpot/process`).
pub fn run_pots(world: &mut World) {
    if !world.resource::<WorldTime>().tick.0.is_multiple_of(MACHINE_TICKS) {
        return;
    }
    let mut pots = world.query::<(Entity, &PlantPot, &Reagents)>();
    let all: Vec<(Entity, Growing, f32)> = pots
        .iter(world)
        .filter_map(|(e, p, r)| p.growing.map(|g| (e, g, r.get("water"))))
        .collect();
    for (pot, mut growing, water) in all {
        if growing.dead {
            continue;
        }
        let plant = &PLANTS[growing.plant];
        if water > 0.0 {
            growing.growth += GROWTH_PER_CYCLE;
            if let Some(mut reagents) = world.get_mut::<Reagents>(pot) {
                reagents.remove("water", DRINKS);
            }
        } else if world.resource_mut::<GameRng>().range(0, 99) >= plant.endurance {
            growing.health -= 1;
            if growing.health <= 0 {
                growing.dead = true;
            }
        }
        if let Some(mut p) = world.get_mut::<PlantPot>(pot) {
            p.growing = Some(growing);
        }
        set_look(world, pot);
    }
}
