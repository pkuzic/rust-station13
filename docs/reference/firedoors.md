# Firedoors

Sources:
- `luna:code/game/machinery/doors/firedoor.dm` (`border_only`, `attackby`, `process`);
- `luna:code/defines/obj/door.dm` (`/obj/machinery/door/firedoor`);
- `luna:code/game/machinery/doors/door.dm` (`open`, `close`);
- `luna:code/game/area/areas.dm` (`air_doors_close`, `air_doors_open`).

Code: `game/classic/src/firedoors.rs`; edge barriers in the air solver are
`engine/simulation/src/atmos/grid.rs` (`add_edge`, `set_edge_open`). Tests:
`game/classic/tests/firedoors.rs`, `engine/simulation/src/atmos/tests.rs`.

## The firedoor

- **What it seals:** a border firedoor (`firedoor`) seals one edge of its tile, the side it
  faces (Luna's default is south).
- **While closed:** no air crosses that edge, and nobody walks across it. A diagonal step is
  refused only when both ways round it cross a closed edge.
- **What it leaves alone:** walking along the edge, and onto or off its tile any other way.
- **Starting state:** firedoors start open.
- **Sprite:** `door0` open, `door1` closed; `doorc0` while opening and `doorc1` while
  closing. The facing is added, e.g. `door1-south`.
- **On the map:** none of Luna's firedoors sets a direction, so every one on the imported
  map seals the south edge of its tile.

## The air

- **Zones:** the air solver keeps the two sides of a firedoor in separate zones, as it does
  for door tiles.
- **Open:** the edge is a doorway (2 m²), and gas flows through it as through an open
  airlock, wind included.
- **Closed:** the edge holds any pressure difference. A closed edge also stops gas leaking
  into space through it.
- **Wind:** space wind doesn't push people or items across a closed edge.

## Working it

| With | The firedoor | Does |
|---|---|---|
| crowbar | open, not welded, not moving | closes: it seals at once, with a 1.5 s animation |
| crowbar | closed, not welded, not moving | opens after 1.5 s |
| lit welding tool | closed, not moving | welds it, or unwelds it |
| anything else | | nothing. Bumping into it does nothing either. |

- **Messages:** Luna gives none, so neither do we.
- **Welded firedoors:** they look the same as unwelded ones.

## Air alarms

- **Sealing:** when an air alarm seals its area (`air-alarm.md`), every firedoor in or next
  to the area closes.
- **Unsealing:** when no seal covers a firedoor any more, it opens again.
- **Timing:** the alarm's doors seal at once and open after 0.6 s (Luna `door/open`).
- **Exceptions:**
  - power doesn't matter;
  - a welded firedoor ignores the alarm;
  - a firedoor that is moving finishes first, then does what the alarm asked.

## Fire alarms

A fire alarm that goes off closes every firedoor in or next to its area, as an air alarm
seal does, and they open again when it is reset, unless a seal or another alarm still
covers them (`fire-alarm.md`).

## Differences from Luna

- **Both ways:** Luna's border firedoor only stops people entering its tile from the side
  it faces. Leaving across that edge isn't checked, because it has no `CheckExit`. Ours
  blocks the edge both ways, as the air does.
- **Welding fuel:** welding one burns one unit of fuel, as any use of a lit welding tool
  does (`welding.md`). Luna takes two more.
- **Not implemented:**
  - Luna's full-tile firedoors (`/obj/machinery/door/firedoor`);
  - crushing people as a firedoor closes;
  - the "Emergency Shutter" name some of them have;
  - explosions;
  - sounds.
