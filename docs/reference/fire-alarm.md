# Fire alarm

Sources:
- `luna:code/game/machinery/alarm.dm` (`firealarm`: `attack_hand`, `attackby`, `alarm`,
  `reset`, `temperature_expose`, `process`, `power_change`);
- `luna:code/defines/obj/machinery.dm` (`/obj/machinery/firealarm`);
- `luna:code/game/area/areas.dm` (`firealert`, `firereset`).

Code: `game/classic/src/firealarms.rs`; firedoors in `game/classic/src/firedoors.rs`.
Tests: `game/classic/tests/fire_alarm.rs`.

## The alarm

- **Where:** on a wall, in an area (`fire-alarm`). Luna's engineering section has one,
  imported with the map.
- **Power:** 10 W from the area's environment channel (Luna `use_power(10, ENVIRON)`).
  Without power it does nothing.
- **Sprite:** `fire0` normal, `fire1` while its area is on alert, `firep` without power.

## Setting it off

An alarm going off puts its whole area on alert (Luna area `fire`): every firedoor in or
next to the area closes (`firedoors.md`). Ways to set it off:

| How | Does |
|---|---|
| Click it with an empty hand | Raises the alarm, or resets it if the area is on alert: "You pull the fire alarm." / "You reset the fire alarm." |
| Hit it with anything but wirecutters | Raises the alarm |
| Wirecutters | Disconnect or reconnect its heat detector; everyone who sees it is told |
| Heat | A powered alarm with its detector connected goes off when the air in front of it is hotter than 200 °C (473.15 K), checked every machine cycle (2 s) |

Clicks need reach. A reset opens the firedoors again unless an air alarm seal, or another
area's alert, still covers them.

## Differences from Luna

- **The panel:** Luna's click opens a window with the lockdown button and a countdown that
  sets the alarm off when it runs out. Here the click is the button itself, and there is no
  countdown. Luna shows people who aren't human a scrambled panel; we have no such people.
- **Messages:** Luna's panel says nothing; we tell the clicker what happened.
- **Looks:** Luna lays a blue alert overlay over every tile of the area and leaves the alarm
  at `fire0`. We don't draw the overlay, so the alarm shows `fire1` instead.
- **Not implemented:** the AI and the atmospheric alert console hearing about it, cameras,
  the party button, fires (heat comes only from hot gas).
