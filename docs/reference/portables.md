# Portable air pumps and scrubbers

Sources: `luna:code/game/machinery/atmoalter/portable_atmospherics.dm` (`connect`,
`disconnect`, `attackby`), `pump.dm` and `scrubber.dm` (`process`, `update_icon`);
`NSV_Luna.dmm`: 18 pumps and 17 scrubbers.

Code: `game/classic/src/portables.rs`, the window in `game/classic/src/gas_panel.rs` and
`apps/client/src/gas_panel.rs`. Tests: `game/classic/tests/portables.rs`,
`game/classic/tests/gas_panel.rs`.

Both are dense, wheeled about like canisters (pull them), start off with an empty tank at
20 °C, and a wrench fastens them onto the connector port under them, or off again
("You connect the portable air pump to the port."). On a port their tank is part of the
pipes, as a canister's is, and they don't work the room.

## Portable air pump (`portable-pump`)

A 1 000 L tank. While on and off a port, each machine cycle it siphons the air of its tile
(or of the tank held in it) into its own tank, by Luna's pump formula, toward its target in
its tank; turned out, it releases its tank toward the target in the room (or the held
tank). It starts siphoning toward 100 kPa. Sprite: `psiphon:0` or `psiphon:1`, with `siphon-connector` on a port.

## Portable air scrubber (`portable-scrubber`)

A 750 L tank. While on and off a port, each machine cycle it takes the plasma, CO₂ and
N₂O out of its tile's share of the room's air, or of the tank held in it (Luna samples
min(1, rate / volume × 5) of it: at the starting 1 800 L, all of a tile), into its tank,
leaving the rest.
Sprite: `pscrubber:0` or `pscrubber:1`.

## The window

Clicking one (with anything but a wrench or a gas tank) opens its window, as Luna's
`attack_hand` does; it closes when you walk away. It shows:

- its pressure, and whether it is on a port;
- the gas tank held in it, with its pressure and **Remove tank** (it drops on the floor);
- **Power switch** (a canister: **Release valve**), and for a pump **Pump direction** (in or
  out);
- **Target pressure** (a canister: **Release pressure**) with Luna's −/+ 1 000, 100, 10 and
  1 kPa buttons, 0 to 10 atm (a canister's 0.1 to 10 atm); a scrubber's **Rate**, ±10 and
  ±1 L, kept within 0 and 1 013 as Luna's own clamp does.

**A held tank:** clicking one with a gas tank in the hand puts the tank in, if none is there.
It then works on the tank's gas instead of the room's: a canister fills it up to its release
pressure, a pump siphons it or fills it, a scrubber cleans it.

## Differences from Luna

| Luna | Here |
|---|---|
| On a port, they keep pumping between the room and their tank, which the pipes share | On a port their tank is the pipes'; they don't work the room |
| An analyzer reads out their tank | Examining shows its pressure, as for anything holding gas |
