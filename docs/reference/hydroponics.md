# Hydroponics

Source: `luna:code/WorkInProgress/ISaidNo Release/hydroponics.dm`, `plants_food_etc.dm`; `NSV_Luna.dmm`: 9
plant pots and a seed vendor.

Code: `game/classic/src/hydroponics.rs`. Tests: `game/classic/tests/hydroponics.rs`.

Seeds come from the seed vendor. Click an empty plant pot with a seed to plant it. Every 2 seconds a plant
with water grows, as long as there are no more than 200 units around it, which is how much a new pot starts
with. Pouring in more with a beaker or bucket stops it growing until there's less again. A plant with no water loses health, depending on
its endurance, and dies at none.

With an empty hand, click a ripe plant to pick its crop, which lands at your feet, or a dead plant to clear it
away. After a harvest it needs time to ripen again, and after its last harvest it's gone. Examine a pot to see
how it's doing.

Plants take Luna's times, so they take a good while to first ripen. See `PLANTS` in the code.

## Differences from Luna

| Luna | Here |
|---|---|
| 33 plants, with genes, mutations, weeds and man-eaters | 8 food plants, no genes or mutations |
| Fertilizers, toxins and other reagents change how a plant grows | Only water counts |
| Plants' condition can make a harvest better or worse | Always the same amount |
| Water very slowly drains | It stays, so a plant only gets thirsty if something takes the water out |
| Compost, seed extractors and mutators, watering cans, analyzers and more | Not built |
| Each crop has its own reagents, and some are juiced or cooked | They all give the same plain nutrition |
