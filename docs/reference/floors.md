# Floor tiles

Sources: `luna:code/game/turf.dm` (`floor/attackby`, `to_plating`, `restore_tile`,
`levelupdate`), `luna:code/defines/turf.dm` (`intact`),
`luna:code/game/objects/items/stacks/tiles/plasteel.dm` and `stacks/stack.dm`
(`stack/attackby`, `examine`), and the `hide` procs in `luna:code/ATMOSPHERICS/pipes.dm` and
`luna:code/WorkInProgress/UnifiedCabling/Cabling.dm`.

Code: `game/classic/src/floors.rs`. Tests: `game/classic/tests/floors.rs`.

## Floors and plating

- A `floor` tile has its floor tiles in place (Luna `intact`). A `plating` tile is bare.
- Cable is laid and cut only on plating (`cables.md`).

## What floor tiles hide

- Pipes, manifolds and cables are laid under the floor tiles (Luna `level = 1`).
- On a `floor` tile they are hidden: nobody sees them and nobody can click or examine them.
- A click on a tile goes to the tile itself when only hidden things are on it.
- Luna's maps lay some pipes over the floor tiles instead (`level = 2`). Maps write those as
  `(id: "pipe", dir: North, above_floor: true)`, and they always show.
- Things show again as soon as the floor tiles are gone, and hide again when they are put
  back.

## Clicking

| Click | With | Does |
|---|---|---|
| A floor tile in reach | crowbar | pries up the floor tiles: the tile becomes plating, and one floor tile drops on it |
| Plating in reach | floor tiles | puts the floor tiles back, using one |
| A pile of floor tiles | floor tiles | the pile in hand takes all it can, up to 60; from the other hand, just one |

Nothing is said in any of these cases, as in Luna (which plays a sound).

## The item

- **Floor tiles:** size 3, force 6, up to 60 in a pile.
- Examined: "There are N floor tiles in the stack."

## Differences from Luna

- Luna plays the crowbar and tile sounds; we have no sounds yet.
- Luna's floors keep their own look, such as white or dark tiles, and get it back when
  the tiles return. All our floors look alike.
- **Not implemented:**
  - broken and burnt floors;
  - building on space and lattices;
  - reinforcing plating with rods;
  - the engine's reinforced floor (imported as plating);
  - floor tiles as a thrown weapon;
  - vents and scrubbers switching to Luna's under-floor sprites (they always show, as in
    Luna).
