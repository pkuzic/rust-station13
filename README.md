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
cargo run -p rs13-client                 # connects to 127.0.0.1:13013
cargo run -p rs13-client -- 10.0.0.5:13013
cargo run -p rs13-headless -- --dir east # a bot that walks east and reports what it saw
```

In the client (`rs13-client [ADDR] [--lang en|ru]`):

| Input | Action |
|---|---|
| WASD / arrows | Move (hold Shift to walk) |
| Left click | Use the active hand on something in reach: pick up, open/close a door |
| Shift + click | Examine |
| Q | Drop the active-hand item |
| X | Swap hands |

Graphics are placeholder squares until the Luna sprites are imported.

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
its upstreams are CC-BY-SA 3.0; see [client-assets/README.md](client-assets/README.md).
