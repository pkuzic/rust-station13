# 0010: Fire cells

Status: accepted (2026-11-01); refines `0009-zone-level-combustion.md`

## Context

Burning per zone (ADR 0009) lights a room all at once or not at all. Luna's `obj/fire/New`
adds a flame's temperature to the air of the one tile it touches, so the zone-level
version gave a room a tile's share of that heat: enough for a small room, never for a
large one. `docs/ATMOSPHERICS.md` plans active local cells for this; a full cell layer for
leaks and gradients is a larger piece of work than fire needs.

## Decision

Active cells for fire only, inside `AtmosGrid` (`atmos/grid/fire_cells.rs`):

- A flame that lights a tile (Luna's conditions: over 250 °C, more than 0.5 mol of plasma
  and of oxygen in the tile's air) makes it a **fire cell**: one tile's air, taken out of
  its zone, whose volume shrinks by a tile. Its air warms by the flame's temperature.
- Each step a cell burns its own air with Luna's reaction, and trades gas and heat with
  neighbouring cells and with its zone through each side facing the zone's tiles (0.4 of
  the difference per second per side), never across walls, closed doors or panes.
- A cell that has burnt half a second lights each neighbouring tile of open floor whose air
  can catch.
- A cell that has stopped burning for 2 s and is within 30 K of its zone (or after 30 s)
  goes back into the zone, and every cell of a zone venting to space goes back at once.
  Letting pressure even out between cells instead (gas pushing out of a hot cell) was tried:
  it starves a burning tile of fuel, so fires died at once in a plasma-rich room. When
  zones are rebuilt (a wall breached, a floor changed), every cell goes back first.
- A zone hot enough still burns as a whole, as before.

## Consequences

- Fire starts where the flame is and spreads across a room of any size at about two tiles
  a second, while it has fuel; a thin haze of plasma doesn't sustain it.
- Gas is conserved exactly as it moves between cells and zones.
- A burning tile's air keeps its volume as it heats, so its pressure reads very high (a few
  thousand kPa) until it cools: the pressure picture shows fires as hot spots.
- Cells are a first piece of the local-cell layer; leaks and gradients can grow from it.
- The mixing rate is a balance: much faster and a burning tile loses its heat before it can
  sustain itself; much slower and fire stays put. It lives in one constant.
