# Derelict

Luna's `gamemodes/derelict` (`derelict.dm`, `wreak.dm`), in `game/classic/src/derelict.rs`. Tests:
`game/classic/tests/derelict.rs`.

The station is found wrecked and dark. Everyone comes aboard on the arrival shuttle as a scavenger,
with tools to bring it back. Luna's `config.txt` leaves the mode out, so it keeps the default chance of 1
when the round is picked at random. It can be voted for. The round ends when the pods arrive.

## The wreck

Before anyone is aboard, across every deck:

- **APCs:** every one is flat. A quarter lose their cell. Of the rest, 40% are broken: a broken APC
  powers nothing, charges nothing, and shows Luna's broken sprite.
- **Floors:** each is hit as by a light blast 70% of the time, so about a third end up as plating.
  Now and then (one tile in 2 000) a big explosion goes off (3, 5, 7, 9).
- **Walls:** each is hit as by a light blast 40% of the time, so one in 12 or so is left a girder.
- **Window panes:** each is hit twice, at 40% and then 65% (Luna's "hacking airlocks" step smashes
  windows again). A hit breaks the pane half the time.
- **Air alarms:** 80% broken.
- **Loose things:** each is shoved 2 to 6 times in random directions, and one in ten is gone.

A tile with Luna's `derelict/nodamage` landmark is spared the per-tile hits and the alarm and pane
damage. A tile with `derelict/noblast` never sets off a big explosion. Neither stops a big explosion
that has gone off nearby.

Then the supply landmarks put things out:

| Landmark | What |
|---|---|
| `glass`, `metal` | A stack of 5 to 50 sheets |
| `o2canister` | An air canister (40%) or an oxygen one |
| `o2crate` | A crate of three gas masks and three oxygen tanks |
| `superpacman` | A S.U.P.E.R.P.A.C.M.A.N. and 100 sheets of uranium |
| `supplycrate` | Four engineers' emergency oxygen tanks, two each of tool belts, insulated gloves, multitools and magboots, a toolbox of each kind, and an empty engineering crate |

The S.U.P.E.R.P.A.C.M.A.N. works like the P.A.C.M.A.N. (`generators.md`), but burns uranium, puts out
15 000 W, and makes a sheet last 160 cycles.

## Scavengers

Everyone in the round, and anyone who joins later, comes aboard as a scavenger:

- **Removed:** what they had on, apart from their backpack and what's in their hands.
- **Worn:** a headset, a firesuit, a black jumpsuit, a gas mask, a tool belt and a red hard hat, plus a
  backpack if they had none.
- **Packed:** an engineers' emergency oxygen tank, a box of seven circuit boards, and two high-capacity
  cells.
- **ID card:** in their name, saying "Scavenger".
- **Random:** a mechanical toolbox in the left hand (55%) or an electrical one; magboots (50%) or black
  shoes; a multitool (45%); insulated gloves (40%).

They're put on a random clear tile of the arrival shuttle.

## Differences from Luna

| Luna | Here |
|---|---|
| The emergency tank and the multitool go in the pockets | In the backpack: there are no pockets yet |
| A broken pane leaves a shard, and rods if it was reinforced | Nothing: there are no shards or rods yet |
| A broken pane's edge lets air through | It stays sealed, as when a pane is taken off with a wrench (`windows.md`) |
| Bots are deleted | There are no bots |
| The captain's spare ID is never lost | There's no spare ID on the map yet |
| Things in the `hangar/derelict` area are moved to the hangar's level | Not done: neither area is in our maps |
| The lobby is told "Derelict takes time to load" and each step of the wreck | Not said |
| The AI is turned off for the round | There's no AI yet |
