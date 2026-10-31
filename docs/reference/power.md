# Reference spec: electrical power

## What Luna does

Sources (all under `luna:code/`):
- network: `WorkInProgress/UnifiedCabling/*`, `modules/power/Cabling.dm`, `PowernetController.dm`, `Machinery.dm`, `MachineProcs.dm`
- devices: `modules/power/apc.dm`, `smes.dm`, `port_gen.dm`, `solar.dm`, `cell.dm`
- consumers and areas: `game/machinery/doors/airlock.dm`, `game/area/areas.dm`

The classic /tg/ powernet is gone. Luna uses Sukasa's "Unified Cable Network".

**Cycle.** Everything runs once per master-controller cycle (about 2.4 s). Nothing scales by
time: every "W" is an amount per cycle.

### Cables and networks
- **Joining:** cables have two ends, `D1-D2`; an end of 0 is a knot on the cable's own tile.
- **Networks:** a flood fill over cables; machines are leaves, and each machine belongs to at most one network.
- **Bugs:**
  - hand-laid cables join the network as if they pointed north;
  - cutting a stub can leave machines attached;
  - explosions never update networks;
  - **cables never shock.**

### Network each cycle
- Sources call `AddPower`; consumers call `AddLoad`.
- Machines see **last cycle's** surplus (`Power − Draw`).
- `PowerPerAPC` = last cycle's supply ÷ number of APCs.
- A SMES only loses what was actually used: `restore()` gives back the unused surplus.

### Sources

| Source | Output |
|---|---|
| PACMAN | 5 000 per cycle × output setting 1–4, burning plasma sheets |
| SMES | capacity 5 × 10⁶ charge units at 0.05 units per W, so 10⁸ W-cycles; output 30 000 per cycle by default (maximum 200 000); charges from its terminal's network when that has surplus |
| Solar panel | 1 500 × cos²(angle to the sun) |

### APC
- **Cell:** 2 500 units, starting at 95%. `CELLRATE` = 0.002 units per W.
- **Area:** the APC bills its area's usage per channel (equipment, lighting, environment) every cycle.

**Paying the bill:**
- If the grid has any surplus, or its share per APC covers the usage, the grid pays.
- Otherwise the cell makes up the difference.
- When the cell can't either, every channel turns off.
- With a positive surplus, the cell trickle-charges by up to 0.1% of its capacity per cycle.

**Channels by cell charge** (automatic settings; a manual "off" stays off):

| Cell charge | Equipment | Lighting | Environment |
|---|---|---|---|
| ≥ 30% | on | on | on |
| 15–30% | on | off | on |
| < 15% | off | off | on |
| empty, or no cell | off | off | off |

**Area power:** an area is powered on a channel when its APC's breaker is on, the APC isn't shorted, and the channel is on. **An area without an APC stays powered forever.**

### Consumers
- **Airlocks:**
  - Use 50 on equipment per open and per close.
  - Refuse to move without power, even by hand.
  - **A crowbar forces an unpowered closed door open.** A crowbar also closes an open door at any time unless it is bolted.
- **Lights:** 25 × brightness on the lighting channel, e.g. 200 for a tube.
- **Atmos machines:**
  - Vents run with no power at all.
  - Scrubbers and pumps stop without power but draw nothing.
  - `idle_power_usage` and `active_power_usage` exist but are never read.

## What we build (`game/classic/src/power.rs`)

Same mechanics, with the bugs fixed, in real units on our 2-second machine cycle. 1 W for
one cycle is 2 J.

| Item | Ours | From Luna |
|---|---|---|
| Cables | Join by their ends: an end joins cables pointing back and machines on the tile it points to; a knot joins its own tile (`cables.md`) | keep; laid and cut with coils and wirecutters |
| Network accounting | Supply this cycle is shared out this cycle; a SMES only loses what was used | Luna's one-cycle lag and `restore()` give the same totals |
| P.A.C.M.A.N. | 5 kW while burning plasma (`generators.md`); drawn on before SMES units | keep |
| SMES | 200 MJ (Luna's 5 × 10⁶ units at 40 J), output 30 kW and full by default; Luna's map settings (charge, capacity, input and output levels, charge mode, charging, online) are kept. Stops its output when empty, until turned on again | keep |
| SMES terminal | the underfloor terminal on the tile next to a SMES, facing it (Luna `power/terminal`), joins the cable on its tile. While its charge mode is auto, the SMES draws up to its input level from what is left over on that network each cycle | keep; Luna's few-cycle wait before it starts charging is left out |
| SMES window | an empty hand opens it: stored %, charging and its mode (toggle), input level, output on/off, output level (M, ±10000, ±1000, ±100, M; 0–200 kW) and output load. Lights show output, charging and charge level | keep (Luna `smes/interact`) |
| Luna's engine room | the generators share a network with the SMES bank's terminals. Most bank units start at a tenth charge with output 0, and the station's own SMES nearly empty, so most of the station runs on its APC cells until engineers start the engine and set up the SMES units | keep |
| APC cell | 2.5 MJ (2 500 units at 1 kJ each), starts at 95% | kept |
| Grid pays when it has enough this cycle | | keep |
| Cell covers any shortfall; if it can't, every channel goes off | | keep |
| Trickle charge | 0.1% of capacity per cycle while the grid has spare | keep |
| Channel thresholds and auto settings | as in the table above | keep |
| Area without an APC | always powered | keep |
| APC interface | An ID card with `engine_equip` access locks or unlocks it (cover shut). An empty hand opens the APC window: breaker, grid power (none/low/good), cell % and charging, each channel's load and mode, cover lock. Unlocked and in reach, the window switches the breaker, sets channels off/on/auto, turns cell charging on or off, and locks or unlocks the cover | keep (Luna `interact` and `Topic`); shielding and AI rows left out |
| APC cover and cell | With the cover lock off, a crowbar opens or shuts the cover; through the open cover an empty hand takes the cell (as a power cell item, with its charge) and a cell goes back in. Without a cell every channel goes off | keep |
| Cell charging | only while automatic charging is on (Luna `chargemode`) | keep |
| Power monitoring computer | dense; 250 W on the equipment channel; blue glow while powered, dark without. Clicked in reach, its window shows the grid's total supply and load last cycle and every APC: area, channel modes, load, cell % and charging | keep (Luna `power/monitor`); Luna lists every APC, as the unified network has one grid |
| Airlock | Equipment channel; 100 J per open or close; no power means no movement | keep |
| Crowbar on an unpowered closed door | forces it open; also closes an open door unless bolted | keep |
| Vents, scrubbers, pumps | Environment channel, 150 W each while on; stop without power | **changed**: Luna's vents ignore power and nothing draws; we follow /tg/ |
| Examine | APC: cell % and channel states. SMES: charge % | keep |
| Electrocution | none yet | Luna's cables never shock |

## Not yet

Solar panels, APC wires, hacking and emagging. Lights draw on the lighting channel (`lights.md`).
