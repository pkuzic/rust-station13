# Emergency wall lockers

Sources: `luna:code/game/machinery/walllockers.dm` (`walllocker`: `attack_hand`, `attackby`,
`dispence`, `New`; `emerglocker` and its four subtypes); `NSV_Luna.dmm`: 48 lockers.

Code: `game/classic/src/walllockers.rs`. Tests: `game/classic/tests/walllockers.rs`.

## Emergency locker (`emergency-locker`)

Hung on a wall: Luna's subtypes `emerglocker/north`, `south`, `east` and `west` set their
facing and a 32-pixel offset toward the wall (the importer's `type_vars`; the map's own
offset wins where it sets one). It starts with three crowbars, three breath masks and
three emergency oxygen tanks.

- **Empty hand:** one of each kind still there drops onto the locker's tile: "Dispensed a
  crowbar, breath mask and emergency oxygen tank." With nothing left: "It's empty..."
- **One of its kinds in the hand:** it goes back in while there are fewer than three of that
  kind: "You put the crowbar back in the locker." Anything else does nothing.
- **Sprite:** `emerg` from `lockwall.dmi` (a 31×31 icon, drawn in the bottom-left of its
  tile as BYOND does), facing the room.

## Differences from Luna

| Luna | Here |
|---|---|
| "Dispenced a" (sic) | "Dispensed a" |
| Silicons can't use it | There are no silicons yet |
