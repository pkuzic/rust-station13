# Floor tiles

Sources: `luna:code/game/turf.dm` (`floor/attackby`, `to_plating`, `restore_tile`,
`levelupdate`), `luna:code/defines/turf.dm` (`intact`),
`luna:code/game/objects/items/stacks/tiles/plasteel.dm` and `stacks/stack.dm`
(`stack/attackby`, `examine`), and the `hide` procs in `luna:code/ATMOSPHERICS/pipes.dm` and
`luna:code/WorkInProgress/UnifiedCabling/Cabling.dm`; open floor and lattices from
`luna:code/defines/turf.dm` (`floor/open/attackby`) and `luna:code/game/objects/structures/lattice.dm`.

Code: `game/classic/src/floors.rs`. Tests: `game/classic/tests/floors.rs`, `game/classic/tests/lattice.rs`.

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
| Open floor in reach | rods | builds a support lattice over it, using one rod: "Constructing support lattice ...". Nothing if there is one already |
| Open floor or its lattice, in reach | floor tiles | on a lattice: the lattice goes and the tile becomes plating, using one tile; the decks' air is apart again. With no lattice: "The plating is going to need some support." |
| A lattice in reach | lit welding tool | cuts it up, leaving one rod: "Slicing lattice joints ..." |
| A pile of floor tiles | floor tiles | the pile in hand takes all it can, up to 60; from the other hand, just one |

Nothing is said in any of these cases, as in Luna (which plays a sound).

## Lattice (`lattice`)

- Anchored and not solid: people walk over it, and it doesn't stop anyone falling through the open floor
  under it, as in Luna (`multiz.md`).
- **Sprite:** Luna's `lattice15`: over open floor every side meets a floor or wall.

## The item

- **Floor tiles:** size 3, force 6, up to 60 in a pile.
- Examined: "There are N floor tiles in the stack."

## Differences from Luna

- Luna plays the crowbar and tile sounds; we play the crowbar (`sound.md`) but not the
  tile sound, nor the sound of building a lattice or laying tiles on one.
- Luna's welding tool cuts a lattice even unlit; here it must be lit.
- Luna's floors keep their own look, such as white or dark tiles, and get it back when
  the tiles return. All our floors look alike.
- **Not implemented:**
  - broken and burnt floors;
  - building lattices and floors on space (only over open floor);
  - lattices on Luna's map (it lays them over space; they aren't imported);
  - lattices broken by explosions and blobs;
  - reinforcing plating with rods;
  - the engine's reinforced floor (imported as plating);
  - floor tiles as a thrown weapon;
  - vents and scrubbers switching to Luna's under-floor sprites (they always show, as in
    Luna).
