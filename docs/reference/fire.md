# Reference spec: fire

## What Luna does

Sources: `luna:code/FEA/FEA_fire.dm` (`hotspot_expose`, `obj/fire`, `obj/hotspot`),
`luna:code/FEA/FEA_gas_mixture.dm` (`react`), `luna:code/setup.dm` (constants),
`luna:code/game/objects/items/weapons/tools.dm` (welding tool `process`).

- **Ignition:** a spark or flame calls `hotspot_expose(temperature, volume)` on its tile. A lit welding tool does so every process with `SPARK_TEMP` (3 500 K), in hand or on the floor. Above `PLASMA_MINIMUM_BURN_TEMPERATURE` (250 °C), with at least 0.5 mol of plasma and of oxygen on the tile, it spawns an `obj/fire`. A floor-mounted igniter does the same at 6 000 K while it is on (`igniters.md`).
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
| Where it burns | a whole zone (room) at once: hot air carried through doors and breaches spreads it | **simplified**: zone-level until the local cells of `ATMOSPHERICS.md` (ADR 0009) |
| Ignition | a flame touches the air: 125 L of the zone at the flame's temperature burns for a second and mixes back, heating the room until it burns by itself | keep (Luna's hotspot) |
| Sources | lit welding tools, every machine cycle, in hand or on the floor, at 3 500 K | keep |
| Looks | every tile of a burning zone shows fire (`fire.dmi` 2, or 3 when fierce) and gives off warm light | keep |
| Effects | the hot air does the rest: heat damage to people (`environment.md`), fire alarms above 200 °C (`fire-alarm.md`) | keep |
| Putting it out | burn the fuel or the oxygen away, or vent the room | extinguishers aren't on the map yet |
| Heat loss | the air trades heat with its zone's walls, floor and ceiling at 5 W/(m²·K), toward 20 °C (`AtmosGrid::exchange_heat`): a 9-tile room has a time constant of about 80 s, so a room at 5 000 K is back near room temperature in five or six minutes | **ours**: walls are a heat sink at 20 °C until materials get their own temperature (P1) |
| Other sparks | none yet | electrical sparks, igniters, lighters |
