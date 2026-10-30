# Pressure meters

Sources: `luna:code/game/machinery/atmoalter/meter.dm` (`New`, `process`, `examine`,
`Click`) and `luna:code/defines/obj/machinery.dm` (`/obj/machinery/meter`).

Code: `game/classic/src/meters.rs`. Tests: `game/classic/tests/meters.rs`.

## The meter

- A meter sits on a pipe, manifold or gas tank and reads the pressure of its pipe network.
  It finds the pipe on its own tile, and finds a new one if that pipe goes.
- It runs on the area's equipment power (5 W).
- It isn't dense, and it can't be moved or taken apart.

## The dial

| Pipe pressure | Sprite |
|---|---|
| no pipe on its tile | `meterX`: the connect error light |
| unpowered | `meter0` |
| up to 0.15 atm | `meter0` |
| up to 1.8 atm | `meter1_1` … `meter1_6` |
| up to 30 atm | `meter2_1` … `meter2_6` |
| up to 59 atm | `meter3_1` … `meter3_6` |
| above | `meter4` |

Within a band the step follows Luna's formulas, with BYOND's `round`: for example
1 atm shows `meter1_4`, and 2 atm shows `meter2_1`.

## Reading it

- **Clicking it,** with anything or an empty hand, from up to 3 tiles away: "Pressure:
  N kPa". With no pipe: "Results: Connection Error!". Farther away: "You are too far away."
  Unpowered, it says nothing.
- **Examining it:** "A gas flow meter. The pressure gauge reads N kPa.", or "The connect
  error light is blinking." with no pipe.

## Differences from Luna

- **Precision:** pressures are whole kPa, like our other gauges; Luna shows tenths.
- **Not implemented:** the radio frequency that sends readings to consoles, the AI reading
  meters from anywhere, and the sensor error for a pipe with no air.
