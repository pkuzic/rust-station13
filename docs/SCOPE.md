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

## Later milestones

Sequence from the master plan. Estimates are speculative and phases overlap.

| Phase | Exit criterion |
|---|---|
| Research | Reference spec (`GAME_REFERENCE.md`) and this executable skeleton |
| Engine foundation | Two clients moving in one room (**done**: `apps/headless/tests/two_clients_one_room.rs`) |
| Gameplay prototype | Recognisable classic interaction loop (**in progress**: hands, items, airlock, examine, names, speech, ID access and basic health/combat done) |
| Engineering | Functional engineering test chamber (the first milestone above). **In progress:** atmos zones, decompression, space wind, breathing and vacuum damage done; pipes and machinery next |
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
- 3D models or a Blender sprite pipeline. Sprites are 2D, starting from Luna's art.
- Declining player names. Character names are English-only.
