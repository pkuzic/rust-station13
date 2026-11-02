# Space heaters

Sources: `luna:code/game/machinery/spaceheater.dm` (`New`, `update_icon`, `examine`,
`attackby`, `attack_hand`, `Topic`, `process`); `NSV_Luna.dmm`: 10.

Code: `game/classic/src/heaters.rs`; the window in `apps/client/src/heater_panel.rs`.
Tests: `game/classic/tests/space_heaters.rs`.

A dense machine that isn't bolted down, so it can be pulled. Each starts with a full
standard power cell inside (2 500 units), set to 50 °C.

## Heating

Every machine cycle (2 s), a heater that is on and has charge checks its tile. If the air
is colder than its setting, it takes a quarter of the tile's air, puts 100 kJ of heat into
it and gives it back, using 5 units of its cell (Luna `heating_power / 20000`). A full cell
lasts 500 heating cycles. Being colder is checked before each cycle, so the air can end up
above the setting by up to one cycle's heat. With no cell, or an empty one, it switches off.

## Using it

- **An empty hand** switches it on or off ("You switch on the space heater.", seen by
  others as "Jane Doe switches on the space heater."). With the hatch open it opens the
  window instead.
- **A screwdriver** opens or shuts the hatch ("You open the hatch on the space heater.").
  Shutting it closes the window.
- **A power cell** goes in through the open hatch ("You insert the power cell into the space
  heater."). With the hatch shut: "The hatch must be open to insert a power cell."; with a
  cell already inside: "There is already a power cell inside."
- **The window** ("Space Heater Control Panel"):
  - *Power cell: Installed* takes the cell out into an empty hand; *Removed* puts the
    cell in your hand in;
  - *Power Level* is the cell's charge;
  - *Set Temperature* goes 5 °C at a time with - and +, between 15 and 90 °C.
  It closes when you step away or the hatch shuts.
- **Examining it**: "The heater is on and the hatch is closed. The charge meter reads 80%."
  With the hatch open, whether the cell is installed instead.
- Its sprite is `sheater0` off, `sheater1` on, `sheater-open` with the hatch open.

## Differences from Luna

| Luna | Here |
|---|---|
| Cyborgs and the AI use its window from afar | There are no cyborgs or AI yet |
