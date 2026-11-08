# Sinks, showers, toilets and mirrors

Sources: `luna:code/game/objects/structures.dm` (`structure/sink`, `structure/toilet`: the ones
compiled, as `watercloset.dm`'s are commented out), `luna:code/game/objects/structures/watercloset.dm`
(`machinery/shower`, `effect/mist`), `luna:code/game/objects/structures/mirror.dm`;
`NSV_Luna.dmm`: sinks in the washrooms, the kitchen sink, showers and toilets in the dormitories.

Code: `game/classic/src/plumbing.rs`. Tests: `game/classic/tests/plumbing.rs`.

## Sink (`sink`, `kitchen-sink`)

Clicked in reach:

- **Empty hand**: "You start washing your hands." After 4 seconds, if they haven't moved, everyone
  around sees them wash their hands.
- **A bucket** fills with 70 units of water. **Other glassware** (beakers, bottles) gets 10. It
  happens at once, and those around see it.
- **Anything else held**: washed the same way, in 4 seconds. Nothing happens if they've moved or
  are no longer holding it.

One person at a time: anyone else is told "Someone's already washing here."

## Shower (`shower`)

- **Empty hand**: turns it on or off. While it's on, water falls over its tile. Five seconds after
  it's turned on it mists up, unless the water is freezing; the mist lingers 25 seconds after it's
  turned off.
- **Wrench**: "You begin to adjust the temperature valve with the wrench." After 5 seconds standing
  still and still holding it, the water goes from normal to freezing to boiling and back.
- **Air analyzer**: "The water temperature seems to be normal" (or freezing, or boiling).

Anyone under it while it's on feels it as they step in, and every 2 seconds after:

| Water | Effect |
|---|---|
| Normal | nothing |
| Freezing | body temperature 80 K lower, never below 80 K: "The water is freezing!" |
| Boiling | body temperature 35 K higher, never above 500 K, and 5 burn: "The water is searing!" |

## Toilet (`toilet`)

A grab used on it (`combat.md`): held by both hands or tighter, and standing on the toilet's tile,
they're slammed into it for 8 brute, which those around see. Otherwise: "You need a tighter grip."
Its lid never lifts, so it never comes to a swirlie, and an empty hand does nothing (both as in
Luna's compiled toilet).

## Mirror (`mirror`)

Hit with something in hand, it shatters at twice the item's force, in percent: those around see
it smashed, it looks broken from then on, and glass breaks. Otherwise they see it hit. A broken
one only makes a sound.

## Differences from Luna

| Luna | Here |
|---|---|
| Washing and showers clean blood off people and what they wear and carry | There's no blood on things yet: only the messages |
| Showers clear dirt off their floor | No dirt yet |
| Shots can shatter a mirror | Only things in hand |
| A mirror is drawn on the wall above its tile | The same, from its offset on the map |
