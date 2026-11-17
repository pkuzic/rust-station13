# Grenades

Sources: `luna:code/game/objects/items/weapons/grenades.dm` (the flashbang and EMP grenade),
`luna:code/modules/Chemistry/Chemistry-Tools.dm` (chemical grenades), `Chemistry-Recipes.dm` (smoke
and foam), `Chemistry-Reagents.dm` (chemicals on the skin), `luna:code/game/objects/effects/effect_system.dm`
(smoke, foam and foamed metal),
`luna:code/defines/obj/storage.dm` (their boxes), `luna:code/game/objects/structures/crates_lockers/closets/secure/security.dm`
(the lockers), `luna:code/modules/mob/living/carbon/life.dm` (ears healing). `NSV_Luna.dmm`: four
boxes of flashbangs and four of tear gas in the armoury, two boxes of metal foam grenades, eight
empty casings.

Code: `game/classic/src/grenades.rs`, `game/classic/src/chem_effects.rs`. Tests:
`game/classic/tests/grenades.rs`, `game/classic/tests/chem_grenades.rs`.

Luna counts stuns, deafness and eye trouble in life ticks of 2 s; so do we.

## Using a grenade

- **In the hand**, a grenade is primed: "You prime the flashbang! 3 seconds!" Its sprite turns
  live, and it goes off when its time is up, wherever it is by then: in a hand, in a pocket or bag,
  or on the floor. Primed, it can't be stopped.
- **Clicked at somewhere out of reach**, it is primed if it wasn't (with the arming sound), the
  thrower turns that way and it leaves the hand, flying a tile every 0.3 s along a straight line.
  It stops short of a wall, a person or anything else dense other than a table or rack, and when
  it gets there. Picked
  up on the way, it stops.
- **A clumsy person** ("Huh? How does this thing work?!") sets a flashbang or EMP grenade off in half
  a second either way.
- A chemical grenade can only be primed once it's locked, and is only thrown as it's primed; it
  always takes 3 seconds and always makes the arming sound.
- **A screwdriver** on a flashbang switches its delay between 3 and 10 seconds. The EMP grenade
  always takes 5.

## Flashbang

A bang (heard up to 25% loud), a spark at the tile, and everyone who can see the spot, out to 7
tiles, is hit by how far off they are:

| Where | What happens |
|---|---|
| Holding it, on its tile, or 2 tiles or closer | Dazzled and knocked down for 10 ticks; deaf for 30. Ears hurt 1–10 one time in seven (seven in ten when held), otherwise 0–5 three times in ten; eyes 0–5 unless unconscious, and 0–4 more one time in ten. Held, eyes take 2–5 more and, six times in ten unless earmuffs are worn, ears 15 more and 60 more ticks deaf |
| 3 to 5 tiles | A monkey is dazzled and knocked down for 7 ticks. A person is deaf for 10 ticks, ears hurt 0–4 one time in five, and dazzled and knocked down for 7 ticks unless wearing sunglasses or a welding helmet (or out cold); with them, eyes hurt 1–3 |
| Further | Dazzled unless the eyes are covered; eyes hurt 1–2; deaf for 5 ticks |

Everyone hit hears "BANG". Then, for each:

- Eye damage of 20 or more: "Your eyes start to burn badly!", nearsighted for good, and with a
  chance of (damage − 19)% blind for good ("You can't see anything!").
- Ear damage of 15 or more: "Your ears start to ring badly!", and with a chance of (damage − 5)%
  deaf for good ("You can't hear anything!"). From 5: "Your ears start to ring!"

Ear damage under 25 heals by 0.05 a life tick. Eye damage doesn't heal by itself.

Blob cells out to 8 tiles lose 30 / (distance + 1) health.

## EMP grenade

An electromagnetic pulse out to 6 tiles, as `explosions.md` describes (cells, energy guns and
batons lose 1000 charge, locker locks may flip), and a spark at the tile. Besides:

- Everyone who can see it within 6 tiles has every radio they carry switched off, receiving and
  sending, and every flash they carry goes off as if fired into the air.
- Doors within 6 tiles become live four times in ten for 30 seconds, unless they already are.
- Cameras within 6 tiles go dark for 90 seconds.

## Chemical grenades

A **metal casing** (`grenade-casing`) is put together in three steps, each in the hand or on the floor
within reach:

