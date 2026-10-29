# 0003: AGPL-3.0 code, Luna as behaviour reference, Luna art reused

- Status: accepted
- Date: 2026-10-29

## Context

NSV Luna (github.com/SS13/luna, commit `e13cf12`) is the reference build. Its repository
has no licence file. Its code descends from /tg/station and Baystation12 (GPL/AGPL); its
sprites and sounds come from the same lineage, which licenses assets under CC-BY-SA 3.0.

## Decision

- **Code** in this repository is AGPL-3.0-or-later.
- **Luna's code is a behaviour reference only.** It is read and described, never
  translated. Numeric constants and formulas may be used. Rules in
  `docs/GAME_REFERENCE.md`.
- **Luna's sprites and sounds are reused** as the starting asset pack in `client-assets/`,
  under CC-BY-SA 3.0 with attribution kept in `client-assets/ATTRIBUTION.md`.
- No 3D models or 3D-to-sprite pipeline; new art is hand-made 2D in the same style.

## Consequences

- Server operators who modify and host the server must publish their changes (AGPL).
- Any derivative of the Luna asset pack is also CC-BY-SA 3.0.
- Before the first import, check each `.dmi` against its upstream (/tg/, Bay12,
  Goonstation) where possible: Goonstation-origin art is CC-BY-NC-SA and must be listed
  separately or replaced.
