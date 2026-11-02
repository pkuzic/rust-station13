# Monkeys

Sources: `luna:code/modules/mob/living/carbon/monkey/` (`monkey.dm`, `life.dm`, `emote.dm`);
`NSV_Luna.dmm`: 13 on deck 3.

Code: `game/classic/src/monkeys.rs`. Tests: `game/classic/tests/monkeys.rs`.

A monkey is a mob with a body like anyone's and no player. It can be hit, pulled, flashed and laid
in a morgue, and it breathes and hungers like people do. It doesn't open doors, and dying leaves no
ghost.

Every 2 seconds, while awake:

- it has a 33% chance of a step in a random direction, if the way is clear;
- it has a 1% chance of scratching, waving its tail, rolling or jumping, which anyone who can see it
  is told.

## Differences from Luna

| Luna | Here |
|---|---|
| Its sex is picked at random, with DNA for genetics | Not built |
| Players can become monkeys, and monkeys can wear masks and carry things | Not built |
| Most other emotes | Only the four it does by itself |
| The name "monkey" with a number | The same, untranslated, since names aren't declined or translated |
