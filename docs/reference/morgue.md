# Morgue and crematorium

Sources: `luna:code/game/machinery/morgue.dm` (`morgue/update`, `attack_hand`,
`relaymove`; `m_tray/attack_hand`, `MouseDrop_T`; `crematorium/update`, `attack_hand`,
`relaymove`, `cremate`; `c_tray`; `crema_switch/attack_hand`),
`luna:code/defines/obj/machinery.dm` (`crema_switch`), `luna:code/game/objects/items/weapons/urn.dm`;
`NSV_Luna.dmm`: 6 drawers, 1 crematorium in the chapel with its igniter.

Code: `game/classic/src/morgue.rs`. Tests: `game/classic/tests/morgue.rs`,
`game/classic/tests/crematorium.rs`.

## Drawer (`morgue`) and tray (`morgue-tray`)

A dense drawer, bolted down.
- **An empty hand on the drawer** pulls its tray out one tile east, if nothing solid or dense
  is there. What was inside is laid out on the tray. Clicked again, or **an empty hand on
  the tray**, slides the tray back in with every item and person lying on it. Luna's
  `Deconstruct.ogg` plays at 50 either way, even when the tray has no room.
- **The tray** is dense too, so nobody walks onto it. **Clicking the tray while pulling
  someone** lays them on it and lets go: "Jane Doe stuffs John Smith into the morgue tray!"
  This stands in for Luna's dragging a body onto it.
- **Inside**, people are hidden and stay put. Someone awake inside who tries to walk pulls
  the tray out, and themselves with it (Luna `relaymove`).
- Its sprite is `morgue1` empty, `morgue2` with something inside, `morgue0` with the tray
  out; the tray is `morguet`.

## Crematorium (`crematorium`, `crematorium-tray`) and igniter (`crematorium-igniter`)

A crematorium is a morgue drawer whose tray (`cremat`) slides out **south**. Its sprite is
`crema0` with the tray out, `crema2` with something inside, `crema1` empty.

- **The igniter** on the wall lights every crematorium with its map `id`. It wants
  crematorium access (a Counselor's card), in the hand or worn; without it: "Access denied."
  Anything in the hand works as a hand.
- **Lit empty**, it only crackles: "You hear a hollow crackle."
- **Lit with something inside**: "You hear a roar as the crematorium activates." It stays
  shut while it burns: "It's locked.", and nobody inside can get out. Each second for ten
  seconds, everyone inside takes 30 burn damage spread over the body.
- **After ten seconds** everyone inside is gone, an urn (`urn`) left for each. Of what each
  held or wore, every piece is left beside the urn one time in ten, and otherwise burns with
  them, along with anything stored in it. Things put in without anyone are left as they
  were (Luna burns only people). It dings (Luna `ding.ogg`) and opens again.

## Not in Luna's build

Luna's map places sinks (11) and a kitchen sink, but their code (and toilets' and urinals')
is commented out in `watercloset.dm`: the map's sinks are bare types with no sprite and no
use, so they aren't imported.

## Differences from Luna

| Luna | Here |
|---|---|
| A body is dragged onto the tray | Click the tray while pulling them |
| A pen labels the drawer ("Morgue- 'label'") | Not built: objects don't take names yet |
| Explosions break drawers open | There are no explosions yet |
| A cremated player becomes a ghost | There are no ghosts yet: the player is left with no body |
| The urn is named and described after whose ashes it holds | Objects don't take names yet: a plain urn |
| Players burn one after another at 30 a second, others together at 50 | Everyone inside burns together at 30 a second |
| The admin room's crematorium (`admincell1crema`) | Not on the station decks, so not imported |
