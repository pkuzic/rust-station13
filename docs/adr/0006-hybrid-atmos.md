# 0006: Hybrid zone/cell atmos with real physics; deadly space

- Status: accepted (supersedes `0005-atmos-fea-equivalence.md`)
- Date: 2026-10-29

## Context

ADR 0005 assumed Luna's atmos was FEA. Tracing the code (`docs/reference/atmos.md`) showed
Luna actually runs Aryn's ZAS3 zones for gas, with FEA reduced to a near-static temperature.
It has no pressure damage, and an operator-precedence bug disables human cold damage.

The project owner decided not to copy either system. The target is the hybrid design in
`docs/ATMOSPHERICS.md`: ZAS-like bulk zones, LINDA-like local cells where detail matters,
and simplified real physics. Space must be realistic, robust and deadly.

## Decision

- **Zones:** airtight connected spaces each hold one gas mixture. Doors split zones; an open
  door is an **opening** between two zones, not a merge.
- **Real physics:** ideal gas law with the real R = 8.314 J/(mol·K). Luna's doubled R is
  dropped.
- **Flow through openings:** isentropic orifice flow, choked above the critical pressure
  ratio. The size of the opening sets the flow. One global gameplay scale keeps venting
  dramatic but survivable for a few seconds.
- **Conservation:** moles are conserved exactly, except gas lost to space. Energy moves with
  the gas. A venting volume cools adiabatically, so rooms that vent get cold.
- **Space:** vacuum at 2.7 K, an infinite sink.
- **Space wind:** pushes mobs and loose items toward openings in proportion to the flow.
- **Vacuum exposure:** suffocation through breathing, low- and high-pressure damage, and
  body cooling with cold damage. Thresholds and damage are inspired by /tg/station; each
  value is recorded in the reference spec.
- **Determinism:** fixed steps on the 20 TPS tick, no wall-clock time, and stable
  iteration order.
- **Local cells:** active cells (spec phase 2) come later. The solver API is shaped so a cell
  can take over part of a zone's volume.
- Luna remains the reference for non-atmos behaviour. For atmos, Luna's findings are
  background only.

## Consequences

- Gameplay feel differs from Luna's: decompression is directional, faster, and colder.
- The global flow scale and damage numbers need playtesting; they live in one place.
- Phase 1 is delivered in two parts: core air and vacuum first, then pipes and machinery.
