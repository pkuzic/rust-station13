//! Building by hand: what metal sheets make, girders and the walls on them, grilles and window panes,
//! and taking walls apart (Luna's `stack_recipe`, `girders.dm`, `grille.dm`, `glass.dm`). Spec:
//! `docs/reference/construction.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_simulation::atmos::Cell;
use rs13_world::{
    Appearance, BuildPanelView, Dense, Dir, Facing, GridPos, MapLayer, NetId, ObjectSpec, TileDef, TilePos,
};

use crate::actions::{Outbox, in_reach};
use crate::atmos::StationAtmos;
use crate::hands::{Hands, HeldBy, Stack};
use crate::objects::{spawn_object, spawn_stack};
use crate::time::WorldTime;
use crate::welding::Welder;
use crate::windows::{WINDOW_PANE, WindowPane};
use crate::wires::PanelViewer;

pub const METAL: &str = "metal-sheet";
pub const GLASS: &str = "glass-sheet";
pub const PLASTEEL: &str = "plasteel-sheet";
pub const RODS: &str = "rods";
pub const GIRDER: &str = "girder";
pub const REINFORCED_GIRDER: &str = "girder-reinforced";

/// What a stack makes: what, how many sheets, how many come of it, and how long it takes, ds. Luna's
/// `metal_recipes`, of what exists here (`construction.md`).
pub const RECIPES: [(&str, u32, u32, f32); 11] = [
    ("stool", 1, 1, 0.0),
    ("chair", 1, 1, 0.0),
    ("bed", 2, 1, 0.0),
    ("table-parts", 2, 1, 0.0),
    ("rack-parts", 1, 1, 0.0),
    ("closet", 2, 1, 15.0),
    ("floor-tile", 1, 4, 0.0),
    (RODS, 1, 2, 0.0),
    (PLASTEEL, 2, 1, 0.0),
    (GIRDER, 2, 1, 50.0),
    (crate::frames::FRAME, 5, 1, 25.0),
];
/// Things that fill a tile, so only one can be built there.
const ONE_PER_TILE: [&str; 6] = ["stool", "chair", "bed", "closet", GIRDER, crate::frames::FRAME];
/// How long it takes, ds (Luna's `do_after`s).
const UNWELD_DS: f32 = 100.0;
const PLATE_DS: f32 = 40.0;
const REINFORCE_GIRDER_DS: f32 = 60.0;
const FINISH_REINFORCED_DS: f32 = 50.0;
const TAKE_APART_DS: f32 = 40.0;
/// Most panes on a tile: one to each side.
const MOST_PANES: usize = 4;

/// A girder: dense, not yet a wall.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Girder {
    pub reinforced: bool,
}

/// What someone is in the middle of.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Work {
    /// Make this line of `RECIPES` out of the stack in hand.
    Recipe(usize),
    /// Take the plating off a wall.
    Unweld(TilePos),
    /// Plate a girder with the stack in hand.
    Plate(Entity),
    /// Take a girder apart with the wrench in hand.
    TakeApart(Entity),
    /// Fasten a computer frame down, or loosen it (`frames.rs`).
    WrenchFrame(Entity),
    /// Take a computer back to its frame.
    TakeDownComputer(Entity),
}

/// Someone busy at something, who has to stay put holding the tool (Luna `do_after`).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Job {
    pub work: Work,
    pub tool: Entity,
    pub at: TilePos,
    pub done_at: f32,
}

/// What a mob asked of an open stack window: the stack, and the line.
#[derive(Component, Debug, Default)]
pub struct PendingBuildOps(pub Vec<(Entity, usize)>);

/// What a mob's open stack window shows.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct BuildPanelOut(pub Option<BuildPanelView>);

pub fn spawn_girder(world: &mut World, proto: &str, pos: TilePos, id: NetId) -> Entity {
    let reinforced = proto == REINFORCED_GIRDER;
    let look = if reinforced { "reinforced" } else { "girder" };
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(proto, look),
            Dense,
            Girder { reinforced },
        ))
        .id()
}

fn tell(world: &mut World, mob: Entity, id: &str) {
    if let Some(mut outbox) = world.get_mut::<Outbox>(mob) {
        outbox.0.push(LocMsg::new(id));
    }
}

