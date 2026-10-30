# Cables and cable coils

Sources: `luna:code/WorkInProgress/UnifiedCabling/Cabling.dm` (`/obj/cabling`: `New`,
`AllConnections`, `attackby`, `DropCablePieces`), `CablingCoil.dm` (`/obj/item/weapon/cable_coil`:
`LayOnTurf`, `JoinCable`, `afterattack`, `attackby`, `use`, `examine`),
`luna:code/modules/power/Cabling.dm` and `luna:code/game/turf.dm` (`floor/attackby`).

Code: `game/classic/src/cables.rs`, networks in `game/classic/src/power.rs`.
Tests: `game/classic/tests/cables.rs`.

## Cables

**Ends:**
- A cable has two ends. Each end points at one of the eight neighbouring tiles, or is a
  **knot** in the middle of its own tile.
- The sprite is named by Luna's direction numbers, knot first: `0-4` is a knot pointing east,
  `1-2` runs north–south, `1-4` bends from north to east.
- Maps write the ends as `(id: "cable", ends: [North, South])`. A single direction means a
  knot plus that direction. A cable without ends is Luna's default, a knot pointing north.

**Networks** (Luna's unified cable network):
- An end joins every cable on the tile it points to whose own end points back, and every
  power machine (SMES, APC) on that tile.
- A knot joins the other knots and the machines on its own tile.
- Machines never pass power from one cable to another, and each machine belongs to at most
  one network.
- Networks are rebuilt whenever a cable or machine appears, goes or changes.

## Cable coils

**The item:**
- A coil holds up to 30 lengths, and a new one is full. Its size is 2.
- Electrical toolboxes start with three coils; utility belts take coils and the loaded belt
  starts with one.
- Its sprite shows three loops, or a piece of two or one length.
- **Examine:**
  - 1 length: "A short piece of power cable."
  - 2 lengths: "A piece of power cable."
  - more: "A coil of power cable. There are N lengths of cable in the coil."
- The last length used up takes the coil out of the hand.

**Floor tiles:** cable is laid, extended and cut only on bare plating. On a floor tile you
see "You can't lay cable there unless the floor tiles are removed." Cables under floor tiles
can't be reached at all.

## Clicking

| Click | With | Does |
|---|---|---|
| A bare tile in reach | coil | lays a knot pointing at you; on your own tile, the way you face. Uses one length. |
| A tile out of reach | coil | "You can't lay cable at a place that far away." |
| A cable with an end pointing at you | coil | lays a knot on your tile, pointing back at it. Uses one length. |
| A knot not pointing at you | coil | turns it into a cable from its end to you. Uses one length. |
| A power machine on bare plating | coil | lays a knot under it, pointing at you. Uses one length. |
| A cable | wirecutters | cuts it: everyone around sees "X disconnects the power cable.", and a coil of 2 lengths (1 for a knot) drops on its tile |
| A coil of 3 or more | wirecutters | "You cut a piece off the cable coil."; a piece of one length drops at your feet |
| Another coil | coil | joins them: the one in your hand takes all it can, up to 30 |

**Refusals:**
- If an end would repeat one already on that tile: "There's already a cable at that position."
- Joining coils when yours is full: "The coil is too long, you cannot add any more cable to it."
- When only part fits: "You transfer N lengths of cable from one coil to the other."
- Clicking a cable on your own tile, or a cable that is neither a knot nor pointing at you,
  does nothing.

**Clicking a tile:** a click on a tile with nothing on it reaches the tile itself. A click
on a tile that has a cable or machine reaches that instead, as in Luna.

**Facing:** the server remembers the way each mob last tried to move (south until then).

## Differences from Luna

- **Extending a cable:** Luna puts the new piece on the old cable's tile, pointing away from
  you, and checks your tile for duplicates. We put it on your tile, pointing back at the
  cable, as intended.
- **Hand-laid cables:** in Luna they join their network as if they pointed north. Ours join
  by their real ends.
- **Electrical toolbox:** Luna swaps its third coil for insulated gloves one time in ten; ours
  always has three coils.
- **Not implemented:**
  - lifting floor tiles with a crowbar, and hiding cables under them;
  - cable restraints;
  - mending robotic limbs;
  - other cable colours;
  - multitools and other devices on cables;
  - explosions;
  - ends that lead up or down to other decks (the map import drops them).
- Luna's cables never shock, and neither do ours.
