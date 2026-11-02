# Window doors

Sources: `luna:code/game/machinery/doors/windowdoor.dm` (`New`, `Bumped`, `CanPass`,
`CheckExit`, `open`, `close`, `attackby`), `luna:code/defines/obj/door.dm` (the subtypes);
`NSV_Luna.dmm`: about 75, a direction and a side each.

Code: `game/classic/src/windoors.rs`. Tests: `game/classic/tests/windoors.rs`.

## Kinds

- **Interior door** (`windoor`, Luna "Interior Door"): looks `left` or `right`.
- **Brig door** (`brig-windoor`, Luna "Brig Door"): looks `leftsecure` or `rightsecure`.

Luna's subtypes (`northleft`, `brigdoor/eastright`, …) give the edge it stands on and its
look; the importer turns them into a facing and a `base_state`. A door that wants access
takes its look from the map's icon state instead, as Luna's `New` does. Access comes from
`req_access_txt`, as for airlocks.

## Behaviour

- It stands on one edge of its tile, the one it faces. Shut, it blocks that edge both ways,
  for people, pulled things, thrown puffs and space wind, and it seals the edge for air.
  The rest of the tile is free.
- **Walking into it** with its access opens it: the `opening` animation and Luna's
  `windowdoor.ogg` (volume 100), passable after 1 s. It then shuts by itself, after 10 s for
  a door anyone may use and after 4 s for one that wants access.
- **Clicking it** in reach, with an empty hand or anything: with the access, a shut door
  opens and an open one shuts, and it stays so. Without, a shut door flashes `deny`.
- Shutting plays the `closing` animation and the sound; it seals at once, and takes 1 s
  before it can be used again. Nothing happens to a door mid-swing.
- The card counted is the one in the active hand, else the one in the ID slot.

## Differences from Luna

| Luna | Here |
|---|---|
| The opening and closing animations play their seven frames | Their first frame shows for the second they take |
| An emag sparks it open for good | There is no emag yet |
| Bots with the access open it | There are no bots yet |
| Maps give some doors names ("Bar Pickup", "Brig Cell 1") | Not kept: every door goes by its kind |
| Brig doors are tied to a cell timer by `id` | There are no cell timers yet |
