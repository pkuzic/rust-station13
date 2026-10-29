# 0005: Atmos produces FEA-equivalent outcomes on a new solver

- Status: accepted
- Date: 2026-10-29

## Context

Luna uses FEA, the pre-ZAS SS13 atmos system (`luna:code/FEA/`): per-turf gas
mixtures, air groups that merge uniform regions, fire and superconduction. Players'
expectations (how fast a room spaces, how fires spread, what a canister does) come from
FEA's behaviour, including some quirks.

## Decision

- Implement a new solver in Rust. Do not port FEA's code structure.
- Match FEA's **outcomes**: the same gas maths and constants (including Luna's doubled
  `R_IDEAL_GAS_EQUATION`), the same per-tile volume, comparable spacing and fire timings.
- Write `docs/reference/atmos.md` describing FEA's behaviour first. Mark each quirk
  **keep** (affects gameplay that players rely on) or **fix** (unintended bug).
- Verify with scenario tests comparing our results to values measured in, or derived from,
  Luna.

## Consequences

- We are free to use data-oriented layouts and parallelism for performance.
- "Equivalent" needs measurable tolerances; these are set in the atmos spec.
