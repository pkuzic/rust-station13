# Shield generator

Sources: `luna:code/WorkInProgress/ShieldGenerator/` (`Emitters.dm`, `Shield.dm`,
`ShieldCapacitor.dm`, `Generator.dm`, `ShieldNetwork.dm`), `luna:code/game/machinery/airlock_control.dm`
(`shieldsbutton`); `NSV_Luna.dmm`: 71 plates across the decks, 3 capacitors with 3 converters in
the shield room on deck 2, the button on the bridge.

Code: `game/classic/src/shields.rs`. Tests: `game/classic/tests/shields.rs`.

## Parts

| Part | What it does |
|---|---|
| **Emitter plate** (`shield-plate`) | Lies under the floor. Shows `plate` while the shields are up, `plate-p` otherwise. |
| **Field** (`shield`) | One tile on every space tile within 10 tiles of a plate on its deck that touches anything but space, diagonals included. Down, nobody is sent it and it does nothing. Up, it sparkles, stops people and things, and seals air against any open tile next to it. |
| **Capacitor** (`shield-capacitor`) | Dense. Holds up to 10,000,000 units and starts with 1,000,000. Lights show charge in fifths. `cap-o` shows while its converter isn't running. Examining tells its charge. |
| **Energy converter** (`shield-converter`) | Dense. Charges the capacitor directly west of it from the station's equipment power. Clicking steps it through off, manual and automatic. It starts on manual. |
| **Button** (`shields-button`) | Raises and lowers the shields. It needs no access. It won't raise them unless at least 3 capacitors hold charge. |

## Every 2 seconds

1. A running converter adds `rate^2.15` units to its capacitor, unless that is full, and draws
   as many watts, at most 100 kW. It adds nothing without power.
   - **Manual:** a rate of 60, about 6,600 units.
   - **Automatic:** a rate climbing by 50 per cycle up to the maximum, 500. Below full, Luna's
     target always asks for more than the maximum, so in practice it runs flat out.
2. While the shields are up, each capacitor loses 80,000 units.
3. The shields are up while they are switched on and 3 capacitors with converters hold charge.
   Once one runs out they drop, and come back by themselves when it has charge again.

So on manual, the shields last about half a minute from the starting charge. To keep them up,
someone has to set the converters to automatic in the shield room first.

## Differences from Luna

| Luna | Here |
|---|---|
| The converter's window: rates, target charge, remote mode | Clicking only changes the mode. Manual stays at 60 and the target at 75. Remote mode has nothing to control it in Luna either |
| The button says nothing | It tells you what happened |
| The button keeps its own state, separate from whether the shields started | One state for the whole station |
| Explosions cost the field charge | There are no explosions yet |
| New holes in the hull get a field tile | Hull can't be removed yet, so the field is laid once, at the start |
| Floors made into open space get a field tile, a Luna bug | Not copied |
