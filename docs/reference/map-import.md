# Importing maps from Luna

Tool: `tools/dmm-import` (`rs13-dmm-import`).
Rules: `game/classic/maps/luna_engineering.import.ron`.
Output: `game/classic/maps/luna_engineering.ron`.
Tests: `tools/dmm-import/src/lib.rs`, `game/classic/tests/luna_map.rs`.

```sh
cargo run -p rs13-dmm-import -- ../luna/maps/NSV_Luna.dmm \
    game/classic/maps/luna_engineering.import.ron game/classic/maps/luna_engineering.ron
cargo run -p rs13-server -- --map luna-engineering
```

## How it works

**Reading the map:**
- A `.dmm` file is a dictionary of tile keys, each a list of atoms (type path plus the
  variables the map overrides), followed by one grid of keys per z level.
- NSV Luna's map has 8 levels of 90 × 149 tiles. Decks 1–4 are the station; engineering is
  on deck 2.

**Region:** the rules pick a z level and a block of columns and rows of the file's text,
counted from 0 with the top row first.

**Each tile becomes:**
- **Tile:** from its turf, by the longest matching type-path prefix.
- **Window:** a grille or any window on the tile makes it a window tile instead (solid, see-through).
- **Objects:** from the object rules, again by longest matching prefix.
  - An airlock's `req_access_txt` (Luna access numbers separated by `;`) becomes our access ids.
  - A cable's `icon_state` (`"1-2"`, with 0 for a knot) becomes its ends. Ends that lead up or down to another deck are dropped and reported.
  - A rule with `state` keeps the atom's `icon_state` (tables, whose edges the mappers set by hand), with the given default.
  - Any object with `level = 2` gets `above_floor: true`: Luna lays those pipes over the floor tiles, so they show (`floors.md`).
  - A rule can also place a container's contents loose on its tile (for containers we can't model yet).
- **Spawns:** a landmark rule makes the tile a spawn and can place things there.
- **Area:** from the area table.

**Output:**
- Our RON map format, with a generated legend. Plain tiles get memorable characters (space,
  `.`, `,`, `#`, `X`, `=`), and the rest come from a pool.
- Anything the rules don't map is skipped and listed at the top of the output, which makes
  it a to-do list for content.
- The tool checks that the result loads before writing it.

## The engineering section

**Region:** deck 2, columns 23–56 and rows 56–80, giving a 34 × 25 map. It holds:
- the engine hallway, control room, monitoring room and core;
- electrical storage;
- the Chief Engineer's office;
- bits of the surrounding maintenance.

**What came across:**
- **Tiles:** floors, plating, walls, reinforced and heat-shield walls (both drawn as reinforced), windows and space.
- **Airlocks:** with Luna's access, for example the Chief Engineer's office needs `construction`. External airlocks have no access in Luna either.
- **Power:**
  - APCs, SMES units and all of Luna's cabling, with its ends. Every APC but one shares a network with the engine's SMES units. The starboard maintenance APC's cable runs north out of the region, so it lives on its cell.
  - Air alarms.
- **Atmos piping:** pipes, bends and manifolds (insulated ones too), vents, scrubbers, connectors, valves and air tanks, all with Luna's directions. The supply and scrubber lines stay separate networks. The supply line is empty: Luna feeds it from deck 4 through a pipe between decks, outside the region, so the vents have nothing to give.
- **Items:** screwdrivers, wrenches, insulated gloves, breath masks, the engineering hardsuits (as our space suit and helmet) and oxygen tanks.
- **Toolboxes and utility belts:** real containers with Luna's tools, welding tools and cable coils inside, minus analyzers and scanners, which have no prototype yet (`storage.md`). A loose cable coil too.
- **Tables and racks:** plain and reinforced tables with their mapped edges, and racks (`tables.md`).
- **Fuel tank:** the high-volume fuel tank, as an ordinary one (`welding.md`).
- **Spawns:** Luna's five Engineer start points. Each has an engineer ID card on it, as a stand-in for job equipment.

**Skipped for now** (104 types, listed in the generated file):
- **Atmos machines without a prototype:** tanks of other gases, canisters, filters, injectors, and the engine's freezers and generator inputs. Pipes that joined them end there.
- **Furniture and structures:** closets and chairs.
- **Machines:** computers, vending machines, the engine itself, lights, cameras, firedoors, disposals.
- **Stairs and ladders:** multi-z.
- **Most items.**

## Limits

- The core floor (`floor/engine/vacuum`) starts with air, because every zone does.
- Luna's `req_access_txt` lists every access a door needs, but our doors open for any one. In this region every door needs a single access, so the result is the same.
- The map stops at the region's edge. Tiles beyond it count as solid, so corridors that leave the section end in an invisible wall.
- Open pipe ends don't leak, unlike in Luna.
