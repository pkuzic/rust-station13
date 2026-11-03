//! Computer frames: built up from a sheet recipe with a circuit board, cables and glass into a
//! working computer, and taken back apart the same way (Luna's `buildandrepair.dm`). Spec:
//! `docs/reference/construction.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, Dense, GridPos, NetId, ObjectSpec, TilePos};

use crate::actions::{Outbox, in_reach};
use crate::closets::InCloset;
use crate::construction::{GLASS, Job, METAL, Work};
use crate::hands::{Hands, HeldBy, Stack};
use crate::objects::{spawn_object, spawn_stack};
use crate::time::WorldTime;
use crate::welding::Welder;

pub const FRAME: &str = "computer-frame";
/// The cables and glass a frame takes (Luna's 5 and 2), and the metal it's made of.
const CABLES: u32 = 5;
const GLASS_PANES: u32 = 2;
const FRAME_METAL: u32 = 5;
/// How long a wrench takes, or taking a computer down, ds (Luna's 20).
const WRENCH_DS: f32 = 20.0;

/// Which board makes which computer (Luna's `computertype`), of the computers built here.
pub const BOARDS: [(&str, &str); 8] = [
    ("board-security", "security-console"),
    ("board-med-data", "medical-records"),
    ("board-secure-data", "security-records"),
    ("board-card", "id-computer"),
    ("board-comms", "comms-console"),
    ("board-arcade", "arcade"),
    ("board-cloning", "cloning-console"),
    ("board-power-monitor", "power-monitor"),
];

pub fn is_computer(proto: &str) -> bool {
    BOARDS.iter().any(|(_, computer)| *computer == proto)
}

fn board_of(computer: &str) -> Option<&'static str> {
    BOARDS.iter().find(|(_, c)| *c == computer).map(|(b, _)| *b)
}

fn computer_of(board: &str) -> Option<&'static str> {
    BOARDS.iter().find(|(b, _)| *b == board).map(|(_, c)| *c)
}

/// How far along it is: 0 loose, 1 fastened down, 2 with its board screwed in, 3 wired, 4 glazed.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ComputerFrame {
    pub state: u8,
    pub board: Option<Entity>,
}

pub fn spawn_frame(world: &mut World, pos: TilePos, id: NetId) -> Entity {
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(FRAME, "0"),
            Dense,
            ComputerFrame::default(),
        ))
        .id()
}

fn sprite(world: &World, thing: Entity) -> Option<String> {
    world.get::<Appearance>(thing).map(|look| look.sprite.clone())
}

fn tell(world: &mut World, mob: Entity, id: &str) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(LocMsg::new(id));
    }
}

fn set_state(world: &mut World, frame: Entity, state: u8, board: Option<Entity>) {
    world.entity_mut(frame).insert((
        ComputerFrame { state, board },
        // Luna draws a loose or fastened frame the same, and one with a board loose in it as "1".
        Appearance::new(FRAME, look(state, board.is_some())),
    ));
}

fn look(state: u8, has_board: bool) -> &'static str {
    match (state, has_board) {
        (0 | 1, false) => "0",
        (0 | 1, true) => "1",
        (2, _) => "2",
        (3, _) => "3",
        _ => "4",
    }
}

fn take_from_stack(world: &mut World, mob: Entity, stack: Entity, amount: u32) -> bool {
    let Some(mut pile) = world.get_mut::<Stack>(stack) else {
        return false;
    };
    if pile.amount < amount {
        return false;
    }
    pile.amount -= amount;
    if pile.amount == 0 {
        if let Some(mut hands) = world.get_mut::<Hands>(mob) {
            if hands.left == Some(stack) {
                hands.left = None;
            }
            if hands.right == Some(stack) {
                hands.right = None;
            }
        }
        world.entity_mut(stack).despawn();
    }
    true
}

fn board_out(world: &mut World, board: Entity, at: TilePos) {
    world.entity_mut(board).remove::<InCloset>().insert(GridPos(at));
}

