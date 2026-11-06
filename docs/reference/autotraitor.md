# AutoTraitor

Luna's `gamemodes/extended/extended_traitor.dm` (Extend-A-Traitormongous, mode id `autotraitor`): a
traitor round that keeps adding traitors as the round goes on. It's built on the traitor round
(`traitor.md`): the same objectives, briefing and uplink.

Luna's `config.txt` leaves it out, so it keeps the default chance of 1 when the round is picked at
random. It can be voted for.

- At the start, with `p` players: at most `p / 10 + 1` traitors. It picks one fewer than that, and one
  more with a chance of `(p - (most - 1) × 10) × 10` percent. Those who'd like to be a traitor are
  picked first, as in the traitor round.
- Everyone's told it's a test bed for an endless traitor round, after the usual words of welcome.
- Every 15 minutes, until the pods leave, it counts the living players and the living traitors.
  - It may add one more traitor below the most, with a chance of `(p - (most - 1) × 10) × 5` percent.
  - The chance is 50 more while the traitors are two or more short.
  - The new traitor is picked at random from the living who'd like to be one and aren't yet.
  - They're told it's time to pay their debt to the Syndicate, given objectives, and given an uplink.
- Someone joining late who'd like to be a traitor gets the same roll, and is told they're a traitor.
- The round ends, and is reported, as the traitor round does.

## Differences from Luna

| Luna | Here |
|---|---|
| Counts any living player with a special role as one of the traitors | Only living traitors count (no other special roles exist in this round) |
| Lets the dead respawn (`abandon_allowed`) | As in every round here |
| A round with nobody to make a traitor fails to start | It goes ahead with none, and adds them later |
