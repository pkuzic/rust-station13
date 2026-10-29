# 0002: Pin Bevy 0.20.1, Lightyear 0.31.0 and Rust 1.98.0

- Status: accepted
- Date: 2026-10-29

## Context

Bevy breaks its API every minor release, and Lightyear tracks one Bevy minor at a time.
Candidates on 2026-10-29:

| Bevy | Released | Lightyear for it |
|---|---|---|
| 0.19.1 | 2026-08-13 | 0.30.1 (2026-09-16) |
| 0.20.1 | 2026-10-28 (0.20.0 on 2026-10-08) | 0.31.0 (2026-10-21) |

## Decision

Use Bevy `=0.20.1` (client: full `bevy`; server: `bevy_ecs` / `bevy_app` only) and
Lightyear `=0.31.0`, pinned exactly in the workspace `Cargo.toml`. Pin the toolchain to
Rust 1.98.0 in `rust-toolchain.toml`.

Starting on 0.20 avoids an immediate migration. The pair was verified by building the
workspace with Lightyear's `client`, `server`, `replication`, `netcode` and `udp`
features.

## Consequences

- Bevy and Lightyear are upgraded together, in one PR, with an ADR update.
- Lightyear 0.31 is young; if it blocks progress, falling back to Bevy 0.19.1 +
  Lightyear 0.30.1 costs little while the code base is small.
- The server stays headless: it must not depend on the full `bevy` crate.
