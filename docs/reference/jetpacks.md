# Reference spec: jetpacks

## What Luna does

Sources: `luna:code/game/objects/jetpack.dm`, `luna:code/modules/mob/mob.dm` (`/client/Move`),
`luna:code/game/turf.dm` (space `Entered`).

- **The tank:** a jetpack is a gas tank (`tank/jetpack`, w_class 4) filled with oxygen at 6 atmospheres, like the big oxygen tank. It can feed internals like any tank.
- **Variants:** plain, chief engineer's, syndicate and void. They differ only in sprite.
- **Switches:** two verbs. "Toggle Jetpack" turns the thrusters on; the sprite becomes `jetpack1`. "Toggle Jetpack Stabilization" turns on the stabiliser.
- **Moving in space:** a mob on a space turf with no simulated turf, grille or lattice next to it can only move with a jetpack on its back or belt.
  - Each step needs `allow_thrust(0.005)`: the thrusters on, at least 0.005 mol in the tank, which it uses.
  - A push also stops drifting (`inertia_dir = 0`).
  - Without a push, the mob can't move.
- **Stabiliser:** keeps the wearer from drifting on in space.
- **Decks:** diagonal keys move a jetpack wearer up or down between decks through open space.

## What we build (`game/classic/src/jetpacks.rs`)

| Item | Ours | From Luna |
|---|---|---|
| Items | `jetpack` and `jetpack-ce`: 70 L of oxygen at 6 atm, worn on the back, size 4, usable for internals | keep; the syndicate and void ones aren't on the map |
| Switch | clicking it in the active hand switches it on or off; it looks lit in hand and on the back | **simplified**: one switch for thrusters and stabiliser, since we have no verbs |
| Moving in open space | needs a switched-on jetpack on the back; each step uses 0.005 mol and stops drifting; no push, no step | keep. Back only: at size 4 it doesn't fit the belt. |
| Drifting | a switched-on jetpack keeps its wearer from drifting | keep (the stabiliser) |
| Between decks | none | multi-z is deferred |
| Ion trail | none | an effect only |
