# Hydroponics

Source: `luna:code/WorkInProgress/ISaidNo Release/hydroponics.dm`, `plants_food_etc.dm`; `NSV_Luna.dmm`: 9
plant pots and a seed vendor.

Code: `game/classic/src/hydroponics.rs`. Tests: `game/classic/tests/hydroponics.rs`.

Seeds come from the seed vendor. Click an empty plant pot with a seed to plant it. Every 2 seconds a plant with
water grows by 3 and drinks half a unit. A new pot starts with 200 units, enough for over 13 minutes, so a plant
needs watering eventually: pour more in with a beaker or bucket. With no water it stops growing and loses health,
depending on its endurance, and dies at none.

With an empty hand, click a ripe plant to pick its crop, which lands at your feet, or a dead plant to clear it
away. After a harvest it needs time to ripen again, and after its last harvest it's gone. Examine a pot to see
how it's doing.

They first ripen in about 6 to 14 minutes, depending on the plant.

## Differences from Luna

| Luna | Here |
|---|---|
| 33 plants, with genes, mutations, weeds and man-eaters | 8 food plants, no genes or mutations |
| Fertilizers, toxins and other reagents change how a plant grows | Only water counts |
| Plants' condition can make a harvest better or worse | Always the same amount |
| Plants take up to 43 minutes to first ripen, growing by 1 a cycle | 3 times as fast, so a round can see it |
| Plants only grow with no more than 200 units of water around them, and it hardly ever drains | Any water will do, and they drink it |
| Compost, seed extractors and mutators, watering cans, analyzers and more | Not built |
| Some crops are juiced or cooked | They're just eaten, for Luna's 8 nutriment and number of bites |
