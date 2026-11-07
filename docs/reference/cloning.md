# Cloning

Sources: `luna:code/WorkInProgress/cloning.dm` (`computer/cloning`: `scan_mob`, `Topic`;
`clonepod`: `growclone`, `process`, `go_out`), `luna:code/game/dna.dm` (`dna_scannernew`);
`NSV_Luna.dmm`: 4 scanners, 2 consoles and 1 pod on deck 3.

Code: `game/classic/src/cloning.rs`, `apps/server/src/lib.rs` (`follow_to_clones`). Tests:
`game/classic/tests/cloning.rs`, `apps/headless/tests/cloning.rs`.

This is how the dead come back. Someone has to have been scanned, alive or dead, before their
body is lost.

1. **Scanner** (`dna-scanner`): pull someone and click it to lay them inside. Click again, pulling
   nobody, to let them out.
2. **Console** (`cloning-console`): needs power. With someone in the nearest scanner within 8 tiles,
   clicking scans them into the station's records. With nobody there, clicking starts cloning the
   first person on record who is a ghost now, in the nearest pod within 8 tiles.
3. **Pod** (`clone-pod`): the player goes straight into the new body, which starts with 60 to 90 brute
   damage spread over it, 30 of toxin and 40 of oxygen damage, unconscious and without clothes. Every
   2 seconds it heals, drawing 65 kW. A power cut makes it wait. Once health reaches 80, which takes
   about two and a half minutes, the pod lets them out, under the same name.

The record is used up by cloning. A ghost who has started over as a new character can't be cloned,
and the old body stays where it was.

## Differences from Luna

| Luna | Here |
|---|---|
| A window of records, each to view, delete and clone | Clicking does the next useful thing, first record first |
| The ghost is asked "Do you want to return to life?" | No question yet: they come back. R starts over instead, beforehand |
| Consoles and pods are linked to what stands right next to them | Nearest within 8 tiles |
| A clone comes out with 101 or more toxin damage, relying on chemicals to recover | It comes out as healed as the pod got it, as there are no chemicals yet |
| Brain damage, and random genetic defects | Not built |
| Early release with a card, emag, gibs when it fails | Not built |
| Saving records to disks | Not built |
| Mutations | The scanner and genes are `genetics.md`'s; cloning doesn't use them yet |
| Cloning brings back their mind's job and antagonist status | There are neither yet |
