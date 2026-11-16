# Grenades

Sources: `luna:code/game/objects/items/weapons/grenades.dm` (the flashbang and EMP grenade),
`luna:code/defines/obj/storage.dm` (their boxes), `luna:code/game/objects/structures/crates_lockers/closets/secure/security.dm`
(the lockers), `luna:code/modules/mob/living/carbon/life.dm` (ears healing). `NSV_Luna.dmm`: four
boxes of flashbangs in the armoury.

Code: `game/classic/src/grenades.rs`. Tests: `game/classic/tests/grenades.rs`.

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
- **A clumsy person** ("Huh? How does this thing work?!") sets it off in half a second either way.
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

## Where they are

- **Box of flashbangs** (`flashbang-box`, seven flashbangs): four in the armoury, one in the head of
  security's and the warden's lockers, one in the riot gear crate. A security locker holds a single
  flashbang seven times in ten, otherwise a box.
- **Box of EMP grenades** (`emp-grenade-box`, five): the traitor's uplink sells one for 4 crystals.

## Differences from Luna

| Luna | Here |
|---|---|
| A flashbang's sunglasses check misreads a welding helmet, so one still lets the far flash through | Sunglasses or a welding helmet keep it out at any distance |
| Hulks were meant to shrug off the knock-down (a misplaced `!` stopped it working) | No one shrugs it off, as in Luna in practice |
| Cloaking devices switch off | Not built |
| The EMP grenade's own list: silicons hurt and the AI's camera cut, thermals fail, machines draw power, computers break one time in five, fire alarms sound, SMES units and APCs drained and shut down for a while, airlocks opened, clone pods fail, shield tiles drop | The general pulse plus radios, flashes, doors and cameras. The rest wait for silicons (3.9, 3.10), thermals and those machines' EMP behaviour |
| The EMP pulse and spark pictures | Only the sound |
| Imidazoline and surgery mend eyes, and ears mend under 25 | Ears mend; eyes don't yet (3.6) |
| Chemical grenades, and the other grenade kinds | Not built yet |
