# Morgue

Sources: `luna:code/game/machinery/morgue.dm` (`morgue/update`, `attack_hand`,
`relaymove`; `m_tray/attack_hand`, `MouseDrop_T`); `NSV_Luna.dmm`: 6 drawers.

Code: `game/classic/src/morgue.rs`. Tests: `game/classic/tests/morgue.rs`.

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
| Crematoriums, with their switch | Not built yet |