fn start(world: &mut World, mob: Entity, tool: Entity, work: Work, takes_ds: f32) {
    let Some(at) = world.get::<GridPos>(mob).map(|p| p.0) else {
        return;
    };
    let done_at = world.resource::<WorldTime>().deciseconds() + takes_ds;
    world.entity_mut(mob).insert(Job {
        work,
        tool,
        at,
        done_at,
    });
}

/// Take from a stack in hand, letting go of it if that was the last.
fn use_up(world: &mut World, mob: Entity, stack: Entity, amount: u32) -> bool {
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

fn sprite(world: &World, thing: Entity) -> Option<String> {
    world.get::<Appearance>(thing).map(|look| look.sprite.clone())
}

fn dense_here(world: &mut World, at: TilePos) -> bool {
    let mut dense = world.query_filtered::<&GridPos, (With<Dense>, Without<crate::movement::Mob>)>();
    dense.iter(world).any(|p| p.0 == at)
}

/// A stack used in hand. Metal opens its list, glass puts up a pane, and rods a grille.
pub fn use_stack(world: &mut World, mob: Entity, stack: Entity) {
    match sprite(world, stack).as_deref() {
        Some(METAL) => {
            world.entity_mut(mob).insert(PanelViewer(stack));
        }
        Some(GLASS) => build_pane(world, mob, stack),
        Some(RODS) => build_grille(world, mob, stack),
        _ => {}
    }
}

/// A pane on your own tile, on the side you're facing or the next free one (Luna `construct_window`).
fn build_pane(world: &mut World, mob: Entity, stack: Entity) {
    let Some(at) = world.get::<GridPos>(mob).map(|p| p.0) else {
        return;
    };
    let mut panes = world.query::<(&GridPos, &WindowPane)>();
    let taken: Vec<Dir> = panes
        .iter(world)
        .filter(|(p, _)| p.0 == at)
        .map(|(_, w)| w.dir)
        .collect();
    if taken.len() >= MOST_PANES {
        tell(world, mob, "build-too-many-panes");
        return;
    }
    let facing = world.get::<Facing>(mob).copied();
    let sides = [Dir::North, Dir::East, Dir::South, Dir::West];
    let first = sides
        .iter()
        .position(|d| Some(Facing::of(*d)) == facing)
        .unwrap_or(0);
    let side = (0..4)
        .map(|i| sides[(first + i) % 4])
        .find(|d| !taken.contains(d));
    let Some(side) = side else {
        return;
    };
    if !use_up(world, mob, stack, 1) {
        return;
    }
    let mut spec = ObjectSpec::new(WINDOW_PANE);
    spec.dir = Some(side);
    let _ = spawn_object(world, &spec, at);
}

/// Two rods make a grille on your own tile, which then blocks the way (Luna `rods/attack_self`).
fn build_grille(world: &mut World, mob: Entity, stack: Entity) {
    let Some(at) = world.get::<GridPos>(mob).map(|p| p.0) else {
        return;
    };
    let tile = tile_id(world, at);
    if tile.as_deref() == Some("grille") || tile.as_deref() == Some("space") {
        return;
    }
    if world.get::<Stack>(stack).is_none_or(|s| s.amount < 2) {
        tell(world, mob, "build-need-more");
        return;
    }
    use_up(world, mob, stack, 2);
    set_tile(world, at, "grille", true, Some(false), Some(false));
}

fn tile_id(world: &mut World, at: TilePos) -> Option<String> {
    let mut maps = world.query::<&MapLayer>();
    maps.iter(world)
        .find(|m| m.0.z == at.z)
        .and_then(|m| m.0.tile(at).map(|t| t.id.clone()))
}

fn set_tile(
    world: &mut World,
    at: TilePos,
    id: &str,
    solid: bool,
    opaque: Option<bool>,
    airtight: Option<bool>,
) {
    let mut maps = world.query::<&mut MapLayer>();
    if let Some(mut map) = maps.iter_mut(world).find(|m| m.0.z == at.z) {
        map.0.set_tile(
            at,
            TileDef {
                id: id.to_owned(),
                solid,
                opaque,
                airtight,
            },
        );
    }
    let cell = if solid && airtight != Some(false) {
        Cell::Solid
    } else {
        Cell::Open
    };
    if let Some(mut atmos) = world.get_resource_mut::<StationAtmos>() {
        atmos.0.set_cell(at, cell);
    }
}

/// A tool on a tile itself. A lit welding tool starts on a wall, and wirecutters take a grille down.
pub fn use_on_tile(world: &mut World, mob: Entity, tool: Entity, at: TilePos) {
    let Some(from) = world.get::<GridPos>(mob).map(|p| p.0) else {
        return;
    };
    if !in_reach(from, at) {
        return;
    }
    match (sprite(world, tool).as_deref(), tile_id(world, at).as_deref()) {
        (Some("wirecutters"), Some("grille")) => {
            set_tile(world, at, "plating", false, None, None);
            let _ = spawn_stack(world, RODS, at, 2);
        }
        (Some("welder"), Some("wall")) => {
            let lit = world.get::<Welder>(tool).is_some_and(|w| w.lit && w.fuel > 5);
            if !lit {
                tell(world, mob, "build-need-fuel");
                return;
            }
            if let Some(mut welder) = world.get_mut::<Welder>(tool) {
                welder.fuel -= 5;
            }
            tell(world, mob, "build-unwelding");
            start(world, mob, tool, Work::Unweld(at), UNWELD_DS);
        }
        (Some("welder"), Some("r-wall")) => tell(world, mob, "build-too-tough"),
        _ => {}
    }
}

/// A tool or a stack on a girder or a pane.
pub fn use_on_thing(world: &mut World, mob: Entity, tool: Entity, target: Entity) {
    let Some(from) = world.get::<GridPos>(mob).map(|p| p.0) else {
        return;
    };
    let Some(at) = world.get::<GridPos>(target).map(|p| p.0) else {
        return;
    };
    if !in_reach(from, at) {
        return;
    }
    let held = sprite(world, tool);
    if world.get::<WindowPane>(target).is_some() {
        // Luna unfastens it with a screwdriver first, which nothing else here uses.
        if held.as_deref() == Some("wrench") {
            world.entity_mut(target).despawn();
            let _ = spawn_stack(world, GLASS, at, 1);
        }
        return;
    }
    let Some(girder) = world.get::<Girder>(target).copied() else {
        return;
    };
    match held.as_deref() {
        Some("wrench") => start(world, mob, tool, Work::TakeApart(target), TAKE_APART_DS),
        Some(METAL) if !girder.reinforced => {
            if world.get::<Stack>(tool).is_none_or(|s| s.amount < 2) {
                tell(world, mob, "build-need-more");
                return;
            }
            start(world, mob, tool, Work::Plate(target), PLATE_DS);
        }
        Some(PLASTEEL) => {
            let takes = if girder.reinforced {
                FINISH_REINFORCED_DS
            } else {
                REINFORCE_GIRDER_DS
            };
            start(world, mob, tool, Work::Plate(target), takes);
        }
        _ => {}
    }
}

fn make_recipe(world: &mut World, mob: Entity, stack: Entity, line: usize, at: TilePos) {
    let Some((proto, cost, made, _)) = RECIPES.get(line).copied() else {
        return;
    };
    if ONE_PER_TILE.contains(&proto) && dense_here(world, at) {
        tell(world, mob, "build-no-room");
        return;
    }
    if !use_up(world, mob, stack, cost) {
        tell(world, mob, "build-need-more");
        return;
    }
    let _ = spawn_stack(world, proto, at, made);
}

fn finish(world: &mut World, mob: Entity, job: Job) {
    match job.work {
        Work::Recipe(line) => make_recipe(world, mob, job.tool, line, job.at),
        Work::Unweld(at) => {
            dismantle_wall(world, at, false);
            tell(world, mob, "build-unwelded");
        }
        Work::TakeApart(girder) => {
            if let Some(at) = world.get::<GridPos>(girder).map(|p| p.0) {
                world.entity_mut(girder).despawn();
                let _ = spawn_stack(world, METAL, at, 1);
            }
        }
        Work::WrenchFrame(frame) => crate::frames::finish_wrench(world, frame),
        Work::TakeDownComputer(computer) => crate::frames::finish_take_down(world, computer),
        Work::Plate(target) => {
            let Some(at) = world.get::<GridPos>(target).map(|p| p.0) else {
                return;
            };
            let Some(girder) = world.get::<Girder>(target).copied() else {
                return;
            };
            match (sprite(world, job.tool).as_deref(), girder.reinforced) {
                (Some(METAL), false) if use_up(world, mob, job.tool, 2) => {
                    world.entity_mut(target).despawn();
                    set_tile(world, at, "wall", true, None, None);
                }
                (Some(PLASTEEL), false) if use_up(world, mob, job.tool, 1) => {
                    world.entity_mut(target).despawn();
                    let _ = spawn_object(world, &ObjectSpec::new(REINFORCED_GIRDER), at);
                }
                (Some(PLASTEEL), true) if use_up(world, mob, job.tool, 1) => {
                    world.entity_mut(target).despawn();
                    set_tile(world, at, "r-wall", true, None, None);
                }
                _ => {}
            }
        }
    }
}

/// Down to plating. A wall taken apart, or not blown quite to bits, leaves a girder and 2 sheets. One
/// that was devastated leaves 4 (Luna `dismantle_wall`).
pub fn dismantle_wall(world: &mut World, at: TilePos, devastated: bool) {
    let was = tile_id(world, at);
    set_tile(world, at, "plating", false, None, None);
    let reinforced = was.as_deref() == Some("r-wall");
    if devastated {
        let _ = spawn_stack(world, METAL, at, if reinforced { 2 } else { 4 });
        if reinforced {
            let _ = spawn_stack(world, PLASTEEL, at, 2);
        }
    } else {
        let proto = if reinforced { REINFORCED_GIRDER } else { GIRDER };
        let _ = spawn_object(world, &ObjectSpec::new(proto), at);
        if reinforced {
            let _ = spawn_stack(world, PLASTEEL, at, 1);
        } else {
            let _ = spawn_stack(world, METAL, at, 2);
        }
    }
}

/// Buttons pressed, and work going on.
pub fn run_jobs(world: &mut World) {
    let mut askers = world.query::<(Entity, &mut PendingBuildOps)>();
    let asked: Vec<(Entity, Vec<(Entity, usize)>)> = askers
        .iter_mut(world)
        .filter(|(_, ops)| !ops.0.is_empty())
        .map(|(e, mut ops)| (e, std::mem::take(&mut ops.0)))
        .collect();
    for (mob, ops) in asked {
        for (stack, line) in ops {
            let holds = world.get::<HeldBy>(stack).is_some_and(|h| h.0 == mob);
            let Some((_, cost, _, takes)) = RECIPES.get(line).copied() else {
                continue;
            };
            if !holds || world.get::<Stack>(stack).is_none_or(|s| s.amount < cost) {
                continue;
            }
            if takes > 0.0 {
                start(world, mob, stack, Work::Recipe(line), takes);
            } else if let Some(at) = world.get::<GridPos>(mob).map(|p| p.0) {
                make_recipe(world, mob, stack, line, at);
            }
        }
    }
    let now = world.resource::<WorldTime>().deciseconds();
    let mut busy = world.query::<(Entity, &Job, &GridPos)>();
    let all: Vec<(Entity, Job, TilePos)> = busy.iter(world).map(|(e, j, p)| (e, *j, p.0)).collect();
    for (mob, job, at) in all {
        let holding = world
            .get::<Hands>(mob)
            .is_some_and(|h| h.active_item() == Some(job.tool));
        if at != job.at || !holding {
            world.entity_mut(mob).remove::<Job>();
            tell(world, mob, "build-interrupted");
        } else if now >= job.done_at {
            world.entity_mut(mob).remove::<Job>();
            finish(world, mob, job);
        }
    }
}

/// Show each player the stack they have open.
pub fn update_build_views(
    mut viewers: Query<(Entity, Option<&PanelViewer>, &mut BuildPanelOut)>,
    stacks: Query<(&Stack, &NetId, Option<&HeldBy>, &Appearance)>,
    mut commands: Commands,
) {
    for (mob, viewer, mut out) in &mut viewers {
        let target = viewer.and_then(|v| stacks.get(v.0).ok());
        let view = target.and_then(|(stack, id, held, look)| {
            (held.is_some_and(|h| h.0 == mob) && look.sprite == METAL).then(|| BuildPanelView {
                target: *id,
                amount: stack.amount as u16,
                recipes: RECIPES
                    .iter()
                    .map(|(proto, cost, made, _)| ((*proto).to_owned(), *cost as u8, *made as u8))
                    .collect(),
            })
        });
        if view.is_none() && target.is_some() {
            commands.entity(mob).remove::<PanelViewer>();
        }
        if out.0 != view {
            out.0 = view;
        }
    }
}
