# Reference spec: power cells and cell chargers

## What Luna does

Sources: `luna:code/modules/power/cell.dm`, `luna:code/game/machinery/cell_charger.dm`.

### Power cells
- **Capacity:** `item/weapon/cell` holds 2 500 units and `cell/high` 10 000. A mapped cell's `charge` is a percentage of that, 100 by default; `cell/high/empty` starts at 0.
- **Meter overlay:** `cell-o2` at 99.5% or more, `cell-o1` above zero, none when empty.
- **Examine:** the power rating and the charge meter's percentage.
- **Rigging:** a syringe of plasma rigs a cell to explode on use. We have no syringes or explosions.

### Cell charger
- **Mounting:** anchored. A wrench detaches or attaches it, but only when it is empty.
- **Putting a cell in:** only into a secured charger, one at a time. In an area whose equipment channel is off, "blinks red" and refuses. Everyone nearby sees it.
- **Hand:** takes the cell out into that hand.
- **Charging:** each cycle, a secured charger with a cell, in a powered area, uses 200 from the equipment channel and gives the cell 175 units. An empty cell fills in about 15 cycles.
- **Sprite:** `ccharger0` empty, `ccharger1` with a cell. While powered, an overlay `ccharger-o(round(percent × 4 / 99))`, rounded down: 0–4. The icon file misspells level 2 as `ccharge-o2`, so Luna never shows it.
- **Examine:** whether there is a cell, and its charge in units.

## What we build (`game/classic/src/cells.rs`)

| Item | Ours | From Luna |
|---|---|---|
| Units | 1 unit = 1 kJ, as the APC's cell (`power.md`): 2.5 MJ and 10 MJ | keep |
| Cells | `power-cell`, `high-power-cell`, full | keep |
| Meter overlay and examine | as above | keep |
| Charger: wrench, one cell, refusing without equipment power, empty hand takes the cell | as above | keep |
| Charging | 175 kJ into the cell per machine cycle, drawing 200 W on the equipment channel | keep Luna's numbers. Like Luna, it makes more energy than it uses. |
| Charge overlay | levels 0–4, level 2 shown | **fixed**: the misspelt icon state |
| Rigged cells, EMP | none | no syringes, explosions or EMPs |
| Putting cells in APCs | none | APC covers are locked; that needs the APC's panel |
