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
- **Rods** used in hand: 2 make a grille on your tile. Wirecutters on it: gone, for 2 rods. A grille lets air through.
- **Glass** used in hand: a pane on your tile, on the side you face or the next free one. A wrench on it: back to a sheet.

## Differences from Luna

| Luna | Here |
|---|---|
| Reinforced walls come apart in several steps, with wirecutters, a screwdriver, a welder, a crowbar and a wrench | Can't be taken apart yet |
| Panes are fastened and unfastened with a screwdriver, reinforced glass has more steps, and a full-tile window is two panes | One step, plain glass, single panes |
| A grille is an object, can be unfastened, and windows go on it | It's a tile (`windows.md`) |
| Girders can be dislodged, to make false walls that open | Not built |
| Metal also makes swivel and comfy chairs, canisters, turret and light frames, grenade casings, iron doors | Not built |
| Stacks build more than one at a time | One at a time |
| Computer and machine frames | Next (`LUNA_PARITY.md`) |
| Rods can be welded back into metal | Not built |
