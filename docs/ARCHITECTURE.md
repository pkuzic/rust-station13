# Architecture

## Crates

| Crate | Path | Owns |
|---|---|---|
| `rs13-core` | `engine/core` | Tick, stable entity ids, shared constants |
| `rs13-world` | `engine/world` | Chunked x/y/z tile grid, spatial types; later containment, visibility, collision |
| `rs13-simulation` | `engine/simulation` | Tick schedule ordering, solver contracts |
| `rs13-network` | `engine/network` | Protocol version, sessions, replication (Lightyear) |
| `rs13-content` | `engine/content` | Prototype ids, RON loading and validation |
| `rs13-i18n` | `engine/i18n` | Fluent localisation, noun lexicon with case/number/gender |
| `rs13-classic` | `game/classic` | Classic game rules, content, locales, maps |
| `rs13-server` | `apps/server` | Dedicated server binary (engine + classic) |
| `rs13-client` | `apps/client` | Universal player client (Bevy) |
| `rs13-hub` | `apps/hub` | Optional server directory |
| `rs13-headless` | `apps/headless` | Scenario runner for tests and benchmarks |

Planned but not created until needed: `engine/actions` (interaction lifecycle),
`engine/modding` (WASM host), `engine/persistence` (SQLite), `engine/ai` (NPC tasks),
`engine/diagnostics`.

## Dependency rules

- `engine/*` never depends on `game/*`. The engine knows nothing about SS13 specifics.
- `game/classic` depends only on engine crates.
- `apps/*` assemble engine and game crates; they hold no gameplay logic.
- The client never depends on `game/classic` gameplay code; it receives content and
  presentation data from the server. **Temporary exception:** the client links
  `rs13-classic` only to embed its locale files, until locales arrive through the
  server's asset manifest.

## Server model

- Authoritative fixed tick at `rs13_core::TICK_RATE_HZ` (20, to be benchmarked).
- Each tick runs `rs13_simulation::TickSet` in order: `Commands` → `Gameplay` →
  `Simulation` → `Publish`.
- Clients send intentions (move, use item on target). The server validates and applies
  them in `Commands`.

## Simulations

- Simulation systems use explicit contracts rather than each other's storage.
- Dense data (the atmos grid, power graphs) lives in solver-owned structures, not as one
  ECS entity per tile or cable. ECS components hold references into them.
- Atmos targets FEA-equivalent outcomes (see `docs/adr/0005-atmos-fea-equivalence.md`).

## World

- Tiles are addressed by `TilePos { x, y, z }`, grouped into 16×16 chunks per deck.
- `z` exists from day one so multi-z never needs a coordinate migration.

## Text

- No player-visible string is hard-coded. Everything goes through `rs13-i18n` message ids.
- The server sends message ids and arguments; each client formats them in its own locale.
  See `docs/LOCALIZATION.md`.
