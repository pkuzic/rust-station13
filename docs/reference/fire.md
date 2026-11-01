# Reference spec: fire

## What Luna does

Sources: `luna:code/FEA/FEA_fire.dm` (`hotspot_expose`, `obj/fire`, `obj/hotspot`),
`luna:code/FEA/FEA_gas_mixture.dm` (`react`), `luna:code/setup.dm` (constants),
`luna:code/game/objects/items/weapons/tools.dm` (welding tool `process`).

- **Ignition:** a spark or flame calls `hotspot_expose(temperature, volume)` on its tile. A lit welding tool does so every process with `SPARK_TEMP` (3 500 K), in hand or on the floor. Above `PLASMA_MINIMUM_BURN_TEMPERATURE` (250 °C), with at least 0.5 mol of plasma and of oxygen on the tile, it spawns an `obj/fire`, which adds the flame's temperature to the tile's air at once (`obj/fire/New`): from then on the hot air reacts by itself. Igniters and sparkers do the same at 6 000 K (`igniters.md`).
- **The compiled fire (`obj/fire`) does not work:**
  - `burn(0.5, 0.5)` takes `round(0.5)` of each gas, which BYOND rounds down to zero, so it burns nothing and adds no heat (only a little carbon dioxide from nothing);
  - it dies unless the tile is already above 100 °C (`FIRE_MINIMUM_TEMPERATURE_TO_EXIST`), so at room temperature it goes out at once;
  - where the air is hot, it spreads to half the neighbouring tiles with plasma and oxygen each process, exposes everything on its tile to the heat, and is opaque.
- **The reaction (`react`), used by the uncompiled `obj/hotspot`:**
  - **Rate:** above 250 °C, `scale = (T − 250 °C) / (1370 °C − 250 °C)`, at most 1. Plasma burns at `plasma × scale / 4`, or `scale × (oxygen / 10) / 4` when there is less than ten times as much oxygen as plasma.
  - **Products:** each mole burnt uses `1.4 − scale` moles of oxygen and makes a mole of carbon dioxide and 3 MJ of heat (`FIRE_PLASMA_ENERGY_RELEASED`).
  - **Hotspot:** heats 125 L of the tile's air to its own temperature, reacts it, and merges it back.

## What we build (`rs13_simulation::atmos::fire`, `game/classic/src/fires.rs`)

The design asks for real combustion (`docs/ATMOSPHERICS.md`, ADR 0006), so we use Luna's
reaction rather than its broken fire object:

| Item | Ours | From Luna |
|---|---|---|
| Reaction | `react`'s formula, as a rate per second, on each zone's air every step: above 250 °C, plasma and oxygen become carbon dioxide and 3 MJ per mole | **changed**: the reaction, not `obj/fire` |
| Where it burns | tile by tile: each burning tile is a fire cell with its own air, trading gas and heat with its neighbours and its room; a burning cell lights each neighbouring tile that can catch after half a second, and that tile's air warms by 100 °C as Luna's new `obj/fire` warms its tile, so fire crosses a room of any size at about two tiles a second while it has fuel. A room hot enough burns as a whole too, and hot air carried through doors and breaches spreads it (ADR 0010, `rs13_simulation::atmos::grid::fire_cells`) | keep: Luna's `obj/fire` spreads to neighbouring tiles with plasma and oxygen each process, adding its `FIRE_MINIMUM_TEMPERATURE_TO_EXIST` to each |
| Ignition | a flame hotter than 250 °C touches a tile whose air holds more than 0.5 mol of plasma and of oxygen, and it isn't burning yet: the tile becomes a fire cell and its air warms by the flame's whole temperature, then burns by itself. In any size of room | keep (Luna's `obj/fire/New`) |
| Sources | lit welding tools, every machine cycle, in hand or on the floor, at 3 500 K; igniters and sparkers at 6 000 K (`igniters.md`) | keep |
| Looks | every tile of a burning zone shows fire (`fire.dmi` 2, or 3 when fierce) and gives off warm light | keep |
| Effects | the hot air does the rest: heat damage to people (`environment.md`), fire alarms above 200 °C (`fire-alarm.md`) | keep |
| Putting it out | burn the fuel or the oxygen away, vent the room, or spray it with a fire extinguisher (`extinguishers.md`) | keep |
| Heat loss | the air trades heat with its zone's walls, floor and ceiling at 5 W/(m²·K), toward 20 °C (`AtmosGrid::exchange_heat`): a 9-tile room has a time constant of about 80 s, so a room at 5 000 K is back near room temperature in five or six minutes | **ours**: walls are a heat sink at 20 °C until materials get their own temperature (P1) |
| Other sparks | none yet | electrical sparks, lighters |

## Fire cells

ADR 0010; `engine/simulation/src/atmos/grid/fire_cells.rs`.

- **A cell** holds one tile's air, taken out of its room, whose volume shrinks by a tile:
  gas is conserved exactly. What you see on the tile (pressure, gases) is the cell's air.
- **Each step** it burns with the reaction above, and trades 0.4 of the difference in gas
  and heat per second with each neighbouring cell and each side facing its room's tiles;
  not across walls, closed doors, closed firedoors or window panes.
- **Spreading:** after burning half a second, a cell hotter than 250 °C lights each
  neighbouring tile of open floor whose air holds more than 0.5 mol of plasma and of oxygen.
- **Settling:** once a cell hasn't burnt for 2 s and is within 30 K of its room (or after
  30 s), its air goes back into the room. Every cell of a room venting to space goes back at
  once, and vents with it: the rush of air out takes the fire's air too. When a room is rebuilt (a wall breached, a floor
  tile changed), every cell goes back first.
- **Too little fuel:** a burning tile has to make heat faster than its neighbours carry it
  off; in a thin plasma haze it can't, and the fire goes out.
