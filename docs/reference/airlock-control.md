# Cycling airlocks

Sources: `luna:code/game/machinery/embedded_controller/airlock_controller.dm`,
`access_controller.dm`, `luna:code/game/machinery/airlock_control.dm` (`access_button`, the doors'
`secure_open`, `secure_close`); `NSV_Luna.dmm`: 6 pumped airlocks (engineering, main, toxins, EVA,
solars, mining), the access lock into virology, 7 buttons on the decks.

Code: `game/classic/src/airlock_control.rs`. Tests: `game/classic/tests/airlock_control.rs`.

A console works two doors by their map tags. It bolts them, so they can't be opened by hand or with
a card while it runs them. Every 2 seconds it takes the next step toward where it was asked to be.

- **A button** (`access-button`) says "cycle to the inside" or "to the outside".
- **Clicking a console** says "cycle to the other side".

## Pumped airlock (`airlock-console`)

It starts with the inside open and the outside bolted shut.

| Asked to go | What happens |
|---|---|
| **Out** | Shut and bolt the inside. Pump the chamber's air into the pipes until it is at 5 kPa. Open and bolt the outside. Rest the pump |
| **In** | Shut and bolt the outside. Fill the chamber from the pipes until it is at 96 kPa. Open and bolt the inside. Rest the pump |

The two doors are never both open. A pump with no air in its pipes can't fill the chamber, so the
airlock waits. The pump is a vent with a tag. Pumping out, it moves 30% of the chamber's air every 2
seconds.

## Access lock (`access-console`)

No pump. It starts with both doors locked, and only ever has one unlocked.

| Asked to go | What happens |
|---|---|
| **Out** | Lock the inside, then unlock the outside |
| **In** | Lock the outside, then unlock the inside |

## Differences from Luna

| Luna | Here |
|---|---|
| Pumped consoles ignore the buttons' `cycle_interior` and `cycle_exterior`, so as written those buttons do nothing | They work |
| A window with the doors' status and pressure | No window: a click cycles |
| A separate sensor in the chamber reports the pressure | Read at the pump's tile |
| Signals over radio, with frequencies | Direct by tag |
| Pumping out lasts as long as its volume rate allows | 30% of the chamber every 2 seconds |
| Also a "sanitize" mode for virology-style airlocks | Not used on Luna's map, not built |
