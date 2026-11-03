# Autolathe

Source: `luna:code/game/machinery/autolathe.dm`; `NSV_Luna.dmm`: 3 on the decks.

Code: `game/classic/src/autolathe.rs`, `apps/client/src/lathe_panel.rs`. Tests:
`game/classic/tests/autolathe.rs`.

Click one with metal, plasteel, glass or reinforced glass sheets to feed it, or with an empty hand to open
its window. It holds up to 150 000 cm³ of metal and 75 000 cm³ of glass. A sheet of metal or glass gives
3750, plasteel 7500 of metal, and reinforced glass 1875 of metal and 3750 of glass.

The window shows what it has and lists what it makes, each with its cost. What it can't afford is greyed
out. Pressing a line takes the materials, and 1.6 s later the thing drops onto the machine's tile. It makes
one thing at a time, and needs power.

It makes 26 things: tools, sheets, surgical tools, beakers, a welding helmet, a console screen, .38 speed
loaders, igniters and timers.

## Differences from Luna

| Luna | Here |
|---|---|
| Any item with metal or glass in it can be fed in | Only sheets |
| Sheets can be made several at a time | One at a time |
| A panel to open and wires to hack, unlocking more recipes | Not built |
| Radios, headsets, tape recorders, rods, light tubes and shells are on the list | Not on it: some aren't items here, and Luna gives radios no cost |
| Upgrading its parts | Not built |
