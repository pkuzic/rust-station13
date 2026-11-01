# Fire extinguishers

Sources:
- `luna:code/game/objects/items.dm` (`extinguisher`: `attack_self`, `afterattack`,
  `examine`; `extinguisher/mini`);
- `luna:code/game/objects/effects/effect_system.dm` (`effect/water`);
- `luna:code/modules/Chemistry/Chemistry-Reagents.dm` (`water`: `reaction_turf`,
  `reaction_obj`);
- `luna:code/game/objects/structures/extinguisher.dm` (`extinguisher_cabinet`);
- `luna:code/game/objects/structures/crates_lockers/closets/utility_closets.dm`
  (`firecloset`), `luna:code/game/objects/storage/toolbox.dm` (`toolbox/emergency`);
- `NSV_Luna.dmm`: 13 extinguishers, 13 cabinets, 10 fire closets, 3 water tanks.

Code: `game/classic/src/extinguishers.rs`, `AtmosGrid::douse` in
`engine/simulation/src/atmos/grid.rs`. Tests: `game/classic/tests/extinguishers.rs`,
`engine/simulation/src/atmos/tests.rs` (`water_…`).

## Fire extinguisher (`fire-extinguisher`, `fire-extinguisher-mini`)

An item: force 12, size 3, conducts (the mini one: force 3, size 2). It holds 50 units of
water (the mini one 30) and starts full with the safety on.

- **Safety:** clicking it in the active hand flips the safety: "The safety is off." /
  "The safety is on." With the safety off it shows `fire_extinguisher1` (`miniFE1`).
- **Spraying:** with the safety off, clicking any tile or anything on the same deck, at
  any range, sprays, at most once every 2 s, with Luna's sound. Five puffs of water
  (a unit each) leave the user's tile, each heading for the clicked tile or one of the
  two beside it across the spray, picked at random. A puff steps toward its tile every
  0.2 s, diagonals included, five steps at most, and stops at walls, closed doors,
  closed firedoors and window panes. It is gone 7 s after the spray.
- **Water on fire:** on every tile a puff reaches, if the air there is burning, it drops
  to max(min(T − 2000, T / 2), 2.7) K and stops burning (Luna's water on a hotspot). Two
  puffs on one tile freeze any flame; a tile of a room burning as a whole becomes a fire
  cell first, so only that tile is cooled. A fire caught as it starts goes out for good;
  once plasma burns through a whole room, one extinguisher can't put it out.
- **Empty:** "The fire extinguisher is empty."
- **Refilling:** clicking a water tank within reach fills it up from the tank, safety on
  or off: "The fire extinguisher is now refilled.", with Luna's refill sound.
- **Examine:** "It contains 45 units of water left!"
- With the safety on it is used like any other item (it hits people).

## Water tank (`water-tank`)

Dense, holding 1 000 units of water. Examining it: "It contains 1000 units of water."

## Extinguisher cabinet (`extinguisher-cabinet`)

Hung on a wall (the map's pixel offset); it starts closed with an extinguisher inside.

- An empty hand takes the extinguisher out, opening the door: "You take the fire
  extinguisher from the extinguisher cabinet." With nothing inside, an empty hand opens
  or closes the door.
- An extinguisher goes in when the door is open and the cabinet empty: "You place the
  fire extinguisher in the extinguisher cabinet." Otherwise anything in the hand opens or
  closes the door.
- Sprite: `extinguisher_closed`, or open `extinguisher_full`, `extinguisher_mini` or
  `extinguisher_empty`.

## Fire-safety closet (`closet-fire`)

A closet (`closets.md`): `firecloset` / `fireclosetopen`, with two emergency toolboxes and
two fire extinguishers. An emergency toolbox (`toolbox-emergency`, `red`) holds a
crowbar, a welding tool, a mini fire extinguisher, a flashlight and a radio.

## Differences from Luna

| Luna | Here |
|---|---|
| Spraying in space, or buckled to a chair that can move, pushes the user back | Not built |
| Water reacts with everything on the tiles it reaches (slimes, burning people) | Only the air: there are no burning people yet |
| Three or more units of water on a floor make it slippery for 80 s | Puffs carry one unit; not built |
| Each puff moves through the air as an object that only floors stop | Puffs stop at closed doors and sealed edges too, as people do |
| The air under water drops to 0 K | 2.7 K, the coldest anything gets here |
| The emergency toolbox has a red crowbar, a mini welding tool and a flare half the time | The plain crowbar and welding tool, and always a flashlight |
| The fire cabinet's extinguisher can be moved up and down decks (`move_z`) | Not built |