1. A timer-igniter assembly (`devices.md`), used up: "unsecured grenade".
2. Up to two beakers, bottles or buckets with something in them ("The grenade can not hold more
   containers."; an empty one is refused).
3. A screwdriver locks it ("You need to add at least one beaker before locking the assembly." if
   there are none).

Going off, if its beakers are empty it only clicks and can be primed again. Otherwise it goes off
with a whoosh: the beakers are emptied into it one after the other, mixing as they go, so a
reaction with an effect (below) happens where the grenade is. Whatever is left unreacted touches
everyone who can see the spot within 3 tiles. The casing and beakers are gone.

Ready-made, locked, with two beakers each:

| Grenade | First beaker | Second beaker | Makes |
|---|---|---|---|
| Metal foam (`metal-foam-grenade`) | 30 aluminium | 10 foaming agent, 10 polytrinic acid | Metal foam, 4 tiles out |
| Cleaner (`cleaner-grenade`) | 30 fluorosurfactant | 10 water, 10 space cleaner | Foam, 3 tiles out |
| Tear gas (`teargas-grenade`) | 25 condensed capsaicin, 25 potassium | 25 phosphorus, 25 sugar | Smoke carrying the capsaicin |

## Smoke

Potassium, sugar and phosphorus, mixed anywhere (`chemistry.md`), make smoke and take everything else
in the container with them; the container is left empty. Ten puffs appear, each carrying a tenth of
it, and ten more a second later. Each drifts one way of the eight, a tile a second, 0–3 tiles,
stopping at walls and anything dense, and is gone 16–18 seconds after it stops (22 at most). Puffs
block sight. As a puff drifts, everyone next to its new tile is touched by what it carries seven
times in ten; walking into a puff, nine times in ten.

## Foam

Fluorosurfactant and water make foam, which carries a unit of each reagent that was in the container
(water if none) and empties it; aluminium, foaming agent and polytrinic acid make metal foam
(iron instead of aluminium makes iron foam), which carries nothing. Either spreads from where it was
mixed, as far as the square root of a third of what the reaction made: foam already there just
grows. Every 0.3 s (0.6 s for metal foam) a foam tile spreads to each side it could move into and
that has no foam yet, one less far each time. People don't stop it; walls, windows and dense things
do.

- Plain foam lasts 15 seconds, then melts away, touching whoever stands in it with a unit of each of
  its reagents. Heat over 475 K can melt it sooner. Walking into it slips you for 5 ticks unless your
  shoes don't slip ("You slipped on the foam!").
- Metal foam sets after 15 seconds into a **foamed metal wall**: dense, opaque and airtight, sealing
  a breach. By hand it breaks one time in two (aluminium) or four (iron), always for a hulk; with
  something, the item's force × 20 − 25 (or − 50) percent of the time. When it goes the air flows
  again.

## On the skin

What a chemical does when it touches someone (smoke, foam melting, a grenade's splash):

- **Condensed capsaicin** (people only): nothing if eyes and mouth are both covered (a gas mask, a
  space helmet); with only the mouth covered (a breath mask), blurred for 4 ticks, blind for 1 and
  knocked down for 2, dropping what they hold; with only the eyes covered (any glasses, a welding or
  riot helmet), blurred for 3; uncovered, blurred for 8, blind for 2, knocked down for 5 and dropping
  what they hold.
- **Polytrinic acid**: a person's mask and hat melt away and the face takes 20 damage, disfigured;
  once disfigured, or a monkey, 22 burns instead.
- **Sulphuric acid** (people only): a mask melts instead, or else a hat; otherwise one time in five
  the face takes 12 and is disfigured, or 14 burns.

## Where they are

- **Box of flashbangs** (`flashbang-box`, seven flashbangs): four in the armoury, one in the head of
  security's and the warden's lockers, one in the riot gear crate. A security locker holds a single
  flashbang seven times in ten, otherwise a box.
- **Box of EMP grenades** (`emp-grenade-box`, five): the traitor's uplink sells one for 4 crystals.
- **Box of tear gas grenades** (`teargas-box`, seven): four in the armoury, one in the riot gear crate.
- **Box of metal foam grenades** (`metal-foam-box`, five): two on the station.
- **Box of cleaner grenades** (`cleaner-box`, five): the janitorial supplies crate.

## Differences from Luna

| Luna | Here |
|---|---|
| A flashbang's sunglasses check misreads a welding helmet, so one still lets the far flash through | Sunglasses or a welding helmet keep it out at any distance |
| Hulks were meant to shrug off the knock-down (a misplaced `!` stopped it working) | No one shrugs it off, as in Luna in practice |
| Cloaking devices switch off | Not built |
| The EMP grenade's own list: silicons hurt and the AI's camera cut, thermals fail, machines draw power, computers break one time in five, fire alarms sound, SMES units and APCs drained and shut down for a while, airlocks opened, clone pods fail, shield tiles drop | The general pulse plus radios, flashes, doors and cameras. The rest wait for silicons (3.9, 3.10), thermals and those machines' EMP behaviour |
| The EMP pulse and spark pictures | Only the sound |
| Imidazoline and surgery mend eyes, and ears mend under 25 | Ears mend; eyes don't yet (3.6) |
| A built casing is renamed "unsecured grenade" and then "grenade" | It keeps the name "metal casing"; its look changes |
| A chemical grenade throws its holder into throw mode, and leaves steam if anything is left | Neither |
| Smoke puffs take the colour of what they carry | All the same grey |
| Capsaicin also confuses, and someone may scream | Not built |
| Space cleaner cleans what the foam melts on; acids melt items they touch | No dirt or blood to clean yet; items are untouched |
| Foamed metal breaks to grabs, explosions, the blob and bullets | Only to hands and items |
| The other grenade kinds | Not built yet |
