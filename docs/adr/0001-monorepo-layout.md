# 0001: One repository, one Cargo workspace

- Status: accepted
- Date: 2026-10-29

## Context

The project has a server, a client, a hub, an SDK and tools. They share the protocol,
content schemas, the asset manifest format and localisation. Work is done by several
coding agents in parallel.

## Decision

Keep everything in one repository and one Cargo workspace:
`apps/{server,client,hub,headless}`, `engine/*`, `game/classic`, `client-assets`, `sdk`,
`tools`, `tests`, `docs`. Create engine crates only when a task needs them, not up front.

## Consequences

- A protocol change and all of its users land in one reviewed commit; no version drift.
- One CI pipeline, one lockfile.
- Possible future splits: `client-assets` to Git LFS or its own repo when it grows; `sdk`
  published separately when outside modders need a stable API.
