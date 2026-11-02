# Arcade machines

Sources: `luna:code/game/machinery/computer/arcade.dm` (`New`, `Topic`, `arcade_action`,
`vend_prize`, `reset`), `luna:code/game/objects/items/toy.dm`; `NSV_Luna.dmm`: 7 on the decks.

Code: `game/classic/src/arcade.rs`, `apps/client/src/arcade_panel.rs`. Tests: unit tests in
`arcade.rs`, `game/classic/tests/arcade.rs`.

A dense machine that needs equipment power (50 W). Clicking one opens its window, which closes if
you walk away or it loses power. Each machine has its own villain, picked at the start of the
round from a title and a name, and keeps its game going whoever plays.

## The game

You start with 30 health and 10 magic, the villain with 45 health and 20 magic. Pick a move, and
the villain answers a second later. Until then, nothing else can be done.

| Move | Effect |
|---|---|
| **Attack** | The villain loses 2 to 6 health |
| **Heal** | You spend 1 to 3 magic and gain 6 to 8 health |
| **Recharge Power** | You gain 4 to 7 magic |

The villain's answer, in order:

1. If it has no health or no magic left, you win, and the machine drops a random toy.
2. If its magic is 5 or less, 70% of the time it steals 2 or 3 of your magic. If you have none left,
   you lose.
3. Otherwise, if its health is 10 or less and its magic above 4, it heals 4 for 4 magic.
4. Otherwise it attacks for 3 to 6.

You also lose at below 0 magic or with no health left. **New Game** starts over against the same
villain.

## Differences from Luna

| Luna | Here |
|---|---|
| About 15 kinds of prize | Three toy mechs, as the rest need things we don't have yet |
| An emag makes it deadly, and a clown stamp honks it | Not built |
| Its name is a call to action: "Defeat Lord Jeff" | Objects don't take names yet, so it's an arcade machine |
