# Paperwork

Sources (Luna's build includes the old paper, not `modules/paperwork/paper.dm`):
`luna:code/defines/obj/weapon.dm` (`paper`), `luna:code/game/objects/items/weapons/papers_bins.dm`
(`paper`: `examine`, `attack_self`, `attackby`), `luna:code/modules/paperwork/paperbin.dm`,
`pen.dm`, `clipboard.dm`, `structures/noticeboard.dm`, `stamps.dm`, `folders.dm`, `photocopier.dm`; `supplyshuttle.dm`
(stamped manifests); `NSV_Luna.dmm`: 28 paper bins, 21 pens, 10 clipboards, 7 sheets, rubber
stamps, folders and two photocopiers.

Code: `game/classic/src/paperwork.rs`, `game/classic/src/bureaucracy.rs` (stamps, folders, the
photocopier); the windows in `apps/client/src/paper_panel.rs`.
Tests: `game/classic/tests/paperwork.rs`, `game/classic/tests/bureaucracy.rs`.

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

## Rubber stamps (`stamp`, `stamp-captain`, `stamp-hop`, `stamp-ce`, `stamp-rd`, `stamp-denied`)

Clicked onto a sheet in reach or in the other hand, or onto a clipboard (its top sheet): "You
stamp the paper with your rubber stamp." The sheet's text gets a line at the end, "This paper
has been stamped with the" and the stamp's name, in the reader's language; its sprite shows
the stamp. A stamped sheet can't be written on any more: "This paper has been stamped and can
no longer be edited." Stamping it again adds another line. A stamped supply manifest sent
back in its crate earns cargo 4 points (`cargo.md`).

## Folders (`folder-red`, `folder-yellow`, `folder-white`, `folder-blue`)

A sheet clicked onto one goes in ("You put the paper into the folder."), and a sheet shows
sticking out of it. Used in the hand, its window lists every sheet with **Remove** (into a free
hand, or at your feet) and **Read**. It only works while you carry it. A folder goes into a
filing cabinet like a sheet (`filing-cabinets.md`).

## Noticeboard (`noticeboard`)

A sheet clicked onto it is pinned up ("You pin the paper to the noticeboard."), five at most
("You reach to pin your paper to the board but hesitate. You are certain your paper will not be
seen among the many others already attached."); its sprite shows how many. An empty hand opens
its window: each sheet by name to read, **Write** (with a pen in the active hand: "You'll need
something to write with!") and **Remove**, which drops it on the floor under the board.
Handcuffed, you can't write or take one down ("It's a bit hard to remove the notice when you're
restrained like this.").

## Photocopier (`photocopier`)

Wrenched down, dense, on the equipment power channel (30 W), though like Luna's it works
unpowered. A sheet clicked onto it goes in ("You insert the paper into the photocopier."), one
at a time ("There is already something in the photocopier."). An empty hand opens its window:

- with a sheet in it, **Remove Paper**; and with toner, **Copy**, "Printing: N copies." and
  **-** / **+** (1 to 10);
- with nothing in it and toner left, "Please insert paper to copy.";
- "Current toner level: N", and "Please insert a new toner cartridge!" at 0.

**Copy** prints that many copies onto its tile, one every 1.5 s, each with the sheet's name and
text (a stamp's line too, but a copy isn't stamped and can be written on) and a unit of toner;
it stops when the toner runs out or the sheet is taken out. It starts with 30 toner. A **toner
cartridge** (`toner`) clicked onto it only goes in when it is empty, filling it to 30 ("This
cartridge is not yet ready for replacement! Use up the rest of the toner."). A **wrench** frees
it to be pulled or bolts it down again ("You unwrench the photocopier.", "You wrench the
photocopier."). An explosion destroys it at the heaviest, half the time at the next, and
otherwise may leave it without toner (always at the second, half the time at the lightest).
Toner cartridges come six to a crate (Toner cartridges, 10 points).

## Differences from Luna

| Luna | Here |
|---|---|
| Reading shows `[b]`, `[i]` and `[u]` as bold, italic and underlined | Shown as plain text, the tags dropped |
| Monkeys read a sheet with words missing | There are no monkeys yet |
| Clumsy people cut themselves labelling paper | There is no clumsiness yet |
| Luna's paper doesn't burn | Nor does ours |
| A pen labels a folder | Not built (custom names, 4.7) |
| Photos go in folders and the photocopier copies them | There are no photos yet (4.9) |
| Its scanner flashes as a sheet goes in, and copying draws 200 W | Neither: it draws its idle 30 W |
| Losing toner in an explosion leaves an oil stain | No stain |
| The hand labeler names things | Not built (custom names, 4.7) |
| A monkey reads a notice with words missing; the board's window can be used from anywhere once open | A notice reads in full; the window works within a tile |
| A notice is written on with a pen in either hand | In the active hand |
| The head of security's, chief medical officer's and clown's stamps | Not on the station |
| A sheet can be worn on the head | Not built |
