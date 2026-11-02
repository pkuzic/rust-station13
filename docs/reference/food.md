# Food, drink and hunger

Sources: `luna:code/modules/Chemistry/reagent_containers/food.dm`, `food/snacks.dm`,
`food/drinks.dm` (`attack`, `On_Consume`, `examine`), `luna:code/game/objects/items/trash.dm`,
`luna:code/modules/Chemistry/Chemistry-Reagents.dm` (what each reagent feeds),
`luna:code/WorkInProgress/NewMobs and etc/human_life.dm` (hunger). Sold by the vending
machines (`vending.md`).

Code: `game/classic/src/food.rs`. Tests: `game/classic/tests/food.rs`.

## Hunger

Every person has a nutrition level (Luna `nutrition`). It starts at 1 600 and drops by 2
every life cycle (2 s). At nothing, once every ten minutes, a pang: "You feel hungry",
"Your stomach rumbles", "Hmm, some pizza would be nice" or one of Luna's six others. Nothing
else comes of hunger in Luna, and nothing does here.

## Snacks

Clicking yourself with a snack in the active hand takes a bite: how full you are (nutrition
÷ 2) decides the message, with an eating verb picked once per snack (bite, chew, nibble,
gnaw, gobble or chomp):

| Fullness | Message |
|---|---|
| up to 100 | You hungrily bite some of the candy and gobble it down! |
| to 250 | You hungrily begin to bite the candy. |
| 250 to 550 | You bite the candy. |
| 550 to 850 | You unwillingly bite a bit of the candy. |
| over 850 | You cannot force any more of the candy to go down your throat. (no bite) |

Clicking someone else in reach feeds them a bite, silently, as in Luna. Each bite plays
`eatfood.ogg`. When the last bite is gone the snack's wrapper is left in your hand.
Examining a bitten snack: "The candy was bitten 2 times!".

| Prototype | Luna type | Bites | Nutrition | Leaves |
|---|---|---|---|---|
| `candy` | `snacks/candy` | 6 | 142.5 | `trash-candy` |
| `chips` | `snacks/chips` | 11 | 165 | `trash-chips` |
| `beef-jerky` | `snacks/sosjerky` | 6 | 180 | `trash-sosjerky` |
| `no-raisin` | `snacks/no_raisin` | 14 | 210 | `trash-raisins` |
| `space-twinkie` | `snacks/spacetwinkie` | 7 | 160 | nothing |
| `cheesie-honkers` | `snacks/cheesiehonkers` | 6 | 180 | `trash-cheesie` |

Bites are Luna's reagents over its bite size, counting the 8 nutriment every snack gets;
nutrition is what those reagents give as they are digested (nutriment 15 a unit, sugar 2.5).

## Drinks

Clicking yourself with a drink takes a gulp: a fifth of what is left, at least 5 units
("You swallow a gulp of the Space Cola.", `drink.ogg`). An empty one stays in your hand:
"None of the Space Cola left, oh no!". Clicking someone else in reach gives them a gulp, and
everyone sees "Name feeds Other the Space Cola.". Examining one: "The Space Cola is half
full!".

| Prototype | Luna type | Units | Nutrition |
|---|---|---|---|
| `coffee` | `drinks/coffee` | 30 of 50 | 0 |
| `tea` | `drinks/cup/tea` | 30 | 0 |
| `hot-chocolate` | `drinks/cup/h_chocolate` | 30 | 60 |
| `dry-ramen` | `drinks/dry_ramen` | 30 of 50 | 30 |
| `cola`, `space-mountain-wind`, `space-up` | `soda/...` | 30 of 50 | 75 |
| `dr-gibb`, `starkist` | `soda/...` | 30 of 50 | 85 |

## Differences from Luna

| Luna | Here |
|---|---|
| Food is reagents, digested over time, with effects: coffee and tea keep you warm and awake, sodas cool you, nutriment heals a little bruising, Space Mountain Wind makes you jittery | Each bite or gulp gives its nutrition at once; no other effects yet |
| Fullness also counts the nutriment still in your stomach | Only your nutrition |
| Every step costs 1 nutrition | Only the life cycle's 2 |
| Feeding someone a drink waits for them to hold still ("Name attempts to feed Other…") | It happens at once |
| Pouring and splashing drinks; adding water to dry ramen makes hot ramen | Not built |
| Wrappers burn up in a fire | Not built |
