# Windows and grilles

Sources:
- `luna:code/game/objects/window.dm` (`CanPass`, `CheckExit`, `update_nearby_tiles`);
- `luna:code/defines/obj/window.dm` (`/obj/structure/window`, `window_pod`);
- `luna:code/game/objects/grille.dm`;
- `NSV_Luna.dmm`: about 820 reinforced panes, as many facing each way, and no full-tile
  ones.

Code: `game/classic/src/windows.rs`; grille and window tiles in `rs13_world::TileDef`.
Tests: `game/classic/tests/windows.rs`, `game/classic/tests/luna_map.rs`.

## Window pane (`window-pane`)

Luna's windows are panes of glass along one edge of a tile: a pane facing north lies along
the tile's north edge. A window in a wall is usually a grille with panes on its edges.

- **Map vars:** `dir`, the edge it lies on (Luna's default is south).
- **People and things:** can't cross its edge, either way (Luna `CanPass`, `CheckExit`). A
  diagonal step is refused only when both ways round it are blocked. They can stand on its
  tile.
- **Air:** can't cross its edge either; the edge is a sealed barrier between zones
  (`AtmosGrid::add_edge`, never opened).
- **Sight and light:** pass through.
- **Sprite:** Luna's reinforced window (`rwindow`) facing its edge.

## Grille and window tiles

- **`grille`:** a grille with no glass. Solid to people, see-through, and air goes through
  it (`airtight: false`), as Luna's grilles.
- **`window`:** for hand-made maps: a grille with glass filling the tile. Solid, see-through,
  airtight; drawn as the grille behind Luna's full-tile window glass.

## Differences from Luna

| Luna | Here |
|---|---|
| Panes can be broken, unfastened, turned and carried | They stay as mapped |
| Full-tile panes (diagonal directions) block their whole tile | None on the map; one would lie along its south edge |
