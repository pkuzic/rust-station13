# Pipes and atmospherics machinery (phase 1, part 2)

Design: `docs/ATMOSPHERICS.md` (section 4). Code: `game/classic/src/machines.rs`.

Formulas follow classic SS13 machinery:
- pump: `luna:code/ATMOSPHERICS/components/binary_devices/pump.dm`
- scrubber: `unary/vent_scrubber.dm`
- vent: the vent formula in `unary/vent_pump.dm`. Luna had commented that block out in favour of Aryn's pipes; we use the classic version.

Machines run every **40 ticks (2 s)**, like SS13's machinery cycle.

## Pipe networks

**Members** connect to members on the same tile and on the four neighbouring tiles. Each
member holds part of the network's gas, in proportion to its volume:

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
| **Gas pump** (facing a direction) | Moves gas from the network behind it to the network ahead until that reaches its target (300 kPa): `moles = Δp × V_out / (T_in × R)`. Click to switch on/off. |
| **Manual valve** | Open: part of the pipe. Closed: splits the pipe in two. Click to open/close. |
| **Air vent** (on a pipe) | Lets network gas into the room toward its target (101.325 kPa): `Δp = min(target − P_room, P_pipe − P_room)`, `moles = Δp × 2500 L / (T × R)`. Click to switch on/off. |
| **Air scrubber** (on a pipe) | Takes 120 L of the tile's air (SS13 `volume_rate`) and moves its CO₂, plasma and N₂O into the network. Click to switch on/off. |
| **Connector port** (on a pipe) | A canister on it can be connected with a wrench. |
| **Canister** | Click to open or close its release valve. While disconnected and open, it releases into the room toward 101.325 kPa, by the vent formula. Wrench it on or off a port. |

**Breach lockout.** Vents do not run while their zone is losing gas to space, either directly or through an open door into a leaking zone. This stands in for an air alarm's breach mode and keeps the station's air from pouring out of a hole.

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

- Power: machines always run.
- Pressure-rated pipes, leaks and bursts.
- Gas filters and mixers.
- Heat exchangers.
- Vents in siphon mode.
- Air alarm and console UIs.
- Directional pipe pieces: every pipe connects to every neighbour.
- Canisters of other gases.
- Pulling and dragging canisters.
