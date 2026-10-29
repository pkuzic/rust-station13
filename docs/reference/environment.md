# Environment: air, bodies and space (phase 1, part 1)

Design: `docs/ATMOSPHERICS.md`, `docs/adr/0006-hybrid-atmos.md`.

Code:
- `engine/simulation/src/atmos/` (solver)
- `game/classic/src/atmos.rs` (doors, wind, drift)
- `game/classic/src/environment.rs` (bodies)

All tuning values live in those files. This page records them and where they came from.

## Air

| Quantity | Value | Note |
|---|---|---|
| Gas constant R | 8.314 J/(mol·K) | real |
| Tile volume | 2500 L (1 × 1 × 2.5 m) | SS13 `CELL_VOLUME` |
| Standard air | 101.325 kPa, 20 °C, 21% O₂ / 79% N₂ | about 104 mol per tile |
| Space | vacuum at 2.7 K | infinite sink |
| Heat capacities (J/mol·K) | O₂, N₂ 20; CO₂ 30; N₂O 40; plasma 200 | SS13 values |
| Door opening | 2.0 m², discharge coefficient 0.6 | |
| Breach (one tile face) | 2.5 m² | |
| Flow scale | 0.05 | gameplay; real flow would empty a room in about 1 s |

**Zones:**
- Flood-filled floor.
- Door tiles are never in a zone. An open door is an opening between the zones (or space) around it.
- Walls breached into floor rebuild the zones and hand the gas over in proportion to tiles.

**Flow:**
- Isentropic orifice flow, choked below the critical pressure ratio (about 0.53 for air).
- Limited to what equalises the two sides.
- The gas left behind cools isentropically. The receiving side gets exactly the energy that left, so moles and energy are conserved between zones.
- Gas sent into space is lost.

**Sleeping regions:** openings with less than 0.001 kPa difference are skipped. Sealed zones cost nothing.

### Measured on the engineering chamber

| Situation | Result |
|---|---|
| Closet (27 tiles), external airlock forced open | 101 → 32 kPa in 5 s; under 1 kPa in 30 s |
| Whole chamber, both doors open | about 33 kPa after 30 s; under 5 kPa in 3 min |

## Space wind and drift

**Wind:**
- Each tile of a zone losing gas blows one step toward the opening along the shortest path.
- Speed is the physical gas speed through the opening, `molar flow / (molar density × area)` (not slowed by the flow scale).
- It falls off as `v₀ / (1 + distance)`.

**What moves:**
- Loose items at ≥ 15 m/s; people at ≥ 30 m/s (a strong gale is about 25 m/s).
- One tile every `200 / speed` ticks, clamped to 2–20 ticks.

**Drift:**
- Anything blown onto a space tile keeps drifting in that direction, one tile per 4 ticks, until it reaches a non-space tile or the map edge.
- In space you can only move with something non-space in one of the 8 neighbouring tiles to push off from.

## Bodies

**Timing:** a life cycle every 40 ticks (2 s); a breath every second cycle (4 s).

### Breathing (rules from Luna's `handle_breath`)

**One breath** is 0.5 L of the tile's air. The exhaled breath goes back to the tile.

| Breath partial pressure | Effect per breath |
|---|---|
| O₂ = 0 | oxy +14 |
| 0 < O₂ < 16 kPa | oxy + min(5 × 16 / O₂, 7); uses a reduced share of O₂ |
| O₂ ≥ 16 kPa | oxy −5 (heals), losebreath −1; 1/6 of the O₂ becomes CO₂ |
| CO₂ > 10 kPa | after 12 s: oxy +6 per breath; after 30 s: +16 more |
| plasma > 0.5 kPa | tox + min(plasma mol / 0.5 × 3, 30) |
| oxy > 10 | losebreath +1 |

**In crit** (health < 0): losebreath +1 per breath. Above 10, the breath is missed: oxy +4.

**Differences from Luna:**
- Good air lowers losebreath; in Luna it never recovers.
- Breath size uses real pressure.

### Pressure, per life cycle (/tg/station values)

| Pressure | Effect |
|---|---|
| < 20 kPa | 4 brute, spread over the body; `VACUUM` alert |
| 20–50 kPa | `Low pressure` alert |
| > 325 kPa | `High pressure` alert |
| > 550 kPa | brute min((P / 550 − 1) × 4, 4) |

### Body temperature, per life cycle

1. **Air** pulls the body toward its own temperature by 5% of the difference per cycle at 1 atm. This scales with density and is capped at 2 atm.
2. **Vacuum** (< 1 kPa) removes 5 K per cycle. Real vacuum cooling is slow; this is exaggerated so space is deadly.
3. **Thermoregulation** then corrects up to 1.5 K toward 37 °C (310.15 K).

**Damage** (/tg/ values, burn spread over the body):

| Body temperature | Burn per cycle |
|---|---|
| < 260.15 K, above 200 K | 0.5 |
| 120–200 K | 1.5 |
| ≤ 120 K | 3 |
| > 360.15 K, below 400 K | 2 |
| 400–460 K | 3 |
| ≥ 460 K | 8 |

### Measured

| Situation | Result |
|---|---|
| Unprotected person in the venting closet | unconscious at 32 s, dead at 62 s |
| Normal air for 2 minutes | no damage, body at 37 °C |

## Not yet

- Internals and space suits.
- Local cells, fire, heat conduction between zones and surfaces.
- Door pressure ratings.
- Atmos alarm machines; the HUD alerts stand in for now.
