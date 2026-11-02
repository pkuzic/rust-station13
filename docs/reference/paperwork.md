# Paperwork

Sources (Luna's build includes the old paper, not `modules/paperwork/paper.dm`):
`luna:code/defines/obj/weapon.dm` (`paper`), `luna:code/game/objects/items/weapons/papers_bins.dm`
(`paper`: `examine`, `attack_self`, `attackby`), `luna:code/modules/paperwork/paperbin.dm`,
`pen.dm`, `clipboard.dm`; `NSV_Luna.dmm`: 28 paper bins, 21 pens, 10 clipboards, 7 sheets.

Code: `game/classic/src/paperwork.rs`; the windows in `apps/client/src/paper_panel.rs`.
Tests: `game/classic/tests/paperwork.rs`.

## Paper bin (`paper-bin`)

Thirty sheets. An empty hand on it takes one out ("You take the paper out of the paper
bin."), the last one put back first; with none left: "The paper bin is empty!". A sheet
clicked onto it goes back in ("You put the paper in the paper bin."). Examining it counts
them. Sprite `paper_bin1`, or `paper_bin0` when empty. As in Luna, clicking it doesn't pick
it up (Luna drags it).

## Paper (`paper`)

Called "Paper" (or what the map names it). **A pen** (`pen`, `pen-blue`, `pen-red`) clicked
onto it opens its text box with what is written there: "What text do you wish to add?". You
edit the whole text: Enter starts a new line, Ctrl+Enter or **Save** writes it, Esc cancels;
at most 3 071 characters. **Examining** it opens it to read. **Used in the hand** it opens
its label box ("What would you like to label the paper?", 31 characters): it becomes
"paper- 'label'", or "paper" with no label. Luna's `[b]`, `[i]` and `[u]` tags are kept in
the text.

## Clipboard (`clipboard`)

A sheet clicked onto it is clipped on top ("You clip the paper onto the clipboard."); a pen
clicked onto it writes on the top sheet. Used in the hand, its window lists **Remove Pen**
or **Add Pen** (the pen in your hand goes in: "You slot the pen into the clipboard."), then
every sheet, top first, with **Write**, **Remove**, **Move to top** and its name to read it.
It only works while you carry it. Its sprite shows the top sheet, the pen and the clip.

## Differences from Luna

| Luna | Here |
|---|---|
| Reading shows `[b]`, `[i]` and `[u]` as bold, italic and underlined | Shown as plain text, the tags dropped |
| Monkeys read a sheet with words missing | There are no monkeys yet |
| Clumsy people cut themselves labelling paper | There is no clumsiness yet |
| Stamps, and paper burning up in a fire | Not built |
| A sheet can be worn on the head | Not built |
