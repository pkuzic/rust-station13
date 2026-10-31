# Pulling

Sources: `luna:code/modules/mob/mob.dm` (the `Pull` and `Stop Pulling` verbs, the `pull`
screen object, the split of diagonal moves), `luna:code/modules/mob/living/carbon/carbon.dm`
(`Move`: what the pulled thing does), `luna:code/game/atom_procs.dm` (`CtrlClick`),
`luna:code/game/hud.dm` (`ui_pull`).

Code: `game/classic/src/pulling.rs`; Ctrl-click and the HUD button in `apps/client`.
Tests: `game/classic/tests/pulling.rs`, and over the network
`apps/headless/tests/pulling.rs`.

## Luna's behaviour

- **Starting:** the Pull verb, or Ctrl-click (and Alt-click), on anything within a tile
  that isn't anchored: items, objects, people. Silent. Pulling something else lets go of
  the old; Ctrl-clicking the pulled thing again does nothing more.
- **Moving:** after the puller steps, the pulled thing follows only if it is now more than
  a tile away or diagonal to the puller. A person is moved straight into the tile the
  puller left; an object takes one step toward it, if nothing blocks the way. Pulling
  doesn't slow the puller.
- **Letting go:** the `pull` HUD button (bottom row, column 10; `pull1` while pulling,
  `pull0` otherwise) or the Stop Pulling verb. Also when the puller tries to step while
  the pulled thing is already out of reach, when it leaves the map (picked up) or becomes
  anchored, when cuffed, on grabbing, and on slipping.
- **The pulled:** a person can just walk away; the puller's next step then lets go. A
  restrained person can't walk away from someone conscious pulling them.

## Ours

| | Ours | From Luna |
|---|---|---|
| Start | Ctrl-click something within a tile; silent | keep (no Alt-click, no verb panel) |
| What can be pulled | items, people, closets and crates, chairs, canisters not wrenched to a port, generators not bolted down | keep: everything not anchored |
| Following | after each step, as in Luna: a person into the tile left, a thing one step toward it unless blocked; next to the puller, not diagonally, stays put | keep |
| Speed | no slowdown | keep |
| Letting go | the HUD pull button; stepping while it is out of reach; picked up, gone or anchored | keep; cuffs, grabs and slipping when those exist |
| The pulled walking away | free to; the pull ends on the puller's next step | keep; the restrained rule when restraints exist |
| Pushing it by clicking the floor | none | later |
| Dragging someone onto a chair to buckle them | none | later (needs mouse drag) |
