# Disposals

Sources:
- `luna:code/WorkInProgress/recycling/disposal.dm` (`machinery/disposal`: `process`,
  `attackby`, `flush`, `update`; `disposalholder`: `move`, `init`; `disposalpipe` and its
  subtypes: `nextdir`, `transfer`, `expel`; `disposaloutlet/expel`);
- `NSV_Luna.dmm`: 57 bins, 5 outlets and about 750 pipes on decks 1–4.

Code: `game/classic/src/disposals.rs`. Tests: `game/classic/src/disposals.rs` (unit),
`game/classic/tests/disposals.rs`, `game/classic/tests/luna_station.rs`.

## Bin (`disposal-bin`)

Dense, on a trunk pipe. It holds what is put in, and an air reservoir of 312.5 L.

- **Putting things in:** click it with something in the hand: it goes in. "You place the
  wrench into the disposal unit."; everyone else sees "Name places the wrench into the
  disposal unit."
- **The handle:** an empty hand engages or disengages it (Luna's window has the handle;
  the window isn't built yet). It also engages by itself five machine cycles (10 s) after
  the last thing goes in.
- **The pump:** each machine cycle, while powered, it draws air from the room, closing a
  tenth of the gap to 2.1 atm; at 2 atm it is ready. Power: 100 W, 600 W while pumping.
  From empty it takes about a minute.
- **Flushing:** with the handle engaged and at least 2 atm, it flushes, with or without
  power: Luna's flush animation and sound, and 1.5 s later everything inside, with its air,
  leaves down the trunk as one bundle. Then the handle disengages and the pump starts
  again. With no trunk under it, everything spills out on its tile instead.
- **Sprite:** `disposal`, with `dispover-handle` while the handle is engaged and, while
  powered, `dispover-full` with anything inside and `dispover-charge` / `dispover-ready`
  for the pump; `disposal-flush` while flushing.

## Pipes (`disposal-pipe`)

Under the floor (hidden by intact floor tiles), each with Luna's shape (its icon state) and
direction:

| Shape | Joins |
|---|---|
| `pipe-s` | its direction and the opposite |
| `pipe-c` | its direction and a quarter turn clockwise |
| `pipe-j1` | its direction, clockwise and the opposite |
| `pipe-j2` | its direction, counter-clockwise and the opposite |
| `pipe-y` | its direction and both quarter turns |
| `pipe-t` | its direction only: the trunk under a bin or outlet |
| `pipe-u`, `pipe-d` | its direction, and up or down to the same spot on the next deck |
| `pipe-b` | nothing (broken) |

- **Travel:** a bundle moves a tile every 0.1 s, onto the pipe on the next tile that joins
  back the way it came. Out of a pipe it leaves the way it didn't come in; a junction sends
  everything from a branch out its main way, and what comes in by the main way down either
  branch, half the time each; a deck crossing turns flat to vertical and back.
- **Arriving at a trunk** under an outlet hands it over; under nothing, it spills there.
- **A gap:** with no pipe that joins on the next tile, it spills there: the things land on
  that tile and the air goes into the room, with a hiss. Into a wall, it stays stuck in the
  last pipe. After 1 000 steps a bundle stops.

## Outlet (`disposal-outlet`)

Dense, on a trunk, facing a way. When a bundle arrives it opens (`outlet-open`) with Luna's
warning buzzer; 2 s later it hisses and throws everything out the way it faces, up to three
tiles, as far as nothing stops it. The air is lost.

## Differences from Luna

| Luna | Here |
|---|---|
| People can climb in, be stuffed in, or be put in with a grab; they're knocked about at bends and junctions | Things only |
| An empty hand opens the bin's window (handle, eject, pump) | It works the handle |
| Delivery chutes swallow what bumps them | Not built |
| Bins, outlets and pipes come apart with a screwdriver and welder | Not built |
| A spill throws things up to 10 tiles, or scatters them up to 5, and pops the floor tile | They land on the tile |
| Three Y junctions on Luna's map have no direction | They face south |
