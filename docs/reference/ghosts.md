# Ghosts

Sources: `luna:code/modules/mob/dead/observer/observer.dm` (`New`, `Move`, `reenter_corpse`,
`dead_tele`), `luna:code/modules/mob/dead/observer/say.dm`. The look follows OnyxBay
(<https://github.com/ChaoticOnyx/OnyxBay>, `code/modules/mob/observer/`): a ghost is its body's
picture, standing and half see-through, not Luna's white sheet.

Code: `game/classic/src/ghosts.rs`, `apps/server/src/lib.rs` (`follow_to_ghosts`),
`apps/server/src/sight.rs`, `apps/client/src/sprites.rs` (`draw_ghosts`). Tests:
`game/classic/tests/ghosts.rs`, `apps/headless/tests/ghosts.rs`.

## Leaving the body

A player leaves their body as soon as it dies, or when it is burnt up alive in a crematorium
(`morgue.md`). They control the ghost from then on. The body stays where it lies and is
nobody's: it stays when they disconnect, and medicine and pulling treat it as before.

A ghost looks as the body did at that moment, in one flat picture at half opacity
(`GHOST_ALPHA`):

- it stands, whatever the body was doing;
- it faces the way the body faced;
- it wears what the body wore, kept with the ghost, so stripping or burning the body changes nothing.

## What a ghost can do

| | |
|---|---|
| **Move** | One tile every 0.1 s, through anything, over open space. Only the edge of the deck stops it. Nothing pushes it. |
| **Be seen** | Only by other ghosts. The server never sends one to the living. |
| **See** | Everything within view on its deck, all round, through walls and in the dark, but not what is under the floor or in bags. Lighting is off for it; F4 brings the dark back. |
| **Hear** | Speech and things happening within 7 tiles, through walls, and sounds as anyone would. |
| **Talk** | Only to other ghosts, wherever they are (`ghost-say`). The living hear nothing. |
| **Click** | A ladder or stairs, from any distance: to the same spot on the deck they lead to. Anything else: told what it is, as if examining. It can touch nothing. |

The HUD's cells are hidden, and the status line says they are a ghost.

## Starting over

Pressing R as a ghost (Luna's "Respawn") starts a new character at once, in the starting
outfit, at the first start point nobody stands on. If there is none, they are told and stay a
ghost. The old body stays where it is, and the ghost is gone.

## Differences from Luna

| Luna | Here |
|---|---|
| White sheet ghost | The body's picture (OnyxBay) |
| "Ghost" verb, for anyone at any time; the dead stay in their body until they use it | Leaving is automatic on death, and only then |
| "Re-enter Corpse" | Death is permanent, so there is nothing to go back to |
| "Teleport" to an area every 10 s | Not built |
| Examining shows descriptions | Only what it is, not machine details (those live in the living's click handling) |
| Radio chatter heard | Only radios within reach of hearing a person would have |
| A new character goes through the lobby and picks a new name | Straight to the first free start point, under the same name: there is no lobby yet |
