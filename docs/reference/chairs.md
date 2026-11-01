# Reference spec: chairs

## What Luna does

Sources: `luna:code/game/objects/structures/stool_bed_chair_nest/chairs.dm`, `bed.dm`,
`stools.dm`; `luna:code/modules/mob/living/carbon/life.dm` (`check_if_buckled`,
`update_canmove`).

Chairs are a kind of bed, which is a kind of stool: anchored, not dense, and facing a way.

- **Buckling:** dragging a mob standing on the chair onto it buckles it in, if the dragger is within one tile and can act. Everyone sees "[name] buckles in!", or "[name] is buckled in to [chair] by [user]!". The mob turns the chair's way.
- **While buckled:** a mob can't move (`canmove = 0`). In a chair (unlike a bed) it doesn't lie down and keeps its hands free.
- **Hand on the chair:** unbuckles whoever is in it. "[name] unbuckled himself!" or "[name] was unbuckled by [user]!".
- **"Rotate Chair" verb:** from within one tile, turns the chair a quarter anticlockwise (`turn(dir, 90)`) and its sitter with it.
- **Wrench:** takes any stool or chair apart into a metal sheet, unbuckling its sitter.
- **Sprites:** `chair` and `comfychair_<colour>` in `icons/obj/objects.dmi`, with four directions. A chair facing north draws over its sitter.

## What we build (`game/classic/src/chairs.rs`)

| Item | Ours | From Luna |
|---|---|---|
| Chairs | `chair` and `comfy-chair-brown`, facing the map's way | keep; other comfy colours aren't in the region |
| Buckling in | an empty hand on the chair you stand on buckles you in | **simplified**: no dragging yet, so you can only buckle yourself |
| Buckled | can't walk; click the chair to get out; leaving its tile any other way unbuckles you | keep |
| Unbuckling someone else | an empty hand on an occupied chair, from beside it | keep |
| Turning | an empty hand on an empty chair from beside it turns it a quarter anticlockwise | **simplified**: the verb becomes a click |
| Wrench | a metal sheet on its tile; the sitter is freed | keep |
| Sitter faces the chair's way | buckled in, you face the chair's way and can't turn (`movement::sync_facing`) | keep |
| North-facing chair drawn over the sitter | the client draws it above people when it faces north | keep |
| Stools (`stool`, `barstool`) | somewhere to sit: an empty hand does nothing, a wrench takes them apart into a metal sheet | keep |
| Beds (`bed`) | an empty hand on the bed you stand on buckles you in lying down (`lying`); click it again to get out; they don't turn; a wrench takes them apart | keep; lying in bed doesn't stop you using your hands yet |
| Roller beds, buckling others by dragging | none | need dragging |
