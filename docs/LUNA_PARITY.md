# Luna parity list

Everything NSV Luna has that this game doesn't yet, in the order it's being built. This is the working reference:
take the first open item, build it as one tested PR, and tick it here in that PR.

What counts as Luna is what `bs12.dme` compiles, not every `.dm` file on disk. Sizes are rough: S, M, L, XL.

- `[ ]` not started
- `[~]` partly done: the note says what's left
- `[x]` done: the note says what was deliberately left out, and the doc that lists differences

Each item names its main Luna source. Read it, describe the behaviour, and write it fresh (`AGENTS.md`).

## 1. A round you can play

| | Item | Size | Luna source | Notes |
|---|---|---|---|---|
| [x] | 1.1 Round flow: pre-game timer, mode choice, restart | L | `gamemodes/gameticker`, `game_mode`, `setupgame` | `round.md`. Secret mode waits for 2.x |
| [x] | 1.2 Lobby: character setup, job preferences, ready-up, late join | M | `mob/new_player` | `round.md`. Luna's saved slots, free colour picker, age, blood type and preview left out |
| [x] | 1.3 Crew manifest | S | `computer.dm` `datacore` | On the ID computer, printed too, `access.md`. PDAs' copy waits for 3.21 |
| [x] | 1.4 Escape pods: call, recall, launch, round end | M-L | `shuttle_controller` | `round.md`. Pods don't physically move; prison shuttle left out |
| [x] | 1.5 Communications console: alert levels, status displays | S-M | `computer/communications` | `round.md`. Message list waits for something to send messages; AI interface for 3.9 |
| [x] | 1.6 Objectives and the end-of-round report | S-M | `newobjective`, `intercept_report` | `traitor.md`. Intercept report left out |
| [x] | 1.7 Votes: restart and mode | S | `game/vote`, `datums/vote` | `round.md`. Custom votes wait for admins |
| [~] | 1.8 Traitor mode and uplink | M | `gamemodes/traitor`, `Abi79/uplinks` | `traitor.md`. Sells only what works: more as those systems arrive |
| [x] | 1.9 Chat: OOC, whisper, dead chat, emotes | M | `say`, `chat_tabs/ooc`, `Whisper`, `Me` | `speech.md`. Emotes with a target left out |
| [x] | 1.10 Radio channels and department headsets | M | `items/devices/radio` | `radio.md`. Swapping keys and the binary key left out |
| [x] | 1.11 Combat: body part aiming, armour, disarm, grabs, knockouts | L | `NewMobs/Damagesystem`, `organs`, `human_defense` | `combat.md`. Limbs and blood are 3.3 |
| [x] | 1.12 Clothing effects: armour, heat, no-slip | M | `clothing/*` | `combat.md`. Masks filter nothing in Luna either |
| [x] | 1.13 Explosions: blasts, breakage, tank bombs, EMP | L | `explosion.dm`, `bomb`, `effect_system` | Blasts, bursting tanks, C4, EMP, sound, `explosions.md`. Tank bombs and grenades go with 3.20 |
| [x] | 1.14 Construction: walls, girders, windows, frames, circuit boards | XL | `grille`, `window`, `turf`, `buildandrepair`, `machine_frame` | `construction.md`. Machine frames go with 3.11 |
| [x] | 1.15 Admin: kick, ban, jump, spawn, adminhelp | L | `modules/admin`, `NewBan` | `admin.md`. The verbs that matter; job bans and the many niche ones left out |

## 2. Game modes

| | Item | Size | Luna source | Notes |
|---|---|---|---|---|
| [x] | 2.1 Extended and sandbox | S | `gamemodes/extended`, `sandbox` | `round.md`. Luna's list of offsets and counts for spawned items left out: one at your feet |
| [x] | 2.2 Nuke: team, bomb, disk, pinpointer, base | M | `gamemodes/nuclear` | `nuke.md`. Goods the closets lack wait on their systems |
| [x] | 2.3 Revolution and rp-revolution | M | `gamemodes/revolution` | `revolution.md`. No intercept report: Centcom's messages aren't built |
| [x] | 2.4 Changeling | L | `gamemodes/changeling` | `changeling.md`. No hallucinations yet, so that sting is left out |
| [ ] | 2.5 Malfunction | M | `gamemodes/malfunction` | Blocked: needs 3.9 AI |
| [x] | 2.6 Meteor | S-M | `gamemodes/meteor` | `meteor-blob.md` |
| [x] | 2.7 Blob | M | `gamemodes/blob` | `meteor-blob.md`. Probability 0 in Luna's config. Guns don't hurt it yet |
| [~] | 2.8 Monkey, takeover, restructuring, derelict, Extend-A-Traitormongous | M | `gamemodes/*` | Monkey, restructuring (`monkey-restructuring.md`), AutoTraitor (`autotraitor.md`) and derelict (`derelict.md`) built. Takeover needs 3.4 viruses, 3.5 powers and 3.9 AI |
| [~] | 2.9 Wizard items and spells | L | `spellbook`, `datums/spells` | Luna's mode datum isn't compiled. Spells built on what exists, with the admin's Give Spell and the mime's wall; the spellbook, magic and teleportation scrolls, staffs, scrying orb, hardsuit and wizard crate (`spells.md`). Blind, Mutate and the orb's X-ray too (`eyes.md`). Left: the spells that wait on gibbing, smoke and passing through walls |

## 3. Departments

