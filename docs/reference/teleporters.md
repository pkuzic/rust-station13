# Teleporters

Sources: `luna:code/game/machinery/teleporter.dm` (the computer, hub and station, `do_teleport`),
`luna:code/defines/obj/computer.dm`, `luna:code/game/objects/items/weapons/teleportation.dm`
(`hand_tele`), `luna:code/game/objects/portals.dm`, `luna:code/game/objects/items/devices/radio/beacon.dm`;
`NSV_Luna.dmm`: the teleporter room on deck 3, and 16 tracking beacons about the station.

Code: `game/classic/src/teleporters.rs`. Tests: `game/classic/tests/teleporters.rs`.

## The teleporter

Three machines in a row: the **computer** (`teleporter-console`), the **station**
(`teleporter-station`) and the **hub** (`teleporter-hub`).

- **Computer**: each has an id, a random number from 1000 to 9999. Clicked, with power, it lists
  every tracking beacon on the station by its area (the second in an area is "(2)", and so on),
  wherever the beacon is, on the floor or carried. Picking one locks the computer onto it: "Locked
  In", for those around.
- **Station**: clicked, with power, it engages or disengages the hub beside it ("Teleporter
  engaged!", "Teleporter disengaged!"). Without power the hub is off, and the station shows it.
- **Hub**: while it's on, walking into it throws you to the beacon the computer two tiles before
  it is locked onto, dead on, in a shower of sparks. One time in twenty you end up somewhere out
  in space instead. With no lock: "Failure: Cannot authenticate locked on coordinates. Please
  reinstate coordinate matrix."

## Hand teleporter (`hand-tele`)

Used in the hand: a list of every hub whose computer is locked onto something, by that computer's
id, "(Active)" or "(Inactive)", and "None (Dangerous)". Picking a line, still holding it and
awake, opens a portal on their tile ("Locked In") to that beacon, or, for None, to a random spot
within ten tiles. A hand teleporter keeps at most four open: "The hand teleporter is recharging!"

**Portals** (`portal`) last 30 seconds and stand in the way. Walking into one, or clicking it,
throws you onto or beside where it leads (a beacon wherever it's been taken since). One time in
25 it goes wrong: out into space with you, and the portal throws no one after that. A portal
whose beacon is gone closes.

## Differences from Luna

| Luna | Here |
|---|---|
| The computer also lists tracking implants | No implants yet (3.20) |
| The computer's id can be changed with a verb; the station has a Test Fire verb | Not built: few verbs (4.14) |
| Teleporting draws 5000 W | No extra draw |
| Space is its own level | A random space tile of a deck |
| Thrown things and things carried on a conveyor go through hubs and portals too | Only people walking |
