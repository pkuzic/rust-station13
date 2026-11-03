# Brig door timers

Sources: `luna:code/game/machinery/doors/brigdoors.dm` (`door_timer`: `process`, `timer_start`,
`timer_end`, `timeleft`, `attack_hand`, `Topic`); `NSV_Luna.dmm`: 5 cells, each with a timer, two
window doors and a flasher.

Code: `game/classic/src/brig.rs`, `apps/client/src/brig_panel.rs`. Tests: `game/classic/tests/brig.rs`.

A timer on the wall shares an id with its cell's window doors and flasher. Clicking one opens its
window, which closes if you walk away or the power goes. Pressing anything in it wants a card with
brig access, in the active hand or worn.

- **The time:** up or down by a second or a minute, from 0 up to 50 minutes.
- **Activate timer and close door:** the count begins and the doors shut. When it reaches zero, the
  doors open again and the time is cleared.
- **Stop timer and open door:** the doors open, and what was left stays set.
- **Activate flash:** fires the cell's flasher (`flashes.md`), if it isn't resting.

Every 2 seconds, each timer with power puts its doors right: shut while running, and otherwise open.
So a cell with no sentence stands open, unless the timer has no power.

## Differences from Luna

| Luna | Here |
|---|---|
| The time left is shown on the timer's face | Only in its window |
| It also locks the cell's brig locker | Those lockers aren't on the map as linked to a cell |
