# Portable air pumps and scrubbers

Sources: `luna:code/game/machinery/atmoalter/portable_atmospherics.dm` (`connect`,
`disconnect`, `attackby`), `pump.dm` and `scrubber.dm` (`process`, `update_icon`);
`NSV_Luna.dmm`: 18 pumps and 17 scrubbers.

Code: `game/classic/src/portables.rs`. Tests: `game/classic/tests/portables.rs`.

Both are dense, wheeled about like canisters (pull them), start off with an empty tank at
20 °C, and a wrench fastens them onto the connector port under them, or off again
("You connect the portable air pump to the port."). On a port their tank is part of the
pipes, as a canister's is, and they don't work the room.

## Portable air pump (`portable-pump`)

A 1 000 L tank. Clicking it switches it on or off. While on and off a port, each machine
cycle it siphons the air of its tile into its tank, by Luna's pump formula, toward
100 kPa in the tank. Sprite: `psiphon:0` or `psiphon:1`, with `siphon-connector` on a port.

## Portable air scrubber (`portable-scrubber`)

A 750 L tank. Clicking it switches it on or off. While on and off a port, each machine
cycle it takes the plasma, CO₂ and N₂O out of its tile's share of the room's air (Luna
samples min(1, 1 800 / volume × 5) of it: all of it) into its tank, leaving the rest.
Sprite: `pscrubber:0` or `pscrubber:1`.

## Differences from Luna

| Luna | Here |
|---|---|
| A window: power, the pump's direction (siphon or release) and target pressure, the scrubber's rate, and a held tank | Not built: a click switches them on or off; a pump only siphons, toward 100 kPa |
| A gas tank put in one becomes what it pumps or scrubs | Not built |
| On a port, they keep pumping between the room and their tank, which the pipes share | On a port their tank is the pipes'; they don't work the room |
| An analyzer reads out their tank | Examining shows its pressure, as for anything holding gas |
