# Explosions

Sources: `luna:code/game/objects/explosion.dm`, `luna:code/game/turf.dm` (`ex_act`), `luna:code/game/machinery/main.dm`,
`luna:code/game/objects/structures.dm`, `luna:code/WorkInProgress/NewMobs and etc/Damagesystem.dm` (`human/ex_act`),
`luna:code/game/objects/tank.dm`, `luna:code/game/objects/items/weapons/syndie.dm`. Code:
`game/classic/src/explosions.rs`. Tests: `game/classic/tests/explosions.rs`.

An explosion has four ranges: devastation, heavy, light, and flash. Half a second after it's set off the blast spreads out
from the middle, and every tile costs distance to pass through: open floor 1, a wall 4, a reinforced wall 5, a closed door
3 more, a window 1 more, and through the floor to the next deck 6. So walls and doors shelter what's behind them. A blast
with a devastation range of 1 or less goes by plain distance, walls or not, as Luna's does.

| Where | Severity | Luna's name |
|---|---|---|
| Closer than devastation | 1 | |
| Up to heavy | 2 | |
| Up to light | 3 | |

| What | 1 | 2 | 3 |
|---|---|---|---|
| Wall | Gone, to space | Down to plating with 2 sheets of metal | The same, 20% |
| Floor | Gone, to space | 64%: gone. Otherwise plating, and it catches. A third leave a sheet of metal | 50%: plating, and it catches |
| Item | Gone | 50% | 5% |
| Machine | Gone | 50% | 25% |
| Anything else | Gone | 50% | Untouched |
| Person | Dead, unless armour against bombs saves them, and then 125 damage | 60 brute and 60 burn, halved by armour. 70%: out for 20 seconds | 30 brute, a quarter of that with armour. 50%: out for 20 seconds |

What was inside something destroyed is left on the floor. Anyone within flash range is dazzled.

## Gas tanks

Once a second what's in a tank burns if it can. Over 30 atmospheres a tank leaks, after holding for a few seconds: a quarter
of its gas each second. Over 40 it spills everything and is gone. Over 50 it blows up: its range is one tile for each 10
atmospheres over, up to 14, with devastation, heavy, light and flash at 0.24, 0.5, 1 and 1.5 of that.

## C4

The uplink sells two charges, each with a detonator that looks like a Zippo. Use the detonator in hand to open it, and again to
press the button, from anywhere. 5 seconds later the charge goes off, with ranges of 1, 2, 3 and 4 times its power (1, or
2 for the heavy one). The walls on its four sides come down, and doors there fly open.

## Differences from Luna

| Luna | Here |
|---|---|
| A wall or floor gone at severity 1 opens onto the deck below | It opens to space: open air between decks doesn't exist yet |
| The deck above takes damage too | Only through the blast spreading between decks |
| A person at severity 1 is gibbed | They die where they are |
| Ears ring, and deafness | Not built |
| A sound, far off for everyone, and the explosion's picture | Only a chat line |
| Machines take particular damage rather than vanishing, and some items, like tanks, go off themselves | All by the table above |
| Lattice is left where a floor went | Just space |
| Window edges still hold air after the window has gone | The same stale edge, for now |
| Bombs of a tank with a timer or signaller (3.20), grenades, EMP | Not built |
| Doppler arrays report blasts | Not built |
