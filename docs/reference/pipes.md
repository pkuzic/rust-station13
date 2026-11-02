# Pipes and atmospherics machinery (phase 1, part 2)

Design: `docs/ATMOSPHERICS.md` (section 4). Code: `game/classic/src/machines.rs`. Tests:
`game/classic/tests/filters.rs`, `game/classic/tests/mixers.rs` and others.

Formulas follow classic SS13 machinery:
- pump: `luna:code/ATMOSPHERICS/components/binary_devices/pump.dm`
- mixer: `trinary_devices/mixer.dm`
- scrubber: `unary/vent_scrubber.dm`
- vent: the vent formula in `unary/vent_pump.dm`. Luna had commented that block out in favour of Aryn's pipes; we use the classic version.

Machines run every **40 ticks (2 s)**, like SS13's machinery cycle.

## Pipe networks

**Connections:**
- Each member connects on some of its four sides, as in Luna's `initialize_directions`. It
  joins a member on the neighbouring tile that connects back, and a member on its own tile
  that shares a side.
- A placed piece takes its sides from its direction:

  | Piece | Sides |
  |---|---|
  | Straight pipe, valve | along its direction (N/S or E/W) |
  | Bent pipe (diagonal direction) | the two sides of the diagonal |
  | Manifold (105 L) | every side but the one it faces |
  | Vent, scrubber, connector, air tank | the side it faces |

- Without a direction, as in the hand-drawn chamber, a piece connects on every side.
- So Luna's supply and scrubber lines can run side by side, or cross on one tile, and stay
  apart.
- Vents, scrubbers and connectors are members with no volume of their own: they reach the
  network through the pipe they face.
- A pump is not a member. It takes from the member behind it that connects toward it, and
  gives to the one ahead that connects back.

**Gas:** each member holds part of the network's gas, in proportion to its volume:

| Member | Volume | Starts with |
|---|---|---|
| pipe | 70 L | vacuum |
| manual valve (open) | 70 L | vacuum |
| air tank | 10 000 L | air at 9 000 kPa (about 37 000 mol) |
| canister (on a connector port) | 1 000 L | air at 4 500 kPa |

- **One gas pool per network.** A network has a single mixture.
- **Rebuilding.** When its shape changes (a valve turns, a canister is wrenched on or off), the gas is first handed back to the members by volume, then pooled again per new network. Nothing is created or lost.
- **Cut-off members.** A closed valve or a disconnected canister keeps its own share.

## Machines

| Machine | What it does each cycle |
|---|---|
| **Gas pump** (facing a direction, Luna `binary/pump`) | Moves gas from the network behind it to the network ahead until that reaches its target: `moles = Δp × V_out / (T_in × R)`. Needs power. Luna's map sets whether it is on (`on`, else off) and its target (`target_pressure`, else 101.325 kPa); the hand-drawn chamber's pump is on at 300 kPa. Click to switch on/off. |
| **Manual valve** | Open: part of the pipe. Closed: splits the pipe in two. Click to open/close. |
| **Air vent** (on a pipe) | Lets network gas into the room toward its target (101.325 kPa): `Δp = min(target − P_room, P_pipe − P_room)`, `moles = Δp × 2500 L / (T × R)`. Click to switch on/off. |
| **Air scrubber** (on a pipe) | Takes 120 L of the tile's air (SS13 `volume_rate`) and moves its CO₂, plasma and N₂O into the network. Click to switch on/off. |
| **Pipe vent** (an open pipe end, Luna `pipe/vent`) | 250 L of the network's gas evens out with the zone it opens into, by volume (Luna `mingle_with_turf(loc, 250)`, which equalises with the whole turf group). In space the 250 L is lost. Always open; nothing to switch. |
| **Gas filter** (facing a direction, Luna `trinary/filter`) | Takes gas from the pipe behind it and sends it to the pipe ahead until that reaches its target (map `target_pressure`, default 101.325 kPa), by the pump formula. The filtered gas (map `filter_type`: 0 plasma, 1 O₂, 2 N₂, 3 CO₂, 4 N₂O) goes instead to the pipe a quarter turn clockwise. Needs all three pipes and power. Luna's "Plasma Toxins Filter" is set to 5, which takes nothing out; we keep that. Click to switch on/off. |
| **Gas mixer** (facing a direction, Luna `trinary/mixer`) | Fills the pipe ahead toward its target (map `target_pressure`, default 101.325 kPa) from the pipe behind it and the pipe a quarter turn clockwise, in the shares `node1_concentration` and `node2_concentration` (default half each): each gives its share of the pressure still missing, by the pump formula at its own temperature. If either can't give its whole share, both give the same fraction less. Needs all three pipes; like Luna's, it runs without power. Dense. Click to switch on/off. |
| **Outlet injector** (on a pipe, Luna `unary/outlet_injector`) | While on, lets out the gas that fills `volume_rate` litres (map setting, default 50) at the network's pressure into the room. No power needed. Click to switch on/off. |
| **Connector port** (on a pipe) | A canister on it can be connected with a wrench. |
| **Canister** | Click to open or close its release valve. While disconnected and open, it releases into the room toward 101.325 kPa, by the vent formula. Wrench it on or off a port. Canisters of one gas, and pressure tanks of one gas: `gas-tanks.md`. |

**Breach lockout.** Vents do not run while their zone is losing gas to space, either directly or through an open door into a leaking zone. This keeps the station's air from pouring out of a hole. Panic fill from an air alarm overrides it (`air-alarm.md`).

**Feedback.** Examining anything that holds gas also reads its pressure gauge. Switching machines on and off gives a localized notice.

## The engineering chamber

**North wall:** an air tank feeds a pump into the distribution pipe, which runs along the wall and through it into the closet.

**On the distribution pipe:**
- two vents in the room and one in the closet;
- a canister on a connector port;
- a manual valve.

**South:** a scrubber with its own short waste pipe.

### Measured

| Situation | Result |
|---|---|
| Closet vented through the external airlock for 30 s, then closed | back above 95 kPa after **148 s**, then approaches 101.3 kPa without overshooting |
| Pump into an empty distribution pipe | settles at 300 kPa |

## Not yet

- Pressure-rated pipes, leaks and bursts.
- Filters, mixers and injectors use our pump formula on the whole output network; Luna's use their own 200 L (300 L for a mixer's outlet), so they move gas more slowly there.
- The pump's and mixer's windows (Luna's power link, output pressure and shares): only switching them on and off is built.
- Radio control of injectors (Luna's `frequency` and `id`).
- Heat exchangers.
- Vents in siphon mode.
- Console UIs, and air alarm control of vent and scrubber settings.
- Canister pressure and damage, labels, and holding a tank in a canister.
- Pulling and dragging canisters.
