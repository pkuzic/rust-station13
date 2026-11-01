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
- **The darkest** a tile is drawn is dim but readable (see Depth), so a dark room is not black.

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
  replicates, with the same engine code as the server (`MapDecks`), per tile as above.
- **When:** the client works it out again only when the map, a light or a closed door
  changes, and redraws it when what the player sees changes.
- **How it is drawn:** a texture with one texel per tile of the deck you stand on. Each
  tile shows its own light flat across most of its area and blends with its neighbour only
  in a narrow band at the edge (a quarter of a tile), so light and shadow read tile by tile,
  and only between tiles the player sees: light from a room behind a wall never
  shows on this side of it. It is multiplied over everything on the map, coloured light
  included. The HUD and the text panel are not darkened.
- **Walls and closed doors** show the light of the floor in front of the faces that turn
  towards you: the brightest of the open tiles beside them that you see. Light from a room
  behind a wall, or on a window beside a door, never shows on the face you see. A room's
  corner, with no face to see, takes the light at its corner.
- **Changes fade:** when a light goes on or off, a door opens or the player moves, the
  drawn light eases to the new levels over about a third of a second instead of snapping.
- **Fire flickers:** the light of a burning tile wavers between three quarters and full
  strength, each tile in its own rhythm. Fires are worked out afresh every frame; the rest
  of the light only when it changes.
- **F4** turns lighting off and on.

## Glow

What gives off light of its own stays bright in the dark and glows
(`apps/client/src/glow.rs`):

