//! The sandbox panel (Luna's `h_sandbox`): in a sandbox round anyone can suit up and have building
//! materials and any item appear at their feet. Spec: `docs/reference/round.md`.

use bevy::prelude::*;
use rs13_classic::equipment::{Equipment, WornBy};
use rs13_classic::jobs::wear;
use rs13_classic::objects::{ItemProtos, spawn_object, spawn_stack};
use rs13_classic::round::{Phase, Round};
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{NoticeChannel, SandboxCommand, SandboxPanel};
use rs13_world::{GridPos, ObjectSpec, SandboxPanelView, Slot, TilePos};

/// What the panel's first line puts on.
const SUIT: [(Slot, &str); 4] = [
    (Slot::Suit, "space-suit"),
    (Slot::Head, "space-helmet"),
    (Slot::Mask, "gas-mask"),
    (Slot::Back, "jetpack"),
];

/// The panel's other lines: what appears, and how many.
const KITS: [(&str, u16); 9] = [
    ("metal-sheet", 50),
    ("glass-sheet", 50),
    ("airlock", 1),
    ("oxygen-canister", 1),
    ("nitrogen-canister", 1),
    ("carbon-dioxide-canister", 1),
    ("empty-canister", 1),
    ("toolbox-mechanical", 1),
    ("first-aid-kit", 1),
];

/// Whether anyone can have any item (an admin can turn it off, as Luna's `hsbtobj`).
#[derive(Resource, Debug)]
pub struct SandboxSpawning(pub bool);

impl Default for SandboxSpawning {
    fn default() -> Self {
        Self(true)
    }
}

pub fn is_sandbox(world: &World) -> bool {
    world
        .get_resource::<Round>()
        .is_some_and(|round| matches!(round.phase, Phase::Playing { mode: "sandbox" }))
}

/// Every item there is, in a fixed order, so a line means the same to client and server.
fn objects(world: &World) -> Vec<String> {
    let mut all: Vec<String> = world.resource::<ItemProtos>().0.keys().cloned().collect();
    all.sort();
    all
}

/// Take off what's in a slot and leave it on the floor, where Luna's suit-up leaves it.
fn drop_worn(world: &mut World, mob: Entity, slot: Slot, at: TilePos) {
    let worn = world
        .get_mut::<Equipment>(mob)
        .and_then(|mut e| e.0.remove(&slot));
    if let Some(item) = worn {
        world.entity_mut(item).remove::<WornBy>().insert(GridPos(at));
    }
}

/// Do what someone asked of the panel. Outside a sandbox round, nothing.
pub fn run_command(world: &mut World, link: Entity, mob: Entity, command: SandboxCommand) {
    if !is_sandbox(world) {
        return;
    }
    world.init_resource::<ItemProtos>();
    let Some(at) = world.get::<GridPos>(mob).map(|p| p.0) else {
        return;
    };
    match command {
        SandboxCommand::Open => {
            let spawning = world.get_resource_or_init::<SandboxSpawning>().0;
            let view = SandboxPanelView {
                suit: "sandbox-suit".to_owned(),
                kits: KITS.iter().map(|(id, n)| ((*id).to_owned(), *n)).collect(),
                objects: if spawning { objects(world) } else { Vec::new() },
            };
            if let Some(mut sender) = world.get_mut::<MessageSender<SandboxPanel>>(link) {
                sender.send::<NoticeChannel>(SandboxPanel(view));
            }
        }
        SandboxCommand::Suit => {
            for (slot, id) in SUIT {
                drop_worn(world, mob, slot, at);
                match spawn_object(world, &ObjectSpec::new(id), at) {
                    Ok(item) => wear(world, mob, item),
                    Err(problem) => warn!(problem, "sandbox suit"),
                }
            }
        }
        SandboxCommand::Kit(line) => {
            let Some((id, amount)) = KITS.get(usize::from(line)).copied() else {
                return;
            };
            let made = if amount > 1 {
                spawn_stack(world, id, at, u32::from(amount)).map(|_| ())
            } else {
                spawn_object(world, &ObjectSpec::new(id), at).map(|_| ())
            };
            if let Err(problem) = made {
                warn!(problem, "sandbox kit");
            }
        }
        SandboxCommand::Spawn(line) => {
            if !world.get_resource_or_init::<SandboxSpawning>().0 {
                return;
            }
            let Some(id) = objects(world).into_iter().nth(usize::from(line)) else {
                return;
            };
            if let Err(problem) = spawn_object(world, &ObjectSpec::new(id), at) {
                warn!(problem, "sandbox object");
            }
        }
    }
}
