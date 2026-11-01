//! Window panes (Luna's `structure/window`): a sheet of reinforced glass along one edge of
//! its tile. It stops people and air crossing that edge, either way, and is see-through.
//! Luna's windows are grilles with panes along the edges that face out, so a window seals
//! where Luna's does and no more. Spec: `docs/reference/windows.md`.

use std::collections::HashSet;

use bevy_ecs::prelude::*;
use rs13_world::{Appearance, Dir, GridPos, NetId, ObjectSpec, TilePos};

/// The window pane prototype.
pub const WINDOW_PANE: &str = "window-pane";

/// A pane along the `dir` edge of its tile.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowPane {
    pub dir: Dir,
}

impl WindowPane {
    /// The edge it seals: its tile and the neighbour across it.
    pub fn edge(&self, at: TilePos) -> (TilePos, Dir) {
        (at, self.dir)
    }
}

/// A pane's sprite state: the edge it lies on.
pub fn pane_look(dir: Dir) -> &'static str {
    match dir {
        Dir::North => "north",
        Dir::East => "east",
        Dir::West => "west",
        _ => "south",
    }
}

/// A pane along the edge the map gives (Luna's default is south). Luna's full-tile panes
/// (diagonal directions) are not on its map; one would lie along its south edge.
pub fn spawn_window_pane(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    let dir = match spec.dir {
        Some(d @ (Dir::North | Dir::South | Dir::East | Dir::West)) => d,
        _ => Dir::South,
    };
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(WINDOW_PANE, pane_look(dir)),
            WindowPane { dir },
        ))
        .id()
}

/// The edges window panes seal, both ways round, for movement (kept up to date by
/// [`update_pane_edges`]).
#[derive(Resource, Debug, Default)]
pub struct PaneEdges(pub HashSet<(TilePos, TilePos)>);

/// Work the sealed edges out again when a pane appears, moves or goes.
pub fn update_pane_edges(
    mut edges: ResMut<PaneEdges>,
    panes: Query<(&GridPos, &WindowPane)>,
    changed: Query<(), (With<WindowPane>, Or<(Added<WindowPane>, Changed<GridPos>)>)>,
    mut removed: RemovedComponents<WindowPane>,
) {
    let gone = removed.read().count() > 0;
    if changed.is_empty() && !gone {
        return;
    }
    edges.0 = pane_edges(panes.iter());
}

/// Every edge a pane seals, both ways round, as movement checks them
/// (`firedoors::crosses_closed_edge`).
pub fn pane_edges<'a>(
    panes: impl Iterator<Item = (&'a GridPos, &'a WindowPane)>,
) -> HashSet<(TilePos, TilePos)> {
    let mut edges = HashSet::new();
    for (pos, pane) in panes {
        let (at, dir) = pane.edge(pos.0);
        edges.insert((at, at.step(dir)));
        edges.insert((at.step(dir), at));
    }
    edges
}
