# Construction

Sources: `luna:code/game/objects/items/stacks/sheets/sheet_types.dm`, `stack.dm`, `rods.dm`, `glass.dm`,
`luna:code/game/objects/structures/girders.dm`, `luna:code/game/objects/grille.dm`, `luna:code/game/turf.dm` (`dismantle_wall`).
Code: `game/classic/src/construction.rs`, `apps/client/src/build_panel.rs`. Tests: `game/classic/tests/construction.rs`.

Metal sheets used in hand open a list of what they make, on your own tile:

| Makes | Sheets | Takes |
|---|---|---|
| Stool, chair | 1 | |
| Bed, table parts | 2 | |
| Rack parts | 1 | |
| Closet | 2 | 1.5 s |
| 4 floor tiles, 2 rods | 1 | |
| Plasteel | 2 | |
| Girder | 2 | 5 s |

Anything that takes time needs you to stay where you are holding the same thing, or you stop.

- **Girder:** dense. 2 metal sheets on it, 4 seconds: a wall. A wrench, 4 seconds: taken apart, for a sheet. A plasteel sheet, 6
  seconds: reinforced. Another, 5 seconds: a reinforced wall.
- **Wall:** a lit welding tool with fuel, 10 seconds: taken apart, leaving a girder and 2 sheets. A blast does the same, or leaves 4
  sheets and no girder.
- **Reinforced wall:** comes apart in Luna's order: wirecutters, a screwdriver (4 s), a lit welding tool (6 s), a crowbar (10 s), a
  wrench (4 s), the welding tool (10 s), a crowbar (10 s). It gives rods at the first and sixth, and ends as a reinforced girder and
  a plasteel sheet. A metal sheet (10 s) mends it at any point.
- **Rods** used in hand: 2 make a grille on your tile. Wirecutters on it: gone, for 2 rods. A grille lets air through.
- **Glass** used in hand: a pane on your tile, on the side you face or the next free one. A wrench on it: back to a sheet.

## Computers

Sources: `luna:code/game/machinery/computer/buildandrepair.dm`, `computer.dm`. Code: `game/classic/src/frames.rs`. Tests:
`game/classic/tests/frames.rs`.

A computer frame is 5 metal sheets and 2.5 seconds, and is put together in order. Each step undoes:

| Step | With | Undo with |
|---|---|---|
| Fastened down (2 s) | Wrench | Wrench. A loose frame comes apart for its metal with a lit welding tool |
| Board put in | A circuit board | Crowbar |
| Board fixed | Screwdriver | Screwdriver |
| Wired | 5 lengths of cable | Wirecutters |
| Glass | 2 sheets of glass | Crowbar |
| Working | Screwdriver: the computer the board is for | Screwdriver (2 s): back to the glass step, board inside |

Boards exist for the security console, medical and security records, the ID computer, communications, arcade, cloning and
the power monitor. Nothing sells or makes spare boards yet, so they come from taking a computer down.

## Differences from Luna

| Luna | Here |
|---|---|
| A reinforced wall looks different at each step | It looks whole until it comes down |
| Panes are fastened and unfastened with a screwdriver, reinforced glass has more steps, and a full-tile window is two panes | One step, plain glass, single panes |
| A grille is an object, can be unfastened, and windows go on it | It's a tile (`windows.md`) |
| Girders can be dislodged, to make false walls that open | Not built |
| Metal also makes swivel and comfy chairs, canisters, turret and light frames, grenade casings, iron doors | Not built |
| Stacks build more than one at a time | One at a time |
| Machine frames and their parts | Waits for research (3.11), which is what makes the boards |
| A broken computer's glass falls out when it's taken down | Computers don't break |
| A board keeps a console's settings | Not kept |
| Rods can be welded back into metal | Not built |
