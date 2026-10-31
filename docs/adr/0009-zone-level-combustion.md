# 0009: Zone-level combustion first

Status: accepted (2026-10-31)

## Context

`docs/ATMOSPHERICS.md` (ADR 0006) wants combustion in LINDA-style local cells that activate
around fires, so a fire can spread across a room tile by tile. Those cells don't exist yet:
gas is simulated per airtight zone. Luna's compiled fire object doesn't work (its burn
rounds to nothing), but its gas reaction does.

## Decision

Burn plasma per zone with Luna's reaction, as a rate per second, every atmos step. A zone
hotter than 250 °C with plasma and oxygen burns as a whole. Sparks and flames heat a 125 L
sample of their zone, as Luna's hotspot does. `Zone` gains `burn_rate` so the game can draw
fire, and the grid gains `expose` and `burning_at`.

## Consequences

- Fires work now: a welding tool in a plasma-filled room sets it alight, it heats up,
  burns its fuel and oxygen, and hot gas carries it through open doors.
- A room catches fire all at once rather than from one corner; local cells will refine
  this without changing the reaction.
- Rooms cool by trading heat with their walls, which are treated as a heat sink at 20 °C
  until materials get their own temperature (P1).
