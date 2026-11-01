# Importing maps from Luna

Tool: `tools/dmm-import` (`rs13-dmm-import`).
Rules: `game/classic/maps/luna_common.import.ron` (what each Luna type becomes, shared) and
one file per region: `luna_engineering.import.ron`, `luna_toxins.import.ron`,
`luna_incinerator.import.ron`.
Output: `game/classic/maps/luna_<region>.ron`.
Tests: `tools/dmm-import/src/lib.rs`, `game/classic/tests/luna_map.rs`,
`game/classic/tests/luna_rooms.rs`.

```sh
cargo run -p rs13-dmm-import -- ../luna/maps/NSV_Luna.dmm \
    game/classic/maps/luna_engineering.import.ron game/classic/maps/luna_engineering.ron
cargo run -p rs13-server -- --map luna-engineering   # or luna-toxins, luna-incinerator
```

**Rules files:** a region's rules name the shared file as their `base`; its turf, window,
object and ignore rules, areas and access numbers come first and the region's own are added
(where both name the same area or access number, the region's win). The region gives where
to cut, its start points and its areas.

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
- **Grille:** a grille on the tile makes it a grille tile: solid and see-through, but air
  goes through it (Luna's grilles let air pass; `airtight: false`).
- **Window panes:** Luna's windows are panes along one edge of their tile; each becomes a
  `window-pane` object facing that edge, sealing it against people and air
  (`windows.md`). A Luna window is a grille tile with its panes, sealed where Luna's is.
- **Objects:** from the object rules, again by longest matching prefix.
  - An airlock's `req_access_txt` (Luna access numbers separated by `;`) becomes our access ids.
  - A cable's `icon_state` (`"1-2"`, with 0 for a knot) becomes its ends. Ends that lead up or down to another deck are dropped and reported.
  - A rule with `state` keeps the atom's `icon_state` (tables, whose edges the mappers set by hand), with the given default.
  - A rule with `type_vars` supplies variables the Luna type sets itself and the map doesn't repeat, such as the wall lockers' facing and offsets per subtype; the map's own values win.
  - Any object with `level = 2` gets `above_floor: true`: Luna lays those pipes over the floor tiles, so they show (`floors.md`).
  - A rule can also place a container's contents loose on its tile (for containers we can't model yet).
- **Spawns:** a landmark rule makes the tile a spawn and can place things there. A rule
  with `name` takes only landmarks of that name (Luna's job start points: `"Scientist"`).
  The first matching rule wins. `spawn_at` adds start points by column and row, for regions
  without landmarks.
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
- **Atmos piping:** pipes, bends and manifolds (insulated ones too), vents, scrubbers, connectors, valves, air tanks and Luna's oxygen, nitrogen and CO2 pressure tanks, all with Luna's directions; pressure meters (`meters.md`); and the oxygen and nitrogen canisters (`gas-tanks.md`). The supply and scrubber lines stay separate networks. The supply line is empty: Luna feeds it from deck 4 through a pipe between decks, outside the region, so the vents have nothing to give.
- **Items:** screwdrivers, wrenches, insulated gloves, breath masks, the engineering hardsuits (as our space suit and helmet) and oxygen tanks.
- **Toolboxes and utility belts:** real containers with Luna's tools, welding tools and cable coils inside, minus analyzers and scanners, which have no prototype yet (`storage.md`). A loose cable coil too.
- **Tables and racks:** plain and reinforced tables with their mapped edges, and racks (`tables.md`).
- **Fuel tank:** the high-volume fuel tank, as an ordinary one (`welding.md`).
- **Lights:** tube and small light fixtures facing their walls, light switches, and boxes of spare tubes and bulbs (`lights.md`).
- **Closets:** the emergency and tool closets, the engineering lockers and the internals, radiation and RCD crates, with what Luna puts in them (`closets.md`).
- **Firedoors:** Luna's border firedoors. None of them sets a direction on Luna's map, so all keep the default and seal the south edge of their tile (`firedoors.md`).
- **Spawns:** Luna's five Engineer start points. Each has an engineer ID card on it, as a stand-in for job equipment.

**Wall-mounted devices:** Luna puts intercoms, alarms, APCs and the like on the floor tile beside a wall and shifts their sprite onto it with `pixel_x` and `pixel_y`. Every object keeps those (`vars`), and the client draws it shifted (`PixelOffset`); it still belongs to its floor tile. APCs store only their facing: Luna works their 24-pixel offset out from it, and so do we (`objects::spawn_object`). Air alarms and fire alarms keep their facing too, for their side-on sprites.

**Skipped for now** (83 types, listed in the generated file):
- **Atmos machines without a prototype:** filters, injectors, and the engine's freezers and generator inputs. Pipes that joined them end there.
- **Furniture and structures:** chairs.
- **Machines:** computers, vending machines, the engine itself, cameras, disposals.
- **Stairs and ladders:** multi-z.
- **Most items.**

## Limits

- The core floor (`floor/engine/vacuum`) starts with air, because every zone does.
- Luna's `req_access_txt` lists every access a door needs, but our doors open for any one. In this region every door needs a single access, so the result is the same.
- The map stops at the region's edge. Tiles beyond it count as solid, so corridors that leave the section end in an invisible wall.
- Open pipe ends don't leak, unlike in Luna.

## The toxins lab

**Region:** deck 3, columns 54–77 and rows 88–107, giving a 24 × 20 map: the toxins lab
and its storage, the burn chamber, the science security checkpoint and the hallways
outside.

**What came across:** as engineering, and:
- **The burn chamber:** its floor igniter and the lab's ignition switch (`toxins`), and the
  three blast doors that vent it to space, worked by the "Chamber Vent Control" button
  (`toxinsgasexp`; `igniters.md`, `blast-doors.md`). The chamber is its own sealed room of
  air.
- **Spawns:** Luna's three Scientist start points, each with a scientist's ID card (Luna's
  Scientist access: `tox`, `tox_storage`, `maint_tunnels`, `medlab`, `laboratories_doors`).
- **Skipped:** the toxins bomb parts (transfer valves, timers, igniter assemblies), the gas
  mixer and air control computer, the mass driver, vending machines, disposals, monkeys,
  the window doors (their tiles are open floor), and empty canisters.

## The incinerator

**Region:** deck 4, columns 39–50 and rows 111–121, giving a 12 × 11 map: disposal's
incinerator, its control room and the burn chamber.

**What came across:** its igniter and ignition switch (`Incinerator`), the shut vent and
input chute blast doors with their buttons, its piping, canisters and injector. Luna has no
start point here: one is set by the controls (`spawn_at`). Conveyors, disposals and the
digital valve are skipped.

## The whole station

**Regions:** all of decks 1–4, 90 × 149 each (`luna_deck1.import.ron` to
`luna_deck4.import.ron`), loaded together, bottom first, as one station of four decks
(`--map luna-station`; `multiz.md`).

**What came across:** everything the shared rules know (above), and:
- **Areas:** every area, named from its Luna path (`auto_areas`: `/area/toxins/lab` is
  `toxins-lab`).
- **Start points:** every job's, each with an ID card carrying Luna's access for that job
  (`get_access`: the Captain's has all of it); Luna's "Unassigned" and other start points
  just let players in. All of Luna's 44 access numbers have ids.
- **Ladders and stairs** between the decks; every one lands on a matching end on the next
  deck.
- **Turfs:** docked shuttles' floors and walls, walls the station can't touch, and open
  floor down to the deck below (`open-space`).
- **Access:** doors that name their access (`access_engine`) or carry stray control
  characters come across; Luna's `0` is no access.

**Skipped for now:** disposal pipes, cameras, computers, vending machines, the AI and most
items without a prototype, listed at the top of each deck's file. Cables that run between
decks lose that end.

