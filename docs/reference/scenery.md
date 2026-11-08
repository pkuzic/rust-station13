# Scenery

Fixtures on `NSV_Luna.dmm` that nothing else needs yet. They stand where the map puts them, so the decks
look as they should and block what they should, and they can be examined. Clicking them does nothing.

Code: `game/classic/src/scenery.rs`. Tests: `game/classic/tests/scenery.rs`.

| Prototype | Luna type | Dense | Blocks the view |
|---|---|---|---|
| `shuttle-engine` | `machinery/shuttle/engine/propulsion` | yes | yes |
| `shuttle-engine-left` | `machinery/shuttle/engine/propulsion/burst/left` | yes | yes |
| `shuttle-engine-right` | `machinery/shuttle/engine/propulsion/burst/right` | yes | yes |
| `pod-engine` | `machinery/shuttle/engine/propulsion/pod` | yes | yes |
| `shuttle-heater` | `machinery/shuttle/engine/heater` | yes |  |
| `holopad` | `machinery/hologram_ai` |  |  |
| `plastic-flaps` | `structure/plasticflaps` | yes | yes |
| `noticeboard` | `structure/noticeboard` |  |  |
| `mop-bucket` | `structure/mopbucket` | yes |  |
| `sign-deathsposal` | `structure/sign/deathsposal` |  |  |
| `sign-nosmoking` | `structure/sign/nosmoking_1` |  |  |
| `sign-fire` | `structure/sign/fire` |  |  |
| `meat-spike` | `structure/kitchenspike` | yes |  |
| `display-case` | `structure/displaycase` | yes |  |
| `washer` | `machinery/washer` | yes |  |
| `bookcase` | `machinery/bookcase` | yes |  |
| `operating-table` | `machinery/optable` | yes |  |
| `laser-machine` | `machinery/party/lasermachine` |  |  |
| `turntable` | `machinery/party/turntable` |  |  |
| `air-sensor` | `machinery/air_sensor` |  |  |
| `airlock-sensor` | `machinery/airlock_sensor` |  |  |
| `doppler-array` | `machinery/doppler_array` | yes |  |
| `party-button` | `machinery/partyalarm` |  |  |
| `mining-laser` | `machinery/mininglaser` | yes |  |
| `scanner` | `machinery/scanner` | yes |  |

## Differences from Luna

These are real machines or structures in Luna, and each will need its own work to do what it does there:

| Luna | Here |
|---|---|
| Shuttle engines belong to a shuttle's flight | Shuttles don't fly yet |
| Bookcases hold books to read | No books |
| Plastic flaps let through someone lying down | They stop anyone walking, and nobody can crawl here yet. What a conveyor carries, what's pulled and shots do get through, and they block air and the view, as in Luna |
| The operating table holds someone for surgery | No surgery |
| Washers wash clothes and spikes hang meat | Nothing. Mirrors, sinks, showers and toilets are in `plumbing.md` |
| Holopads, doppler arrays, gas and airlock sensors, and party lights all do something | Nothing |

## Shuttle doors (`shuttle-door`)

Luna's `door/unpowered/shuttle`, which does have a use: click it to open or close it. It needs no power or
access, and has no panel or wires, so it can't be hacked or cut off. Code: `game/classic/src/doors.rs`.
Tests: `game/classic/tests/shuttle_doors.rs`.

It's the airlock's code, so it also opens when walked into and closes itself, where Luna's stays as left
and only opens on a click.
