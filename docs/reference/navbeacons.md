# Navigation beacons and circuit boards

Two things from Luna's map that wait for something else to be built.

## Navigation beacon (`navbeacon`)

Sources: `luna:code/game/machinery/navbeacon.dm`; `NSV_Luna.dmm`: 24 on the decks, on the
security patrol route and the delivery route.

Code: `game/classic/src/navbeacons.rs`.

It lies under the floor tiles, and shows once they are gone. It keeps what the map set: its
frequency (default 1445), its location, and its codes (`patrol;next_patrol=Chapel`). Nothing asks
it anything yet, because bots aren't built.

Not built: its radio replies, and its cover and lock.

## Circuit board (`circuit-board`)

Sources: `luna:code/game/machinery/computer/buildandrepair.dm`; `NSV_Luna.dmm`: 31 on the decks.

A small item (size 2) to pick up and carry. Luna's boards each build a particular computer, and
ours will need to as well once computers can be built. Until then they are all the same item.