/// A tool on a frame, step by step (Luna `frame/computer/attackby`).
pub fn use_on_frame(world: &mut World, mob: Entity, tool: Entity, target: Entity) {
    let Some(frame) = world.get::<ComputerFrame>(target).copied() else {
        return;
    };
    let (Some(from), Some(at)) = (
        world.get::<GridPos>(mob).map(|p| p.0),
        world.get::<GridPos>(target).map(|p| p.0),
    ) else {
        return;
    };
    if !in_reach(from, at) {
        return;
    }
    let Some(held) = sprite(world, tool) else {
        return;
    };
    let is_board = computer_of(&held).is_some();
    match (frame.state, held.as_str()) {
        (0 | 1, "wrench") => {
            let done_at = world.resource::<WorldTime>().deciseconds() + WRENCH_DS;
            world.entity_mut(mob).insert(Job {
                work: Work::WrenchFrame(target),
                tool,
                at: from,
                done_at,
            });
        }
        (0, "welder") if world.get::<Welder>(tool).is_some_and(|w| w.lit) => {
            if let Some(board) = frame.board {
                board_out(world, board, at);
            }
            world.entity_mut(target).despawn();
            let _ = spawn_stack(world, METAL, at, FRAME_METAL);
        }
        (1, _) if is_board && frame.board.is_none() => {
            if let Some(mut hands) = world.get_mut::<Hands>(mob) {
                if hands.left == Some(tool) {
                    hands.left = None;
                }
                if hands.right == Some(tool) {
                    hands.right = None;
                }
            }
            world.entity_mut(tool).remove::<HeldBy>().insert(InCloset(target));
            set_state(world, target, 1, Some(tool));
        }
        (1, "screwdriver") if frame.board.is_some() => set_state(world, target, 2, frame.board),
        (1, "crowbar") => {
            if let Some(board) = frame.board {
                board_out(world, board, at);
                set_state(world, target, 1, None);
            }
        }
        (2, "screwdriver") => set_state(world, target, 1, frame.board),
        (2, crate::cables::COIL) => {
            if take_from_stack(world, mob, tool, CABLES) {
                set_state(world, target, 3, frame.board);
            } else {
                tell(world, mob, "build-need-more");
            }
        }
        (3, "wirecutters") => {
            set_state(world, target, 2, frame.board);
            let _ = spawn_stack(world, crate::cables::COIL, at, CABLES);
        }
        (3, GLASS) => {
            if take_from_stack(world, mob, tool, GLASS_PANES) {
                set_state(world, target, 4, frame.board);
            } else {
                tell(world, mob, "build-need-more");
            }
        }
        (4, "crowbar") => {
            set_state(world, target, 3, frame.board);
            let _ = spawn_stack(world, GLASS, at, GLASS_PANES);
        }
        (4, "screwdriver") => {
            let Some(board) = frame.board else {
                return;
            };
            let Some(computer) = sprite(world, board).and_then(|b| computer_of(&b)) else {
                return;
            };
            world.entity_mut(board).despawn();
            world.entity_mut(target).despawn();
            let _ = spawn_object(world, &ObjectSpec::new(computer), at);
        }
        _ => {}
    }
}

/// A screwdriver on a computer starts taking it down.
pub fn start_taking_down(world: &mut World, mob: Entity, tool: Entity, computer: Entity) {
    let Some(from) = world.get::<GridPos>(mob).map(|p| p.0) else {
        return;
    };
    let near = world
        .get::<GridPos>(computer)
        .is_some_and(|p| in_reach(from, p.0));
    if !near {
        return;
    }
    let done_at = world.resource::<WorldTime>().deciseconds() + WRENCH_DS;
    world.entity_mut(mob).insert(Job {
        work: Work::TakeDownComputer(computer),
        tool,
        at: from,
        done_at,
    });
}

/// Finished with the wrench: fastened if it was loose, loose if it was fastened.
pub fn finish_wrench(world: &mut World, frame: Entity) {
    if let Some(state) = world.get::<ComputerFrame>(frame).copied() {
        let next = if state.state == 0 { 1 } else { 0 };
        set_state(world, frame, next, state.board);
    }
}

/// A computer back to a glazed frame, fastened down, its board inside (Luna `computer/attackby`).
pub fn finish_take_down(world: &mut World, computer: Entity) {
    let (Some(at), Some(proto)) = (
        world.get::<GridPos>(computer).map(|p| p.0),
        sprite(world, computer),
    ) else {
        return;
    };
    let Some(board_proto) = board_of(&proto) else {
        return;
    };
    world.entity_mut(computer).despawn();
    let Ok(frame) = spawn_object(world, &ObjectSpec::new(FRAME), at) else {
        return;
    };
    let Ok(board) = spawn_object(world, &ObjectSpec::new(board_proto), at) else {
        return;
    };
    world
        .entity_mut(board)
        .remove::<GridPos>()
        .insert(InCloset(frame));
    set_state(world, frame, 4, Some(board));
}
