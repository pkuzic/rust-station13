//! Fixtures that only stand there for now: a look, and whether they block the way and the view.
//! Spec: `docs/reference/scenery.md`.

use bevy_ecs::prelude::*;
use rs13_world::{Appearance, Dense, GridPos, NetId, Opaque, TilePos};

/// Each one: its prototype, whether it's dense, and whether it blocks the view.
const SCENERY: [(&str, bool, bool); 24] = [
    ("shuttle-engine", true, true),
    ("shuttle-engine-left", true, true),
    ("shuttle-engine-right", true, true),
    ("pod-engine", true, true),
    ("shuttle-heater", true, false),
    ("holopad", false, false),
    ("plastic-flaps", true, true),
    ("noticeboard", false, false),
    ("mop-bucket", true, false),
    ("sign-deathsposal", false, false),
    ("sign-nosmoking", false, false),
    ("sign-fire", false, false),
    ("meat-spike", true, false),
    ("display-case", true, false),
    ("washer", true, false),
    ("bookcase", true, false),
    ("laser-machine", false, false),
    ("turntable", false, false),
    ("air-sensor", false, false),
    ("airlock-sensor", false, false),
    ("doppler-array", true, false),
    ("party-button", false, false),
    ("mining-laser", true, false),
    ("scanner", true, false),
];

/// Plastic flaps (Luna's `plasticflaps`): dense to people and to air, but things go through, such as
/// what a conveyor carries and shots.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlasticFlaps;

pub fn is_scenery(proto: &str) -> bool {
    SCENERY.iter().any(|(id, ..)| *id == proto)
}

pub fn spawn_scenery(world: &mut World, proto: &str, pos: TilePos, id: NetId) -> Entity {
    let mut thing = world.spawn((GridPos(pos), id, Appearance::new(proto, "")));
    if proto == "plastic-flaps" {
        thing.insert(PlasticFlaps);
    }
    if let Some((_, dense, opaque)) = SCENERY.iter().find(|(id, ..)| *id == proto) {
        if *dense {
            thing.insert(Dense);
        }
        if *opaque {
            thing.insert(Opaque);
        }
    }
    thing.id()
}
