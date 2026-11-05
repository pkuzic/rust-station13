# The nuclear emergency

Luna's `code/game/gamemodes/nuclear`, in `game/classic/src/nuke.rs`.

## Setting up

One player in three is on the team, at least one and at most five, picked first from those who said in the lobby they'd like to be.
They lose their job and what they arrived with, and start at Luna's Syndicate base in a jumpsuit, black shoes, SWAT gloves, a backpack
and a Syndicate headset (`:t`). The first of them is the leader, renamed Syndicate Czar, Boss, Commander, Chief, Kingpin, Director or
Overlord, and alone told the five-digit code.

The disk lies at one of Luna's four disk spawns on the station. The base gets the team's closet (handcuffs, three energy guns, three
pinpointers and an uplink with 10 crystals for each agent), four gear closets (suit, helmet, jetpack, breath mask, jumpsuit, power cell)
and four charges of C-4 (two strong).

Luna's map has no landmark for the bomb, so its mode never places one: here it goes on the floor beside the team's closet.

## The bomb

A click on the folded bomb unfolds it and bolts it down. Later clicks open its window:

- **Disk**: puts in the disk from your active hand, or takes it out to the floor, which forgets the code.
- With a disk in, the keypad works. More than five digits, or a wrong **E**, shows ERROR. **R** clears it and forgets the code.
- With the right code entered, the timer (5 to 600 seconds), safety and anchor work.

Unanchored, it can be pulled. When the timer runs out with the safety on it just stops. With it off, it can't be stopped, and it blows
two seconds later. Taking out the disk doesn't stop a running timer.

## Pinpointer

In the hand, it switches on and off. Its needle points at the disk, its state showing how far: same tile, up to 8 tiles, up to 16, or
further. Switching it on says whether the disk's on a deck above, below or this one.

## How it ends

- **The bomb goes off**: the Syndicate wins and the round ends.
- **The pods arrive with the disk aboard**: the crew wins.
- **The pods arrive without it**: neutral, as Luna calls it.

A bomb an admin spawns (`spawn nuclear-bomb`) has Luna's code "ADMIN", which no keypad can type.

## Differences from Luna

| Luna | Here |
|---|---|
| A "make deployable" verb has to be used before the first click does anything, which the guide never mentions | The first click unfolds it |
| The pinpointer's overlay shows the disk's deck all the time | Said once, when switched on |
| Agents carry a cyanide pill, and the closets hold a C-20r, plastique, flashbangs, a Syndicate PDA, Syndicate belt and ID | Left out: none of these are built yet |
| Any mob's cinematic plays | A line of text |
| An intercept report reaches the crew a few minutes in | Not built |
