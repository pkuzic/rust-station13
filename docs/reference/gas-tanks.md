# Gas tanks and canisters

Sources: `luna:code/ATMOSPHERICS/pipes.dm` (`pipe/tank` and its gases) and
`luna:code/game/machinery/atmoalter/canister.dm` (the canister gases, `New`,
`update_icon`).

Code: `game/classic/src/machines.rs` (`GAS_TANKS`, `GAS_CANISTERS`,
`update_canister_looks`). Tests: `game/classic/tests/gas_tanks.rs`.

## Pressure tanks

Pressure tanks are pipe pieces filled with one gas. Each joins the pipe on the side it
faces, and nobody walks through it. A meter can sit on one (`meters.md`).

| Prototype | Luna type | Gas | Sprite |
|---|---|---|---|
| `oxygen-pressure-tank` | `pipe/tank/oxygen` | oxygen | blue |
| `nitrogen-pressure-tank` | `pipe/tank/nitrogen` | nitrogen | red |
| `carbon-dioxide-pressure-tank` | `pipe/tank/carbon_dioxide` | CO₂ | plain |
| `plasma-pressure-tank` | `pipe/tank/toxins` | plasma | orange |

- **Size:** each holds 2 620 L (Luna: 0.9 m by 0.9 m by 2 m).
- **Fill:** each starts at 70 atm and 20 °C (Luna: 2 × 35 atm).
- **Air tanks:** the air tank (`air-tank`) is unchanged (`pipes.md`). It now shows Luna's
  red sprite.

## Canisters

Canisters of one gas work like the air canister (`pipes.md`):

- click one to open or close its release valve;
- while it is open and off a port, it releases into the room toward one atmosphere;
- wrench it on or off a connector port.

| Prototype | Luna type | Gas | Colour |
|---|---|---|---|
| `canister` | `canister/air` | air | grey |
| `oxygen-canister` | `canister/oxygen` | oxygen | blue |
| `nitrogen-canister` | `canister/nitrogen` | nitrogen | red |
| `carbon-dioxide-canister` | `canister/carbon_dioxide` | CO₂ | black |
| `plasma-canister` | `canister/toxins` | plasma | orange |
| `nitrous-oxide-canister` | `canister/sleeping_agent` | N₂O | red and white |

- **Size and fill:** each holds 1 000 L and starts at 4 500 kPa (Luna: half of 90 atm).
- **The light:** each shows a gauge light for its pressure, as in Luna:

| Pressure | Light |
|---|---|
| below 10 kPa | `can-o0` |
| below one atmosphere | `can-o1` |
| below 15 atm | `can-o2` |
| above | `can-o3` |

- **The connector:** on a port, it shows the connector as well.
- **The valve:** the release valve doesn't change its look.

## Differences from Luna

- **Unconnected tanks:** a Luna tank with nothing on its side mixes its gas with the tile
  around it. Ours keeps it.
- **Air tank size:** the air tank keeps our larger storage tank's numbers (10 000 L at
  9 000 kPa), not Luna's 2 620 L at 70 atm.
- **Not implemented:**
  - the oxygen and plasma tank (`oxygen_agent_b`), and the empty yellow canister;
  - canister damage: bursting when overheated or shot, and the broken sprite;
  - the canister window: release pressure, labels, and holding a tank inside;
  - the open-tank overlay.
