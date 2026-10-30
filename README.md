# rust-station13

Classic Space Station 13 (circa 2012–2013, modelled on **NSV Luna**) on a new Rust engine.
Modern technology underneath, classic SS13 on the surface.

> Status: scaffold. Nothing is playable yet. The first target is one engineering chamber
> with two players, doors, power, pressure simulation and an interactable machine
> (see [docs/SCOPE.md](docs/SCOPE.md)).

## Layout

```
apps/       server, client, hub, headless (binaries)
engine/     reusable engine crates: core, world, simulation, network, content, i18n
game/       the classic game build (rules, content, locales, maps)
client-assets/  sprites and sounds (Luna-derived, CC-BY-SA 3.0)
sdk/  tools/  tests/
docs/       design docs and architecture decision records
```

## Building

Requires Rust 1.98 (pinned in `rust-toolchain.toml`). On Linux the client also needs
`libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev`.

```sh
cargo test --workspace
cargo run -p rs13-server                 # listens on UDP 13013
cargo run -p rs13-server -- --map luna-engineering  # NSV Luna's engineering section
cargo run -p rs13-client                 # connects to 127.0.0.1:13013
cargo run -p rs13-client -- 10.0.0.5:13013
cargo run -p rs13-headless -- --dir east # a bot that walks east and reports what it saw
```

In the client (`rs13-client [ADDR] [--lang en|ru] [--name "First Last"]`):

| Input | Action |
|---|---|
| WASD / arrows | Move (Shift held: the opposite of the run/walk button) |
| Left click | Use the active hand on something in reach: pick up, open/close a door |
| Shift + click | Examine |
| Q | Drop the active-hand item |
| X | Swap hands |
| Z, or click the active hand | Use the item in it (light or put out a welding tool) |
| E | Internals on/off (breath mask plus a gas tank) |
| HUD inventory slot | Put on the active-hand item, or with an empty hand take it off |
| HUD buttons | Swap hands, drop, intent, run/walk, internals (see `docs/reference/hud.md`) |
| T | Say something (Enter sends, Esc cancels) |
| 1 / 4 | Help / harm intent (a held item hits people on any intent; fists need harm) |
| F3 | Atmos debug overlay (pressure per tile) |

The EVA closet by the outer airlock has a space suit, a helmet, a breath mask and an
emergency oxygen tank. Suit up, then turn internals on before you open the door.

A screwdriver opens an airlock's maintenance panel. Then use the door to see its wires:
wirecutters cut or mend them, and a multitool pulses them. Wear the insulated gloves, or
the power wires may shock you (`docs/reference/wires.md`). A crowbar forces an unpowered airlock. An APC's main breaker needs an ID card with
`engine_equip` access in the active hand.

Toolboxes and utility belts hold tools: click one you carry with an empty hand to open
it, click a tool in it to take it out, and click it with a tool to put that in
(`docs/reference/storage.md`).

A welding tool lights with Z (or a click on the active hand). Lit, it welds a closed
airlock shut, and open again, for a little fuel; a fuel tank fills it while it is out
(`docs/reference/welding.md`).

Click a table or rack with an item to put it down there. A wrench takes them apart; a
reinforced table must be weakened with a welding tool first (`docs/reference/tables.md`).

On bare plating a cable coil lays power cable:
click the plating, or click a cable end to carry it on to your tile. Wirecutters cut a
cable, and the power behind the cut goes out (`docs/reference/cables.md`). Elsewhere, pry
up the floor tiles with a crowbar first: pipes and cables under them show once they are
gone, and floor tiles put back hide them again (`docs/reference/floors.md`).

Pressure meters on the pipes show the pressure on their dial. Click one from up to three
tiles away to read it (`docs/reference/meters.md`).

Pressure tanks and canisters of oxygen, nitrogen, CO2, plasma and N2O hold their own gas,
in Luna's colours. A canister's light shows how full it is (`docs/reference/gas-tanks.md`).

Each area has an air alarm on its wall. Click it to read the room's air. When the pressure
turns dangerous it seals the area: every powered door around it closes and bolts until the
air recovers. Standing next to it you can seal or unseal by hand and turn on panic fill
(`docs/reference/air-alarm.md`).

Graphics are NSV Luna's sprites (see `client-assets/`). `--screenshot PATH` saves the view a
few seconds after joining, then quits.

## Documentation

| File | Purpose |
|---|---|
| [VISION.md](docs/VISION.md) | Classic gameplay philosophy and creative north star |
| [SCOPE.md](docs/SCOPE.md) | Milestones, MVP boundary, non-goals |
| [ARCHITECTURE.md](docs/ARCHITECTURE.md) | Crates, ownership, data flow |
| [GAME_REFERENCE.md](docs/GAME_REFERENCE.md) | How NSV Luna is used as the behaviour reference |
| [ART_DIRECTION.md](docs/ART_DIRECTION.md) | Sprites, UI and palette rules |
| [LOCALIZATION.md](docs/LOCALIZATION.md) | RU/EN text, grammatical case and gender |
| [PROTOCOL.md](docs/PROTOCOL.md) | Network commands, replication, versions |
| [MODDING.md](docs/MODDING.md) | Mod classes and security model |
| [TESTING.md](docs/TESTING.md) | Tests, scenarios, benchmarks |
| [AGENTS.md](AGENTS.md) | Rules for coding agents and contributors |
| [docs/adr/](docs/adr/) | Architecture decision records |

## Licence

Code: [AGPL-3.0-or-later](LICENSE). Assets in `client-assets/` derived from NSV Luna and
its upstreams are CC-BY-SA 3.0; see [client-assets/README.md](client-assets/README.md) and
[client-assets/sprites/ATTRIBUTION.md](client-assets/sprites/ATTRIBUTION.md).