| What | Which pixels glow |
|---|---|
| Fire, laser beams, the supermatter, sparks | all of it |
| Screens and status lights: APCs, SMES units, air and fire alarms, power monitors, the laser computer, cell chargers, the TEG, freezers, engine lasers, meters, P.A.C.M.A.N. generators, light switches, intercoms | the bright, strongly coloured ones (indicator greens, reds, blues; not grey metal) |
| Lamps: tubes, bulbs, desk lamps, flashlights, lanterns | the bright ones, while lit |
| Overlays on those (a SMES's charge lights) | the bright ones |

- **Drawn over the darkness:** the glowing pixels are drawn a second time above it, a
  little brighter than white, and the world camera's bloom spreads a soft halo round them.
  The sight shadows and the view cone still cover them.
- **A faint light of their own:** a glowing screen also lights the floor round it, two
  tiles of reach, in the average colour of its glowing pixels (so an air alarm casts a
  green pool). A speck of a status light casts none. This is drawn only by the client;
  the server's darkness for game rules counts only the lights Luna gives light to.
- Colours are kept exactly as drawn (no tone mapping); only what is brighter than white
  blooms.
- **Fire** glows less than screens and lamps, and partly see-through, so its flames stay
  orange and keep their shape instead of washing out to yellow.

## Air you can see

- **Gas** (`apps/client/src/gas.rs`): plasma and sleeping gas are drawn over the tiles
  that hold enough of them, with Luna's animated `tile_effects.dmi` overlays: plasma
  above 0.5 mol on a tile (Luna `MOLES_PLASMA_VISIBLE`), sleeping gas above 1 mol. Luna
  draws them on or off; here the overlay is faint just above that amount and thickens to
  fully opaque at 10 mol, so a leak shows as a haze and a flooded room as a thick cloud.
  Drawn over people, lit by the light overlay, hidden on tiles out of sight. The server
  sends the amounts with the pressure picture (`AtmosView`), twice a second.
- **Heat haze** (`apps/client/src/heat.rs`, `world_view.wesl`): the air over burning
  tiles and the supermatter bends what is behind it. When the world image is put on the
  screen, each pixel is taken from up to 1.6 sprite pixels to the side and 0.9 up or down,
  in rising, wavering bands, as far as the air is hot: fully on a burning tile, 70 % on
  the supermatter, fading over the tile above, as hot air rises. Walls, windows, floor
  and people behind the heat waver; nothing is drawn over them. Luna has none.
- **Space** (`apps/client/src/space.rs`): instead of Luna's still star tiles, three
  layers of stars, drawn on whole sprite pixels, slide past at a tenth, a quarter and a
  half of the camera's movement, so the far ones seem far, and drift slowly as the ship
  travels. A few stars are blue or warm, the brightest bloom a little, and all twinkle.

## Depth

- **Wall bases** (`apps/client/src/depth.rs`): a crisp dark line three sprite pixels wide
  along the foot of every wall, on the floor side, so walls seem to stand up out of the
  floor. Drawn over the floor, under everything on it.
- **Contact shadows** (`apps/client/src/sprite_shadows.rs`): people and what stands on
  the floor (closets, machines, chairs, tables, canisters) have a soft dark oval on the
  floor under their lowest visible pixels, a few pixels wider than they are. It is
  measured from each sprite frame's own pixels, so it stays under the feet as people walk
  and turn, and it points nowhere, so it can't disagree with where the light is. All
  shadows lie in one layer, over the floor and what lies flat on it, under everything
  that stands, so a shadow never falls over another object. A person's shadow is their
  body's alone. Frames under 14 px tall (small things lying on the floor), what lies
  flat (pipes, cables, vents), what hangs on a wall and light itself have none.
- **Silhouettes** (`apps/server/src/silhouettes.rs`): someone standing on a tile darker
  than 20 % (the server's "dark") is sent to everyone else as a stand-in, a `Silhouette`
  with only their place, facing and body pose: never their name, clothes or what they
  hold, so even a modified client can tell someone is there, not who. Clients draw it as
  a black shape. Clicking it (to hit, help or pull) acts on the person; examining it tells
  nothing. In the light, they are sent in full again. You always see yourself.
- **Darkness is dim, not black:** the darkest a tile is drawn is dim but readable and a
  little cool, so a dark room still shows its shape; only what is out of sight is black.

## Differences from Luna

- **Soft:** Luna lights whole tiles in seven steps; here light fades smoothly within and
  across tiles.
- **Light across decks:** Luna has no light between decks.
- **Not implemented:**
  - darkness hiding things other than people (items on the floor of a dark room are
    still sent and drawn, dimly);
  - red alert lighting (`isalert`, fire alarm lights);
  - other things that give light in Luna: hard hat and suit lamps (flashlights and desk
    lamps do: `flashlights.md`; sparkers: `igniters.md`; floodlights: `floodlights.md`;
    screens glow: Glow);
  - floor lights;
  - light from a lower deck seen through open space (only the light levels are carried).

## Line of sight

Luna's view is BYOND's: a 15 × 15 square around the player, each tile either seen or
black, as the client draws it (`apps/client/src/shadows.rs`):

- **Seen:** a tile whose centre the straight line from the centre of the player's tile
  reaches without crossing a wall or closed door (`rs13_world::sight::sees`). A line
  passing exactly through a corner gets past if either tile beside the corner is open: a
  door diagonally past a wall's corner is seen, a gap between two walls that touch at a
  corner is not.
- **Walls and closed doors** are seen when a floor tile beside them is seen, and a room's
  corner when the floor touching its corner is, so the walls bordering what you see show,
  and the wall behind them doesn't.
- **Everything else** within view is covered by a black square, and nothing on it (items,
  machines, people) is drawn. The squares change as the player steps from tile to tile.
- **Meson scanners** (Luna `SEE_TURFS`): the black squares are see-through, showing the
  floor plan without what lies on it.
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
  darkened (never lightened, fading in from nothing at the player over a tile and a half),
  and other people standing in it are not drawn (nor can they be clicked).
  Items, machines and the floor still show, darker.
- **Neighbours:** the player's own tile and the eight around it are never hidden: you
  notice someone right next to you.
- **Facing** is decided by the server (BYOND `dir`): the way the player last walked or
  tried to walk, or turned with **Ctrl + a direction** without stepping. Someone buckled
  into a chair faces the way the chair does (Luna `buckle_mob`). Diagonal steps face
  diagonally; four-direction sprites then show their east or west side, as BYOND does.
- **Server side:** people behind your back aren't sent either (the same
  `rs13_world::sight::behind_back` test, from your tile and server-side facing). Turning
  round, they reappear after a round trip.
