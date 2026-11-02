# Decks, ladders and stairs

Sources:
- `luna:code/game/objects/MultiZ.dm` (`obj/multiz`, `ladder/attack_hand`, `stairs/New`,
  `stairs/active/Bumped`);
- `NSV_Luna.dmm`: decks 1 (top) to 4 (bottom), about 40 ladder ends and 20 staircases.

Code: `game/classic/src/multiz.rs`; decks in `game/classic/src/decks.rs`,
`rs13_simulation::atmos::AtmosGrid::new_decks`, `objects::spawn_station`.
Tests: `game/classic/tests/multiz.rs`, `game/classic/tests/luna_station.rs`,
`apps/headless/tests/decks.rs`.

## Decks

A station is several decks, each a map with its own `z`, numbered bottom first (Luna's
deck 4 is our 0, its deck 1 our 3). The server loads them together (`--map luna-station`,
or `--map-file` given once per deck, bottom first):

- **Air:** one simulation across all decks. A room reaches across decks only through open
  floor (`open-space`): a hole in the deck shares its air with the tile below it, as Luna's
  open floor shares its zone. Otherwise each deck keeps its own air.
- **Sight:** you see and are sent only your own deck.
- **The client** draws only the deck you are on, and switches when you change deck.

## Ladder (`ladder`)

- **Map vars:** `istop` (Luna's default 1): the top end goes one deck down, the bottom end
  one deck up.
- **Clicking it,** with anything or nothing in the hand, from beside it, takes you to the
  same spot on that deck, if nothing stands there.
- **Sprite:** `ladderdown` at the top end, `ladderup` at the bottom.

## Stairs

Two tiles on each deck: an entry and an active tile, facing the way you walk from top to
bottom. Each deck's entry lies over the other deck's active tile.

| Prototype | Luna | Is |
|---|---|---|
| `stairs-down` | `stairs/active` | the top stairs' active tile: solid; walking into it takes you one deck down, to the same spot |
| `stairs-up` | `stairs/active/bottom` | the bottom stairs' active tile: solid; walking into it from an entry tile takes you one deck up |
| `stairs-entry` | `stairs/enter` | the top stairs' entry |
| `stairs-entry-bottom` | `stairs/enter/bottom` | the bottom stairs' entry |

- **Sprite:** Luna's `ramptop` for the top entry and the bottom active tile, `rampbottom`
  for the others (`stairs/New`), facing their way.

Ghosts click a ladder or stairs from anywhere to go to the same spot on that deck (`ghosts.md`).

## Differences from Luna

| Luna | Here |
|---|---|
| Open floor over space would vent into it | Only sideways openings reach space; none of Luna's open floor lies over space |
| Anything pushed into stairs goes up or down | People only |
