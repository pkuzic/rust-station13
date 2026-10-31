# Lighting

Sources:
- `luna:code/modules/power/UltraLight.dm` (Sukasa's UltraLight: coloured light);
- `luna:code/modules/power/lighting.dm` (each fixture's red, green and blue strength);
- `luna:code/game/area/areas.dm` (areas without lighting).

Code:
- `engine/simulation/src/lighting.rs`: works out the light on each tile;
- `apps/client/src/lighting.rs` and `lighting.wgsl`: draw it;
- `game/classic/src/lights.rs`: which fixtures give off light;
- `game/classic/src/lighting.rs`: the light on each tile on the server, for game rules.

Tests: `engine/simulation/src/lighting.rs` (the rules) and `game/classic/tests/lights.rs`
(lit fixtures give off light; the server knows where it is dark). ADR: `docs/adr/0008-soft-tile-lighting.md`.

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

## Darkness

A tile is **dark** when its brightest colour is below 20 % light (`DARK_BELOW`): about six
and a half tiles from a tube, with nothing else lighting it. Taking the brightest colour
means a tile lit only red is as bright as its red light. Space is never dark.

Game rules ask the server: `TileLight::is_dark(pos)`, `brightness(pos)` (0 to 1) and
`level(pos)` (red, green and blue) in `game/classic/src/lighting.rs`. The server works the
light out from the same lit fixtures, closed airlocks and map as the client draws, with the
same engine code (`MapDecks`), again only when one of them changes, at the end of the tick.
Rules see the light as it was at the end of the previous tick.

Luna has no such check: its game rules never ask how much light is on a tile. Nothing
here uses darkness yet.
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
  replicates. The server works out the same light for game rules (see Darkness).
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
  - darkness hiding things from players (the server knows where it is dark, but no rule
    uses it yet);
  - red alert lighting (`isalert`, fire alarm lights);
  - other things that glow in Luna: lit welding tools, fires, computers, igniters,
    floodlights (flashlights and desk lamps do: `flashlights.md`);
  - floor lights;
  - light from a lower deck seen through open space (only the light levels are carried).

## Line of sight

Luna's view is BYOND's: a 15 × 15 square around the player, minus what walls and closed
doors hide. The client draws it as real shadows, as if sight weren't tile based
(`apps/client/src/shadows.rs`):

- **Shadows:** every wall and closed door near the player casts a shadow away from the
  player's eye, out past the edge of the screen. They follow the player's drawn position,
  so they move smoothly while walking.
- **Walls** are seen when one of their exposed sides is; a wall none of whose sides can be
  seen is shaded whole, so walls seen edge-on show no slivers.
- **What lies on a tile** (items, machines, people) is drawn only when the straight line
  from the player's tile centre to the tile's centre crosses no wall. A line passing
  exactly through a corner gets past if either tile beside the corner is open: a door
  diagonally past a wall's corner is seen, a gap between two walls that touch at a corner
  is not.
- **Meson scanners** (Luna `SEE_TURFS`): the shadows are see-through, showing the floor
  plan without what lies on it.
- **F4** (lighting off) shows everything, for debugging.
- **Server side** (`apps/server/src/sight.rs`): each client is sent only the things on the
  map its mob can see, by the same test (`rs13_world::sight`), out to one tile past the
  view. Always sent: the player's own mob and what it holds and wears, closed doors and
  light sources (the client needs them to draw shadows and light). What other people hold
  and wear is sent while they are. What is inside a container (a backpack, a box, a first
  aid kit, the player's own too) is sent only while the player has it open, and nothing
  shut in a closet or locker is sent. Shut in a closet yourself, you see from where it
  stands. Never
  sent: anything under intact floor tiles (pipes, cables, vents), until the tile is pried
  up. A client that has just joined gets its first pass before anything is sent to it. So
  a modified client can't see through walls or floors.

## Field of view

Not in Luna; it follows later SS13 servers' view cones (`apps/client/src/fov.rs`).

- **Behind your back:** a 120° arc behind the player, centred on the way they face, is
  grayed over, and other people standing in it are not drawn (nor can they be clicked).
  Items, machines and the floor still show, grayed.
- **Neighbours:** the player's own tile and the eight around it are never hidden: you
  notice someone right next to you.
- **Facing** is decided by the server (BYOND `dir`): the way the player last walked or
  tried to walk, or turned with **Ctrl + a direction** without stepping. Someone buckled
  into a chair faces the way the chair does (Luna `buckle_mob`). Diagonal steps face
  diagonally; four-direction sprites then show their east or west side, as BYOND does.
- **Server side:** people behind your back aren't sent either (the same
  `rs13_world::sight::behind_back` test, from your tile and server-side facing). Turning
  round, they reappear after a round trip.
