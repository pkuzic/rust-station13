# Flashlights

Sources:
- `luna:code/game/objects/items/devices/flashlight2.dm` (flashlights, penlights, lanterns,
  desk lamps, the hard hat lamp);
- `luna:code/game/objects/storage/belts.dm` (what a utility belt holds);
- `luna:code/defines/obj/closet.dm` (the tool closet's flashlight).

Code: `game/classic/src/flashlights.rs`. Tests: `game/classic/tests/flashlights.rs`.
Light itself: `lighting.md`.

## The lights

| Prototype | Luna type | Red, green, blue | Starts | Size | Worn |
|---|---|---|---|---|---|
| `flashlight` | `/obj/item/device/flashlight` | 4, 4, 4 | off | 2 | belt |
| `penlight` | `flashlight/pen` | 2, 2, 2 | off | 2 | belt |
| `lantern` | `flashlight/lantern` | 6, 6, 5 | off | 2 | belt |
| `desk-lamp` | `flashlight/lamp` | 6, 6, 6 | lit | 4 | no |
| `desk-lamp-green` | `flashlight/lamp/green` | 6, 6, 6 | lit | 4 | no |
| `hardhat` (all colours) | `clothing/head/helmet/hardhat` | 5, 5, 5 | off | 3 | head |

They conduct electricity and do no damage. A lit one shows its `on` sprite.

## Using one

- **Click it in the active hand** to switch it on or off. Nothing is said.
- **On the floor** a lit light lights its own tile.
- **Held or worn** (in either hand, or on the belt) it lights the tile of whoever carries it,
  and moves with them. Several carried lights add up, as in Luna: a flashlight and a
  lantern give 10, 10, 9.
- **In a bag, a box or a closet** it gives no light. Someone inside a closet gives none either,
  and can't switch a light there, since nothing can be used from inside a closet
  (`closets.md`).

## Where they are

- One in every tool closet (Luna `toolcloset`).
- A utility belt holds flashlights, penlights and lanterns.
- Luna's engineering section has three, imported with the map.

## Differences from Luna

- **Closets:** Luna refuses to switch a light inside a closet with a message; here nothing
  can be used from inside one, so the click does nothing. Luna's light also still shines
  from a mob in a closet; here it doesn't.
- **Desk lamps** inherit Luna's belt slot; ours can't be worn.
- **Not implemented:**
  - shining a flashlight in someone's eyes (Luna checks their pupils, with the eyes zone
    selected);
  - flares, and the emergency toolbox that carries a flashlight or a flare;
  - the janitor's closet's two flashlights (no janitor's closet yet).