| | Item | Size | Luna source | Notes |
|---|---|---|---|---|
| [ ] | 3.1 Cargo: orders, supply shuttle, points, crates | M | `supplyshuttle`, `supplypacks` | |
| [ ] | 3.2 Surgery and the operating table | M | `modules/surgery` | Table is scenery |
| [ ] | 3.3 Fractures, bleeding, limb loss, pain | L | `organs.dm`, `human_life` | |
| [ ] | 3.4 Viruses and PanD.E.M.I.C. | L | `datums/diseases` | |
| [~] | 3.5 Genetics: scanner console, injectors, powers | L | `dnaconsole_new`, `game/dna` | Genes, the DNA console, injectors, radiation, the monkey block, and hulk, X-ray, blindness, cold resistance, no breathing, regeneration, deafness, epilepsy, cough, twitch (`genetics.md`). Left: telekinesis, clumsiness and the rest under Not built |
| [ ] | 3.6 Chemistry: the other ~170 reagents and ~155 recipes, splash, grinder | XL | `modules/Chemistry` | ~30 reagents, 10 recipes now |
| [ ] | 3.7 Kitchen and bar: recipes, drinks, drink mixing, gibber | L | `machinery/microwave`, `WIP/food`, `gibber` | Donk-pockets only |
| [~] | 3.8 Botany: ~25 more plants, genes, mutation, compost, extractor | L | `ISaidNo Release/hydroponics` | 8 plants, `hydroponics.md` |
| [ ] | 3.9 AI: mob, laws, upload, camera eye, door control | L | `silicon/ai`, `ai_laws` | Off by default in Luna's config |
| [ ] | 3.10 Cyborgs: mob, modules, laws, chargers | L | `silicon/robot` | |
| [ ] | 3.11 Research: console, analyzer, protolathe, imprinter, servers | L | `modules/research` | |
| [ ] | 3.12 Mechs and fabricator | XL | `modules/mecha` | |
| [ ] | 3.13 Anomalies: artifacts, analyser, harvester | M | `research/anomaly` | |
| [ ] | 3.14 Mining and the Siberia outpost | M-L | `modules/mining` | |
| [ ] | 3.15 Bots: Beepsky, mulebot, others | L | `machinery/bots` | Navigation beacons exist |
| [ ] | 3.16 Aliens | L | `carbon/alien`, `objects/alien` | |
| [ ] | 3.17 Metroids | L | `carbon/metroid` | |
| [~] | 3.18 Critters and player monkeys | M | `ISaidNo Release/critters` | Monkeys exist |
| [ ] | 3.19 Forensics: prints, fibres, scanner, computer | M | `DetectiveWork` | Records exist |
| [ ] | 3.20 Devices: signallers, proximity, grenades, implants, mousetraps | M-L | `modules/assembly`, `implants`, `grenades` | Inert items exist |
| [ ] | 3.21 PDAs: messaging, cartridges | M-L | `PDA` | Inert item exists |

## 4. Depth in what exists

| | Item | Size | Luna source | Notes |
|---|---|---|---|---|
| [ ] | 4.1 Hacking: wires on APCs, vendors, autolathe; emag; signallers | L | `datums/wires` | Airlock wires only |
| [ ] | 4.2 Inventory: pockets, throwing, grabs, strip window | M | `hands`, `mob/inventory` | |
| [ ] | 4.3 Weapons: casings, other guns, accuracy, energy swords, clumsiness | M | `modules/projectiles`, `items/weapons` | `weapons.md` |
| [ ] | 4.4 Power: solars, trackers, turbine, supermatter meltdown | M | `modules/power` | Needs 1.13 for meltdown |
| [ ] | 4.5 Atmospherics: consoles, heat exchangers, pipe bursts, dispensers | M | `atmospherics` | |
| [ ] | 4.6 Machine windows that are only clicks: cryo, TEG, pumps, mixers, airlock control | M | various | |
| [ ] | 4.7 Custom names on objects | S | various | Morgue, arcade, windoors, cards |
| [ ] | 4.8 Paperwork: stamps, photocopier, folders, burning | S | `paperwork` | |
| [ ] | 4.9 Photography | S | `WIP/Cameras` | |
| [ ] | 4.10 Library: books, writer's desk | S | `head/books` | Bookcases are scenery |
| [ ] | 4.11 Random events: storms, leaks, radiation belt, epidemic | S-M | `WIP/Events` | |
| [ ] | 4.12 Teleporters | S-M | `teleporter` | |
| [ ] | 4.13 Sound: footsteps, ambience, volume | S-M | `game/sound` | |
| [ ] | 4.14 Verbs: the dozens not yet built | M | throughout | 5 now, `verbs.rs` |
| [x] | 4.15 Sinks, showers, toilets | S | `structures.dm`, `watercloset.dm` | `plumbing.md`, with mirrors. No blood or dirt yet to wash off |
| [ ] | 4.16 Remaining scenery behaviour: washers, spikes, holopads, sensors | M | `scenery.md` | Mirrors done with 4.15 |
| [ ] | 4.17 Mass drivers, cyborg chargers, remaining map objects | M | `zcount`: 106 kinds, 290 objects | |

## Not gaps

Present in Luna's folders but not compiled, or not in Luna at all, so not to be built for parity: cult, the wizard game
mode datum, simple animals (carp, corgi), pAI, MMI, ninja, zombie, CTF, deathmatch, `machinery/kitchen/*` other than
the monkey recycler, the `modules/hydroponics` folder (the `ISaidNo Release` version is the one compiled), biogenerator,
mint, mech bay, telecomms and the singularity engine.

## How this list was made

Line and keyword survey of `bs12.dme`'s includes against `game/classic/src`, `apps/*` and `docs/reference`, 2026-11-03,
with the largest absences confirmed by searching the code. Sizes are estimates, and small entries may be missing: add
them when found. `docs/reference/*` can be stale, so check the code before trusting a doc's "not built".
