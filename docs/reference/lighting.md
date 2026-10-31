# Lighting

Sources:
- `luna:code/modules/power/UltraLight.dm` (Sukasa's UltraLight: coloured light);
- `luna:code/modules/power/lighting.dm` (each fixture's red, green and blue strength);
- `luna:code/game/area/areas.dm` (areas without lighting).

Code:
- `engine/simulation/src/lighting.rs`: works out the light on each tile;
- `apps/client/src/lighting.rs` and `lighting.wgsl`: draw it;
- `game/classic/src/lights.rs`: which fixtures give off light.

Tests: `engine/simulation/src/lighting.rs` (the rules) and `game/classic/tests/lights.rs`
(lit fixtures give off light). ADR: `docs/adr/0008-soft-tile-lighting.md`.

## Light sources

A light has a red, a green and a blue strength, in tiles of reach. A lit fixture gives off
its light (`LightSource`, replicated); an unlit, empty, burnt out or broken one gives off
none. Flashlights, lanterns and desk lamps give off light too, from their own tile or from whoever
carries them (`flashlights.md`).

| Prototype | Luna type | Red, green, blue |
|---|---|---|
| `light` | `/obj/machinery/light` | 8, 8, 8 |
| `light-blue` | `light/blue` | 5, 5, 7 |
| `light-red` | `light/red` | 8, 2, 2 |
| `light-small` | `light/small` | 5, 5, 4 |
| `light-small-red` | `light/small/red` | 5, 0, 0 |

The coloured fixtures look like the plain ones and are the same to click.

## The light on a tile

For each colour, as in Luna:

1. Each light the tile can see leaves its strength less the distance to the tile.
2. These add up, but never past the strongest light that reaches the tile.
3. 7 (Luna `ul_Steps`) is fully lit; the level is that sum over 7, at most 1.

A tube lights its own tile and the next ones fully, and fades out eight tiles away.

- **Distance** is straight-line and not rounded, so light fades smoothly. Luna rounds it
  down to whole tiles, which gives rings.
- **Sight:** a tile is seen when the line from the light to its centre, or to near one of
  its corners, crosses no opaque tile: walls and closed airlocks. The opaque tile at the end
  is lit, so walls are lit on the sides that face the light and cast shadows behind them.
- **Space** is always fully lit, as Luna's space area has no lighting of its own.
- **The darkest** a tile is drawn is 4 % light, so a dark room is not quite black.

## Decks

Positions carry the deck. Light passes between decks only through a tile with no floor
(`open-space`):

- light reaching an open tile goes on to the tile below it, one tile weaker, and spreads on
  that deck from there;
- light reaching a tile under an open tile goes up to it in the same way;
- light that went down keeps going down, and light that went up keeps going up.

No map has `open-space` tiles or more than one deck yet. The rules are tested with a
made-up two-deck world.

## Drawing it

- **Who works it out:** the client, from the light sources and closed doors the server
  replicates. Light is only looks for now: nothing in the game depends on it.
- **When:** the client works it out again only when the map, a light or a closed door
  changes.
- **How it is drawn:** a texture with one texel per tile of the deck you stand on. Between
  tile centres the light is blended with smoothstep weights, so it fades smoothly with no
  creases. It is multiplied over everything on the map, coloured light included. The HUD
  and the text panel are not darkened.
- **F4** turns lighting off and on.

## Differences from Luna

- **Soft:** Luna lights whole tiles in seven steps; here light fades smoothly within and
  across tiles.
- **Light across decks:** Luna has no light between decks.
- **Not implemented:**
  - darkness hiding things from players (nothing checks light on the server);
  - red alert lighting (`isalert`, fire alarm lights);
  - other things that glow in Luna: lit welding tools, fires, computers, igniters,
    floodlights (flashlights and desk lamps do: `flashlights.md`);
  - floor lights;
  - light from a lower deck seen through open space (only the light levels are carried).
