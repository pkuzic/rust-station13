# Bedsheets and linen bins

Sources: `luna:code/game/objects/structures/bedsheet_bin.dm` (`bedsheet/attack_self`,
`bedsheetbin/examine`, `update_icon`, `attackby`, `attack_hand`); the wet floor sign,
`luna:code/defines/obj/weapon.dm` (`caution`). `NSV_Luna.dmm`: 19 bedsheets in seven
patterns, 2 linen bins, 12 wet floor signs.

Code: `game/classic/src/bedsheets.rs`. Tests: `game/classic/tests/filing_and_linen.rs`.

## Bedsheet (`bedsheet`, `bedsheet-blue`, `-brown`, `-captain`, `-hop`, `-medical`, `-rd`)

A tiny item (size 1), called "bedsheet" whatever its pattern. **Used in the hand** it is
dropped where you stand and spread out, drawn over whoever lies there (Luna's `layer = 5`);
used again after you pick it up, it is folded and drawn under them again.

## Linen bin (`linen-bin`)

Ten sheets.
- **An empty hand** takes one out ("You take the bedsheet out of the linen bin."). Sheets
  put back come out first; after that the bin makes new plain ones. With none left,
  nothing happens.
- **A bedsheet** clicked onto it goes back in ("You put the bedsheet in the linen bin.").
- **Something smaller than bulky** (size under 4) hides among the sheets, if the bin has
  sheets and nothing is hidden yet: "You hide the pen among the sheets." The next sheet
  taken out brings it with it, and it falls to the floor: "The pen falls out of the
  bedsheet!" Only the person taking the sheet is told.
- **Examining it** counts the sheets.
- Its sprite is `linenbin-full` (6 or more), `linenbin-half` (1 to 5) or `linenbin-empty`.

## Wet floor sign (`caution`)

Luna's "Wet Floor Sign": an ordinary item (size 3, force 1) to carry and set down.

## Differences from Luna

| Luna | Here |
|---|---|
| A bedsheet on the help intent starts surgery | There is no surgery yet |
| Explosions destroy bedsheets | There are no explosions yet |
| Examining the sign reads "Caution! Wet Floor!" | Items don't have descriptions yet |
