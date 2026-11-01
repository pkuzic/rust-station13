# Blast doors

Sources:
- `luna:code/game/machinery/doors/poddoor.dm` (`open`, `close`, `attackby`);
- `luna:code/defines/obj/door.dm` (`/obj/machinery/door/poddoor`, "Podlock");
- `luna:code/game/machinery/door_control.dm` (`/obj/machinery/door_control`,
  `attack_hand`, `power_change`);
- `NSV_Luna.dmm`: the toxins burn chamber's vent ("Chamber Vent Control") and the
  incinerator's vent and input chute.

Code: `game/classic/src/blastdoors.rs`. Tests: `game/classic/tests/blast_doors.rs`,
`game/classic/tests/luna_rooms.rs`.

## Blast door (`blast-door`)

A heavy shutter on a tile. While it blocks, nothing passes it: people, sight, light or air
(it is a door portal in the air simulation, like an airlock).

- **Map vars:** `id` (shared with its buttons); `density = 0` starts it open. Luna's default
  is shut.
- **Opening:** shows `pdoorc0` and keeps blocking for 1 s, then is open (`pdoor0`).
- **Closing:** blocks at once and shows `pdoorc1` for 1 s, then is shut (`pdoor1`).
- **By hand:** nothing opens it but its buttons, except a crowbar, which pries a shut blast
  door open while it has no power. Nobody is told anything.

## Remote door control (`door-button`)

- **Map vars:** `id`.
- **Pressing it** (any hand, anything in it): a powered button opens every shut blast door
  with its id and shuts every open one; a door already moving is left alone. It shows
  `doorctrl1` for 1 s, then `doorctrl0` and `0` or `1` for which way it was last pressed;
  `doorctrl-p` without power.

## Differences from Luna

| Luna | Here |
|---|---|
| Blast doors draw with the icon the mapper chose (the toxins vent's three-tile megapod) | All use `rapid_pdoor.dmi` |
| A blast door can close automatically after opening (`autoclose`) | No map sets it; not built |
| Pressing a button uses 5 W once | No power is drawn |
