# Scope

## First milestone: engineering chamber

One small room, two players, playable end to end:

- Headless server at a fixed 20 TPS loading a small map (later imported from Luna `.dmm`).
- Two clients connect by direct address and move with server-authoritative movement.
- Hands and inventory: pick up, drop, hold a tool.
- One airlock that opens, closes, bolts, and needs power.
- A powernet: one power source, cable, one APC powering the room.
- Atmos with FEA-equivalent outcomes: pressure, a breach to space, an oxygen tank.
- One interactable machine (e.g. an air alarm or space heater).
- All player-visible text through `rs13-i18n`, in Russian and English.
- Regression scenarios for the airlock, power and the breach in `tests/`.

**Exit criterion:** two players in the chamber can open the airlock, cut its power, cause a
breach and watch the pressure drop, in either language.

**Acceptance:** `game/classic/tests/milestone.rs` plays this story with two players, using
the engineer's ID, EVA gear, wirecutters, a crowbar and the air alarm.

## Later milestones

Sequence from the master plan. Estimates are speculative and phases overlap.

| Phase | Exit criterion |
|---|---|
| Research | Reference spec (`GAME_REFERENCE.md`) and this executable skeleton |
| Engine foundation | Two clients moving in one room (**done**: `apps/headless/tests/two_clients_one_room.rs`) |
| Gameplay prototype | Recognisable classic interaction loop (**in progress**: hands, items, airlock, examine, names, speech, ID access, basic health/combat, first aid (bruise packs, ointment, health analyzers, first aid kits), containers (toolboxes, belts), and tables and racks done) |
| Engineering | Functional engineering test chamber (the first milestone above). **Exit criterion met** (`game/classic/tests/milestone.rs` plays the whole story; the network path is covered by `apps/headless/tests/two_clients_one_room.rs`). Luna's own engineering section is imported from `NSV_Luna.dmm` (`docs/reference/map-import.md`), with its directional atmos piping and cabling. Done: atmos zones, decompression, space wind, breathing, vacuum damage, pipes and machinery done (atmos phase 1); powernet with SMES, cables, per-area APCs and powered doors and atmos machines done; laying and cutting cable with coils and wirecutters done; prying up floor tiles with a crowbar, with pipes and cables hidden under them, done; welding tools, fuel tanks and welding airlocks shut done; pipe pressure meters done; pressure tanks and canisters of oxygen, nitrogen, CO2, plasma and N2O done; lights, tubes and light switches on the lighting channel done; soft coloured tile lighting with shadows, ready for several decks, done; flashlights, lanterns and desk lamps done; engineering gear (magnetic boots against space wind, hard hats with lamps, gas masks, firesuits, black gloves) done; closets, engineering lockers and crates done; border firedoors that seal one edge for air and movement, closed by air alarm seals and fire alarms, done; fire alarms (pull, heat detection, wirecutters) done; P.A.C.M.A.N. portable generators burning plasma sheets done; tank storage units done; power cells and cell chargers done; jetpacks for moving in open space done; radios and intercoms on the common channel done; chairs to buckle into done; the APC window, interface and cover locks, and swapping APC cells done; gas filters and outlet injectors done; glasses and ears slots, meson scanners and engineering headsets done; airlock maintenance panel and wires done (cut a door's power, bolt it, electrify it); air alarms with emergency seals and panic fill done |
| Modding and ecosystem | Custom server join with new assets and gameplay |
| Station alpha | Full 1–15 player round on NSV Luna |

## Explicitly deferred

- WASM gameplay scripting (after the engineering chamber works).
- Hub accounts and verified identities.
- LLM-driven NPC dialogue.
- Persistence across restarts.
- Multi-z gameplay (coordinates carry `z` from day one; stairs and lifts come later).

## Non-goals

- A BYOND clone or a Dream Maker interpreter.
- Translating Luna's DM code. Luna is a behaviour reference only.
- 3D models or a Blender sprite pipeline. Sprites are 2D, from Luna's art (imported).
- Declining player names. Character names are English-only.
