# Instructions for coding agents and contributors

Read before starting any task: `docs/VISION.md`, `docs/SCOPE.md`,
`docs/ARCHITECTURE.md`, and the docs for the area you are touching.

## Hard rules

1. **No Luna code in this repository.** Luna (`../luna`, commit `e13cf12`) is a
   behaviour reference. Read it, describe the behaviour, and implement it fresh. Never
   translate DM line by line. See `docs/GAME_REFERENCE.md`.
2. **No hard-coded player-visible text.** Use `rs13-i18n` message ids and add the
   message to every locale (`ru` and `en`). Nouns go in `nouns.ron` with all Russian
   cases. See `docs/LOCALIZATION.md`.
3. **Engine never depends on game.** `engine/*` must not reference `game/*`.
4. **Server authority.** Clients send intentions; all gameplay state is decided on the
   server.
5. **No unreviewed shared-API changes.** If your task needs a change to another
   crate's public API, stop and describe the change in your summary instead.
6. **Pinned versions.** Do not change Bevy, Lightyear or the Rust toolchain version
   without an ADR.
7. **No 3D.** Art is 2D, starting from Luna's sprites.

## Working on a task

- Each task has a scope, owned paths and acceptance tests. Stay inside the owned paths.
- Work on your own branch or worktree.
- Before handing back, run:
  ```sh
  cargo fmt --all
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace
  ```
- Test what you build end to end, not only with unit tests: drive the real behaviour
  before handing back. Tools for that:
  - `apps/headless`: bots over UDP against a real server, with a `script` of steps (walk,
    click a sprite, any action, wait, snapshot what was received); see
    `apps/headless/tests/sight_and_storage.rs`. From the command line, `rs13-headless
    --use SPRITE --wait SECONDS …` runs a short script (pick up and use things) to set a
    scene up for a client screenshot, such as lighting a fire.
  - `rs13-server --map-file PATH`: a small map of your own for the case at hand.
  - `rs13-client --size 1400x900 --use DX,DY|pull:DX,DY|swap|held … --screenshot PATH`: clicks and
    keys a player would do, then a screenshot to look at.
  If a path can't be driven yet, build the means to drive it first.
- Deliver: code, tests, a short summary, known limits, and any interface changes.

## Conventions

- Crate names `rs13-<name>`; Rust edition 2024.
- `unsafe` is forbidden workspace-wide.
- Errors: `thiserror` in libraries; no `unwrap()` outside tests and `main`.
- Logging: `tracing`.
- Content in RON. Prototype ids are lowercase kebab-case (`oxygen-tank`) and double as
  noun ids in the lexicon.
- Significant decisions get an ADR in `docs/adr/` (`NNNN-short-title.md`).
- Commit messages: imperative summary line, body explaining why.
