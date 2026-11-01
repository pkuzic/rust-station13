# Igniters

Sources:
- `luna:code/game/machinery/igniter.dm` (`igniter`: `attack_hand`, `process`,
  `power_change`; `ignition_switch`: `attack_hand`);
- `luna:code/defines/obj/machinery.dm` (`/obj/machinery/igniter`,
  `/obj/machinery/ignition_switch`);
- `NSV_Luna.dmm`: the toxins burn chamber and the incinerator each have an igniter and a
  switch with the same `id`.

Code: `game/classic/src/igniters.rs`. Tests: `game/classic/tests/igniters.rs`.

## Floor-mounted igniter (`igniter`)

- **Map vars:** `id` (shared with its switches), `on` (`0` or `1`; Luna's default is on).
- **Power:** from its area's equipment channel. Without power it doesn't spark.
- **Sparks:** while on and powered, every machine cycle (2 s) it touches the air on its
  tile with a 6 000 K spark (Luna `hotspot_expose(6000, 500)`). With plasma and oxygen
  about, that lights the room (`fire.md`).
- **Clicking it** with a hand or anything else flips it on or off, powered or not.
- **Sprite:** `igniter1` while on and powered, else `igniter0`.

## Ignition switch (`ignition-switch`)

- **Map vars:** `id`.
- **Pressing it:** a powered switch that isn't already down goes down for 5 s
  (`launcheract`) and flips every igniter with its id; then it comes back up
  (`launcherbtt`). Presses while it is down, or without power, do nothing. Nobody is told
  anything.

## Differences from Luna

| Luna | Here |
|---|---|
| A spark heats 500 L of air | It heats the same 125 L as every other flame (`HOTSPOT_VOLUME`); the spark is hotter than a welding tool's, and the room catches the same way |
| The switch also fires wall-mounted sparkers (`migniter`) | No sparkers yet |
| Switching uses 50 W (igniter) and 5 W (switch) once | No power is drawn |
