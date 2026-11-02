# HUD and screen layout

Code: `apps/client/src/hud.rs` (HUD), `apps/client/src/main.rs` (camera, text panel).
Icons: Luna `icons/mob/screen1.dmi`, the `hud` sprite in `client-assets/sprites.ron`.

## Layout

The window follows the classic BYOND client:
- **Left:** a square map screen, as tall as the window.
- **Right:** a grey text panel. It has stats and the hover tip on top, the white message
  output below, and the say line at the bottom.

The screen square is 16 × 16 cells:
- the 15 × 15 map view, centred on the player (view range 7);
- one row of the HUD below the map;
- one column of the HUD to its right.

Cells use Luna's `screen_loc` coordinates: x 1–16 from the left, y 0–15 from the bottom
(`code/game/hud.dm`). The map is scaled to fit the square, so cells are not always 32 px.

## Scaling

Code: `apps/client/src/scaling.rs`.

The square is 512 sprite pixels across, and the window can be any size. Stretching pixel art
by a fractional factor with nearest sampling makes some pixels wider than others, and they
shimmer as the view scrolls. Smooth sampling instead blurs everything. So the client does this:

- **World:** drawn into an image of `k × 512` px with nearest sampling, where `k` is the
  smallest whole number that makes it at least as big as the square on screen (at most
  4). Every sprite pixel is exactly `k × k`. The image is then shown in the square with
  smooth sampling. It only ever shrinks, by between ½ and 1, so pixels stay even and their
  edges soften by about one screen pixel.
- **HUD and panels:** their icons come from a copy of the sprite atlas enlarged `k` times,
  shown the same way.
- **Moving:** the camera and gliding mobs snap to the image's pixels (`1/k` of a sprite
  pixel), so nothing shimmers while walking.
- **Window resizing:** `k` is recomputed and the image and atlas copy are rebuilt when it
  changes.
- **Testing:** `--size WIDTHxHEIGHT` opens the client at a given size. `--use DX,DY` uses whatever is that many tiles east and north of the player two seconds after joining, to get a machine window into a `--screenshot`.

| Cell | Where (x, y) | Luna icon |
|---|---|---|
| Mask, head, back | (1, 2), (2, 2), (3, 2) | `equip` N, `hair`, `equip` NE |
| Right hand, suit, left hand | (1, 1), (2, 1), (3, 1) | `equip` W, `equip` S, `equip` E |
| Shoes, gloves | (4, 1), (5, 1) | `shoes`, `gloves` |
| Ears, glasses | (6, 1), (7, 1) | `ears`, `glasses` |
| ID, uniform, belt | (1, 0), (2, 0), (3, 0) | `equip` SW, `center`, `belt` |
| Swap hands | (6, 0) | `hand`: S when the left hand is active, N for the right |
| Drop | (7, 0) | `act_drop` |
| Intent | (12, 0) | `help` / `harm` |
| Run / walk | (14, 0) | `running` / `walking` |
| Internals | (16, 13) | `internal0/1` |
| Oxygen, toxins, fire | (16, 11), (16, 9), (16, 7) | `oxy0/1`, `tox0`, `fire0/1` |
| Body temperature | (16, 5) | `temp-4` … `temp4` |
| Health | (16, 4) | `health0` … `health7` |

**Inventory slots:**
- The right hand is on the left of the screen: you face the player.
- The slots overlap the bottom-left corner of the map, as in Luna.
- Luna hides shoes, gloves, ears and glasses behind an "other" toggle. We show them all the time.

**Left out, with nothing behind them yet:**
- pockets;
- throw, pull and resist buttons;
- sleep and rest indicators.

## Indicators

| Indicator | Shows | Thresholds (Luna `human_life.dm`) |
|---|---|---|
| Health | health and state | ≥100 → 0; 80 → 1; 60 → 2; 40 → 3; 20 → 4; 0 → 5; below 0 → 6; dead → 7 |
| Body temperature | body temperature (K) | ≥370 → 4; 350 → 3; 335 → 2; 320 → 1; 300 → 0; 295 → −1; 285 → −2; 280 → −3; colder → −4 |
| Oxygen | the low-oxygen alert | |
| Fire | the hot alert | |
| Toxins | always off: nothing toxic is breathed yet | |
| Internals | breathing from a tank | |

Pressure is not one of Luna's icons, so it stays as text in the stat panel.

## Screen effects

How the player is also shows on the view itself (`apps/client/src/world_view.rs`,
`world_view.wesl`), from the same status the indicators use. Luna's crude versions are
full-screen HUD overlays (blindness, the flash); here they are drawn on the map picture,
and each eases in and out over about a quarter of a second.

| State | Effect |
|---|---|
| Low-oxygen alert | the edges of the view darken (60 % at the corners), pulsing a little |
| Health below 70 | the edges go dark red, more as health falls, fully at 0 |
| Health drops | a red flash, strongest at the edges, fading in about half a second |
| Unconscious | the view blurs (2.5 sprite px) and darkens |
| Dead | the view turns grey and dark |
| Body temperature below 300 K | a blue tint, strongest 25 K lower |
| Body temperature above 320 K | a warm tint, strongest 40 K higher |

The HUD and the text panel are not affected.

## Clicks

| Cell | Click |
|---|---|
| Slot | Holding something: put it on. Empty hand: take off what is there. |
| Inactive hand | Make it active |
| Swap hands, drop | As X and Q |
| Intent | Toggle help and harm |
| Run / walk | Toggle the default. Shift held does the opposite. |
| Internals | Turn internals on or off (also E) |

Hovering over a cell shows its name, and what is in it, in the stat panel. A click on the HUD never reaches the map underneath.

## Right-click menu

BYOND's context menu: right-clicking a tile of the map lists everything on it, on your own
deck, topmost first, each with **Examine**, **Use** (as a left click with the active hand)
and **Pull** (as Ctrl-click), then its own verbs; yourself with Examine only. The verbs
(`game/classic/src/verbs.rs`), checked and done by the server:

| Verb | On | Does (Luna) |
|---|---|---|
| **Toggle Open** | closets, lockers, crates, within a tile | what an empty hand does, whatever you hold (`verb_toggleopen`) |
| **Toggle Lock** | secure lockers, from beside them, shut | locks or unlocks with its access (`verb_togglelock`) |
| **Rotate Chair** | chairs, within a tile | a quarter turn anticlockwise, sitter and all (`chair/rotate`) |
| **Toggle Light** | flashlights and lamps you carry | switches it (`flashlight/verb/toggle`) |
| **Toggle Jetpack** | jetpacks you carry | switches it (`jetpack/verb/toggle`) |
 Picking one does it; a left click
elsewhere or Escape closes the menu without doing anything else. Code:
`apps/client/src/context_menu.rs`.

| Luna | Here |
|---|---|
| Every verb of every object (an item's Move to Top, the jetpack's stabiliser, reading an ID card, a window's Rotate) | The verbs above |
