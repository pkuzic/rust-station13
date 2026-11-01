# Emergency floodlights

Sources:
- `luna:code/game/machinery/floodlight.dm` (`update_icon`, `process`, `attack_hand`,
  `attackby`, `New`);
- `NSV_Luna.dmm`: two in the asteroid base's storage.

Code: `game/classic/src/floodlights.rs`. Tests: `game/classic/tests/floodlights.rs`.

## The floodlight (`floodlight`)

A lamp on a stand, dense, that runs from its own power cell, not the area's power.

- **Cell:** comes with a full 2 500-unit cell (2.5 MJ, `cells.md`).
- **Light:** while lit, 10 tiles of white light (Luna `ul_SetLuminosity(10)`), brighter
  and further than a tube (8).
- **Drain:** every machine cycle (2 s) a lit floodlight takes 1 unit (1 kJ) from its cell,
  so a full cell lasts about 83 minutes. When the cell is flat or taken out, it goes out.
- **Sprite:** `flood` + `o` with the panel off + `b` with the cell showing + `0` + `1`
  while lit: `flood00`, `flood01`, `floodo00`, `floodob00`, `floodob01`.

## Using it

| With | Does | Told |
|---|---|---|
| An empty hand, panel off, cell in | Takes the cell out into the hand; the light goes out | "You remove the power cell." |
| An empty hand, lit | Turns it off | "You turn off the light." |
| An empty hand, dark, charged cell | Turns it on | "You turn on the light." |
| An empty hand, dark, no charge | Nothing | |
| A screwdriver, panel on | Undoes or does up the panel's screws | "You unscrew the battery panel." / "You screw the battery panel in place." |
| A crowbar, screws out | Takes the panel off or puts it back | "You remove the battery panel." / "You crowbar the battery panel in place." |
| A cell, panel off | Puts it in, or not if there is one | "You insert the power cell." / "There is a power cell already installed." |

## Differences from Luna

| Luna | Here |
|---|---|
| Taking the cell out puts it in the hand whatever the hand holds | Only into an empty hand |
