# Air alarms and emergency seals

Sources: `luna:code/game/machinery/alarm.dm` (`/obj/machinery/alarm`), the area procs
`activate_air_doors` and `deactivate_air_doors` in `luna:code/game/area/areas.dm`, and
`panic_fill` in `luna:code/ATMOSPHERICS/components/unary/vent_pump.dm`.

Code: `game/classic/src/alarms.rs`. Client: `apps/client/src/alarm_panel.rs`.
Tests: `game/classic/tests/air_alarm.rs`.

## The alarm

**Placement:**
- An alarm hangs on a wall tile and belongs to that tile's area.
- It reads the air of the first open floor tile next to it in the same area, checking north,
  south, east, then west.
- The engineering chamber has one alarm in the room (west wall) and one in the closet (east
  wall).

**Power:** an alarm draws 5 W from its area's environment channel. Without power it shows
`alarmp` and does nothing.

**Checks:** every machine cycle (2 s) it checks the air in Luna's order:

| Reading | Alert | Danger |
|---|---|---|
| Pressure | outside 91–111 kPa (90–110% of 1 atm) | outside 71–142 kPa (70–140%) |
| Oxygen, moles in the tile | outside 90–110% of standard air | – |
| Temperature | more than 10 K from 20 °C | – |
| CO₂ | over 0.05 mol | – |
| Plasma | over 1 mol | – |

Only pressure can make the air dangerous. The light shows `alarm1` while the air is dangerous
and `alarm0` otherwise.

**Wirecutters:** wirecutters disconnect the alarm, and using them again reconnects it.
Everyone who can see it gets "X has deactivated the air alarm!". A disconnected alarm shows
`alarmp`, can't be opened and never seals.

## Emergency seal

- **Sealing:** when the air has been dangerous for 5 s, the alarm seals its area. In practice
  this happens on the first check at least 5 s later, so after 6 s.
- **Doors affected:** every airlock in the area, or on a tile next to an open tile of it.
- **Powered doors:** the seal closes the door if it is open, then drops its bolts. A door
  already bolted for another reason is left alone.
- **Unpowered doors:** ignored, both when sealing and when lifting.
- **Lifting:** the seal lifts as soon as the alarm sees the air go from dangerous to merely
  alerting or normal. The bolts the seal dropped go up again, unless another sealed area
  still covers the door.
- **By hand:** a seal turned on by hand stays on until it is turned off, or until the air
  turns dangerous and recovers.

### Measured

| Situation | Result |
|---|---|
| Outer door jammed open, alarm working | Closet sealed after 6 s, both doors shut and bolted. The vent then refills the closet, and the seal lifts at 71 kPa after **52 s** |
| Outer door's power cut, door prised open | The closet is sealed, but the dead outer door stays open. The inner door bolts, so the room keeps its air while the closet stays in vacuum |
| Panic fill in a vented closet | Back above 95 kPa after **10 s**, against about 150 s for the vent alone |

## The window

**Opening:**
- Using the alarm, by hand or with anything except wirecutters, opens its window.
- It stays open within 3 tiles. It closes further away, or when the alarm loses power or is
  disconnected.

**Readouts:** each line is blue, or red when out of range.

| Line | Red when |
|---|---|
| `<area> atmosphere` | – |
| Pressure, 0.1 kPa | 10 kPa or more from 1 atm |
| Nitrogen % | 20 points or more from standard air (79%) |
| Oxygen % | 2 points or more from standard air (21%) |
| CO₂ % | over 1% |
| Plasma % | shown only over 1%, always red |
| Unknown % (anything else, e.g. N₂O) | shown only over 1%, always red |
| Temperature °C | 7 K or more from 20 °C |

In a vacuum, only the pressure line shows.

**Buttons:** these need the viewer next to the alarm.
- **Activate / Deactivate emergency seal.**
- **Activate / Deactivate panic fill.** If the area has no vent, the window shows "No vents
  connected!" instead.
- **Close.**

## Panic fill

Panic fill is turned on for every vent in the area.

- **Filling:** while the room is below 95% of 1 atm, each machine cycle the vent takes from
  its pipe network enough gas to bring the whole zone to 1 atm. It needs power and must be
  switched on, and it ignores the breach lockout.
- **Limit:** how fast the room fills depends on what the network holds. In the chamber, the
  canister on the connector port supplies most of it.
- **Stopping:** at 95% it switches itself off and the vent goes back to normal.

## Differences from Luna

- **Wall tiles:** Luna mounts alarms on the floor tile in front of a wall, with a pixel
  offset. Here they sit on the wall tile and sense the open tile beside it.
- **Gas colours:** Luna compares the nitrogen and oxygen shares (0–1) with 20 and 2, so they
  are never red. Here the thresholds are read as percentage points, as intended.
- **No fake readouts:** Luna shows people without the right skill random readings (`mockpanel`).
  There are no skills yet, so everyone sees the real air.
- **Seal timer:** Luna starts a 5 s timer on every unsafe check and seals when any of them runs
  out, even if the air has recovered by then. Here the seal waits for 5 s of continuous
  danger.
- **Several vents:** Luna's alarm controls a single vent, the last one found in its area.
  Here panic fill covers all vents in the area.
- **Panic fill temperature:** Luna works out the moles from the zone's temperature, which in
  a vacuum is near 3 K and empties the pipes in one go. Here it uses the temperature of the
  gas being added.
- **Not implemented:**
  - radio alerts to the atmospherics console and AI;
  - firedoors and blue alert lighting;
  - the "stay closed" delay after a manual seal;
  - fire alarms.
