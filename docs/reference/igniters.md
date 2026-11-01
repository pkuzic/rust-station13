# Igniters

Sources:
- `luna:code/game/machinery/igniter.dm` (`igniter`: `attack_hand`, `process`,
  `power_change`; `sparker`: `power_change`, `attackby`, `ignite`; `ignition_switch`:
  `attack_hand`);
- `luna:code/defines/obj/machinery.dm` (`/obj/machinery/igniter`,
  `/obj/machinery/ignition_switch`);
- `NSV_Luna.dmm`: the toxins burn chamber and the incinerator each have an igniter and a
  switch with the same `id`.

Code: `game/classic/src/igniters.rs`. Tests: `game/classic/tests/igniters.rs`,
`game/classic/tests/sparkers.rs`.

## Floor-mounted igniter (`igniter`)

- **Map vars:** `id` (shared with its switches), `on` (`0` or `1`; Luna's default is on).
- **Power:** from its area's equipment channel. Without power it doesn't spark.
- **Sparks:** while on and powered, every machine cycle (2 s) it touches the air on its
  tile with a 6 000 K spark (Luna `hotspot_expose(6000, 500)`). With plasma and oxygen
  about, that lights the room (`fire.md`).
- **Clicking it** with a hand or anything else flips it on or off, powered or not.
- **Sprite:** `igniter1` while on and powered, else `igniter0`.

## Wall-mounted igniter (`sparker`)

- **Map vars:** `id` (shared with its switches).
- **Power:** from its area's equipment channel.
- **Light:** a faint pilot light (2 tiles) while powered and connected (Luna
  `ul_SetLuminosity(2)`).
- **Sparking:** told to by its switch, a powered, connected sparker that hasn't sparked in
  the last 5 s throws one 6 000 K spark on its tile, which lights plasma there at once
  (`fire.md`), and shows `migniter-spark` for half a second.
- **Screwdriver:** undoes or redoes its connection; disconnected, it neither sparks nor
  glows. The user is told "You disable the connection to the wall-mounted igniter." /
  "You fix the connection to the wall-mounted igniter.", and everyone else who sees it
  "Name has disabled the wall-mounted igniter!" / "Name has reconnected the wall-mounted
  igniter!". Anything else in the hand does nothing.
- **Sprite:** `migniter` ready, `migniter-p` without power, `migniter-d` disconnected.

## Ignition switch (`ignition-switch`)

- **Map vars:** `id`.
- **Pressing it:** a powered switch that isn't already down goes down for 5 s
  (`launcheract`), flips every igniter with its id and tells every sparker with its id
  to spark; then it comes back up
  (`launcherbtt`). Presses while it is down, or without power, do nothing. Nobody is told
  anything.

## Differences from Luna

| Luna | Here |
|---|---|
| A spark lights the tile's air (`obj/fire/New` warms it by 6 000 K) | The zone warms by that tile's share (`fire.md`, Ignition) |
| A sparker also throws a shower of spark effects | Only its sprite shows the spark |
| Switching uses 50 W (igniter) and 5 W (switch) once | No power is drawn |
