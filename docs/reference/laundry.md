# Laundry

Source: `luna:code/game/machinery/washer.dm`. `NSV_Luna.dmm`: two laundromats in the laundry.

Code: `game/classic/src/washers.rs`. Tests: `game/classic/tests/laundry.rs`.

## The laundromat (`washer`)

Dense, on the equipment power channel, drawing nothing; it won't start unpowered. It starts with
no cleaning solution.

- **Cleaning solution:** a beaker, bottle or bucket holding at least a unit of cleaner (space
  cleaner, `chemistry.md`) clicked onto it pours all its cleaner in: "You transfer N units of
  cleaning solution to the laundromat." Examining it tells how much is left.
- **Clothes:** anything worn other than belts, ID cards and headsets, and backpacks, clicked onto
  it go in; those around see "X adds Y into the laundromat." Anything else: "The laundromat beeps
  when you try to put in Y."
- **A wash:** an empty hand starts one if it has a unit of cleaner ("You need more cleaning
  solution." otherwise). It churns for 5.1 seconds with its sound; nothing more goes in ("You
  can't put something in it while it's washing."). Then the fingerprints and fibres are gone
  from everything inside (`forensics.md`), it all comes out onto its tile, and a unit of cleaner
  is used.
- **Empty** (right-click, within a tile): everything inside comes out without washing; while it
  washes, "The door is locked."
- An explosion destroys it at the heaviest, half the time at the next and one time in ten at the
  lightest, and what was inside lands on the floor.

## Differences from Luna

| Luna | Here |
|---|---|
| A wash also takes off blood and plasma contamination | Neither stays on things here |
| Empty lets things out even while it washes, after saying the door is locked | Nothing comes out until the wash ends |
| Dragged onto it, someone is put inside (only clothes when it isn't hacked), and a wash hurts whoever is in it | Nothing is dragged in; only clothes and backpacks go in |
| An energy gun inside blows up | No energy guns yet |
| Its gold wire, when cut, keeps it washing and lets a click turn it off | Its wires aren't built (the variables do nothing in Luna either, as nothing cuts them) |
