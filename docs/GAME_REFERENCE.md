# Game reference: NSV Luna

## The reference build

- Repository: <https://github.com/SS13/luna>
- Commit: `e13cf12` (2013-12-16), branch `master`
- A Russian (ss13.ru) Baystation12 fork. Environment file `bs12.dme`, station map
  `maps/NSV_Luna.dmm`.
- Contributors keep a local clone **outside** this repository (for example
  `../luna`). Never add Luna files to this repository except assets imported under
  `client-assets/` with attribution.

OpenDream and RobustToolbox may be consulted for how others have solved engine problems.

## Clean-room rule

Luna's code has no licence file and descends from GPL/AGPL code. We use it as a
**behaviour specification only**:

- **Allowed:** reading Luna to learn what happens, and writing that down in our own words.
  Copying numeric constants and formulas, which are facts about the game.
- **Not allowed:** translating DM procs line by line into Rust, or copying code comments
  or text.
- Player-facing text is written fresh in our locales. Luna's Russian strings may inform
  terminology but are not copied wholesale.

Cite Luna as `luna:<path>` (for example `luna:code/FEA/FEA_gas_mixture.dm`) in
issues, specs and tests.

## Where things live in Luna

| System | Luna path |
|---|---|
| Global constants | `code/setup.dm` |
| Atmos (FEA) | `code/FEA/` (gas mixture, air groups, turf tiles, fire) |
| Pipes | `code/ATMOSPHERICS/` |
| Doors and airlocks | `code/game/machinery/doors/` (`airlock.dm`, `door.dm`, `firedoor.dm`) |
| Airlock control | `code/game/machinery/airlock_control.dm`, `embedded_controller/` |
| Air alarms | `code/game/machinery/alarm.dm` |
| Power | `code/modules/power/` (`apc.dm`, `smes.dm`, `cell.dm`, `lighting.dm`) |
| Jobs | `code/game/jobs/` |
| Mobs | `code/modules/mob/` |
| Master controller | `code/game/master_controller.dm` |

## Notable reference facts

Record findings here as they are verified, with the Luna path.

- `R_IDEAL_GAS_EQUATION` is `8.31 * 2` kPa·L/(K·mol), double the physical value.
  This affects every pressure calculation (`luna:code/setup.dm`).
- `ONE_ATMOSPHERE` 101.325 kPa, `T20C` 293.15 K, `CELL_VOLUME` 2500 L per tile
  (`luna:code/setup.dm`).

## Specs written so far

- [Movement](reference/movement.md)
- [Doors and airlocks](reference/airlock.md)
- [Hands, items and clicking](reference/hands.md)
- [Speech and character names](reference/speech.md)
- [Access and ID cards](reference/access.md)
- [Health, attacks, unconsciousness and death](reference/health.md)

## Writing a behaviour spec

Before implementing a system, write a short spec under `docs/reference/<system>.md`:
inputs, states, timings, constants, edge cases and known Luna quirks. Mark each quirk as
**keep** (players rely on it) or **fix** (an unintended bug). Tests cite the spec.
