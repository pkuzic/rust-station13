# Microwaves

Sources: `luna:code/game/machinery/kitchen/microwave.dm` (`attackby`, `cook`, `fail`, `stop`),
`luna:code/modules/food/recipes_microwave.dm` (`donkpocket/warm`),
`luna:code/modules/Chemistry/reagent_containers/food/snacks.dm` (`donkpocket`, `cooltime`,
`badrecipe`); `NSV_Luna.dmm`: 7 microwaves, 7 boxes of Donk-pockets.

Code: `game/classic/src/microwaves.rs`. Tests: `game/classic/tests/microwaves.rs`.

## Using one

A dense machine that needs equipment power: 5 W idle, 500 W running.

- **Clicking it with a Donk-pocket** puts it in, up to 6 things. Anything else, you're told you have
  no idea what you could cook with that.
- **Clicking it with an empty hand** starts it, for 10 seconds. Everyone who can see it is told. It
  lights up, and dings when done.
- **What comes out**, onto its tile:
  - one Donk-pocket: warm;
  - two or more things: one burned mess, and the food is gone;
  - nothing: it just runs.
- **A power cut** stops it, with everything still inside.

## Donk-pockets

Food of 6 bites, sold as a box of 6 (`donk-pockets`). A warm one is 2 bites at most, and each bite
heals 2.5 of oxygen damage, toxin damage, brute and burn. It goes cold after 7 minutes.

## Differences from Luna

| Luna | Here |
|---|---|
| Over 100 recipes | Only warming Donk-pockets: the rest need ingredients and chemistry we don't have. Anything else isn't accepted |
| A window with Cook and Dispose | Clicking with an empty hand cooks; there is no taking things back out |
| Failing can cover it in muck or break it, to be cleaned or repaired | Not built, as there's no cleaner yet |
| Warmth is 5 units of tricordrazine, working over time | Each bite heals at once |
| A burned mess is toxic | Just poor food, as toxins aren't built |
| The name turns into "Warm Donk-pocket" | Objects don't take names yet |
