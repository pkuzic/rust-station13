# Filing cabinets

Sources: `luna:code/modules/paperwork/filingcabinet.dm` (`initialize`, `attackby`,
`attack_hand`, `Topic`); `NSV_Luna.dmm`: 14 filing cabinets and 3 chest drawers.

Code: `game/classic/src/filing.rs`; the window in `apps/client/src/pick_panel.rs`.
Tests: `game/classic/tests/filing_and_linen.rs`.

## Kinds

- **Filing cabinet** (`filing-cabinet`, sprite `tallcabinet`).
- **Chest drawer** (`chest-drawer`, sprite `chestdrawer`).

Both are dense and start bolted down.

## Behaviour

- At round start the sheets lying on a cabinet's tile go into it.
- **A sheet clicked onto it** goes in ("You put the paper in the filing cabinet.").
- **A wrench** unbolts it, so it can be pulled, or bolts it down again ("You unwrench the
  filing cabinet.", "You wrench the filing cabinet."), with Luna's ratchet sound at 50.
- **Anything else** won't go in ("You can't put the crowbar in the filing cabinet!").
- **An empty hand** opens its window. The title is the cabinet's name, and each sheet is
  listed by name, the last one put in first. Clicking a name puts that sheet in your hands
  (the empty one, else the other, else at your feet) and closes the window. The window
  closes too if you step out of reach or the cabinet empties. An empty cabinet says so:
  "The filing cabinet is empty."
- The drawer shows open for half a second after a sheet goes in or comes out (`-open`).

## Differences from Luna

| Luna | Here |
|---|---|
| Folders and photos go in too | There are no folders or photos yet |
| The rolling chest drawer, from the supply shuttle | There is no supply shuttle yet |
