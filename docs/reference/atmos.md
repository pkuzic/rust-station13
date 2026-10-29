# Reference spec: atmosphere (what Luna actually runs)

Status: **background only.** We do not reproduce this model; see `docs/ATMOSPHERICS.md`
and `docs/adr/0006-hybrid-atmos.md`.

Luna compiles two air systems at once: FEA (`luna:code/FEA/`) and Aryn's ZAS3 zones
(`luna:code/Aryn/`). At runtime they split the job:

- **Gas amounts (O2, N2, CO2, plasma) belong to zones.** Every tile in a zone has the zone
  total divided by the number of tiles.
- **Temperature belongs to FEA** (tile or air group). In practice it barely changes; see below.
- FEA's group-to-group and group-to-space sharing is commented out ("aryn's zones already
  exchange air in masses", `luna:code/FEA/FEA_airgroup.dm:90-91`).

All times below assume BYOND `sleep()` counts deciseconds (BYOND reference). Luna's
`Ticklag.dm` comment assumes ticks instead; if that were true, every time would halve.

## Timing

| Process | Period | Source |
|---|---|---|
| Zone `Update()` (gas exchange, space loss, airflow) | 2.0 s, per zone, own loop | `Atmos-ZAS3-Zones.dm:183-185`, `vsc.zone_update_delay = 10`, `tick_multiplier = 2` |
| Master loop: `Life()` for every mob | ~2.4 s | `game/master_controller.dm` |
| `air_master.process()` (FEA leftovers, door connect/disconnect) | every 5th loop, ~12 s | `master_controller.dm:182-185` |
| Human breath | ≥ 4 s between breaths, so every 2nd `Life()`, ~4.8 s | `modules/mob/living/carbon/life.dm:11-14` |

## Reading and changing a tile's air

- `return_air()` on a simulated tile: copy the zone's per-tile gas into the FEA mixture and
  return it; temperature is FEA's (`FEA_turf_tile.dm:250-280`).
- `remove_air(n)`: removes proportionally and subtracts it from the zone (`:282-311`).
- `assume_air(gas)`: adds the gas to the zone, merges into FEA air with heat-weighted
  temperature (`:191-211`).
- Space tiles: 0 moles at 253.15 K (`TSPC`); gas given to space disappears
  (`FEA_turf_tile.dm:13-58`, `defines/turf.dm`).

## Zones

**Forming** (`Atmos-ZAS3-Zones.dm:18-26, 523-546`): flood fill through tiles that let air
pass in "group" mode. Doors never do, so **every door is a zone boundary**; the door tile
joins one adjacent zone. Initial gas: standard air (O2 21%, N2 79%, 51.99 mol per tile at
101.325 kPa, 293.15 K) times the tile count.

**Connections:** two zones are connected through each border where air can currently pass
(an open door). Doors re-evaluate only on the next `air_master.process()`, so **a door
opening or closing takes effect on air up to ~12 s later**. Zones joined only through
doors never merge.

**Each zone update (every 2 s):**

1. **Space:** if the zone touches space, or is connected to a zone that does:
   `gas = round(gas / 1.2, 0.0001)` for each gas, i.e. **83.3% kept per 2 s**. Temperature
   is not changed. Then space airflow (below).
2. **Merge:** a door-less neighbour zone whose per-tile values all differ by less than
   0.2 mol is merged into this one.
3. **Mix with each connected zone:**
   ```
   flow = max(90, 15 × border_tiles) %
   avg  = (gas_A + gas_B) / (tiles_A + tiles_B)          // per-tile average
   per_tile_A = (per_tile_A − avg) × (1 − flow/100) + avg  // same for B
   ```
   So each side moves **at least 90% of the way to the shared per-tile average** per step,
   and each pair is mixed by both zones' loops (~twice per 2 s). Then zone airflow.

**Pressure** used for airflow: `total_moles × 8.31 × 2 × 293.15 / volume`; zone
temperature is fixed at 293.15 K.

**Known Luna bugs:**
- More than 6 border tiles gives a negative factor (overshoot), and 14+ amplifies it.
- West-side flood fill reads the wrong tile (`:632`).
- `update_mixtures` can apply a delta twice.
- Mixing partly overwrites the same update's vacuum loss (ordering).

## Temperature (FEA)

Rooms in normal air groups never exchange heat with neighbours or space: their temperature
stays at 293.15 K unless fire or hot gas is merged in, or superconduction starts above
493.15 K. **A vented room keeps 293 K while its moles go to zero.** Heat capacity:
`CO2×30 + (O2+N2)×20 + plasma×200 + trace×specific heat`.

## Airflow (`luna:code/Aryn/Airflow.dm`)

**Between zones:**
- **Trigger:** `n = pressure_A − pressure_B` in kPa; nothing happens below 25.33 kPa. Then
  `n` is converted to percent of 1 atm.
- **What moves:** movables near a connection border (within view).
  - Mobs and dense objects move at `n ≥ 60`.
  - Items by size class: w_class 2 at 45, 3 at 75, 4–5 at 95; w_class 1 moves at any `n`.
- **Knockdown:** `n > 200` weakens humans for 2 (5 for other mobs), unless magboots, buckled,
  or an airflow-protective suit.
- **Direction:** objects in A are pulled toward the connection; objects in B are pushed away.

**Toward space:** `n = (O2 + N2 + CO2 per tile) × 2`. Standard air gives ~104: everything
moves toward the breach, no knockdown. Only zones that touch space directly.

**Being thrown:**
- **Speed:** `min(n, 9)`, decaying 0.5 per step.
- **Hitting something:** brute `speed × 5 × 0.1` to head and chest.
- **Stun:** `speed > 10` → paralysis `speed × 1.5`; otherwise stunned `speed × 0.75`.

## Breathing (humans, `carbon/life.dm:456-562`, `NewMobs and etc/human_life.dm:630-722`)

**One breath:** 0.0002 of the tile's moles (0.5 L of 2500 L). The exhaled breath goes back
to the tile.

| Condition (breath partial pressure) | Per breath |
|---|---|
| No breath at all (skipped) | oxyloss +28 if health > 0, else +4 |
| O2 = 0 (vacuum) | oxyloss +14 |
| 0 < O2 < 16 kPa | oxyloss + min(5 × 16 / O2, 7) |
| O2 ≥ 16 kPa | oxyloss − 5 (heal), O2 used = O2 / 6 → becomes CO2 |
| CO2 > 10 kPa | after 12 s: paralysis 3, oxyloss +6; after 30 s: +16 more |
| plasma > 0.5 kPa | toxloss + min(plasma moles in breath / 0.5 × 3, 30), hallucination |
| oxyloss > 10 | losebreath +1 |

**In crit** (health < 0): losebreath +1 per breath. Above 10, breaths are skipped (+4
oxyloss each). Good air heals 5 oxyloss per breath until then. oxyloss > 60 means
paralysis.

**No pressure damage.** `handle_environment` ends with `return //TODO: DEFERRED`
(`carbon/life.dm:229-230`).

**No cold or heat damage for humans:** an operator-precedence bug makes the check always
false.

## Implementation target

Not this. The project follows `docs/ATMOSPHERICS.md` (hybrid zones and cells, real
physics, deadly space) instead; see ADR 0006.
