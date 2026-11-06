# Changelings

Luna's `code/game/gamemodes/changeling`, in `game/classic/src/changeling.rs`. Luna's `config.txt` leaves it out, so it keeps the
default chance of 1 when the round is picked at random; it can also be voted for or forced.

## Setting up

One player becomes it, picked first from those who'd like to be traitors. Their objectives are thefts and killings until the work adds up,
taking four to six genomes (never more than there are others), and either getting away (seven in ten) or staying alive.

## What they have

**F6** opens the window. They start with 20 chemicals of a possible 50, recovering by 0.5 every 2 seconds, and 5 genome points. Genetic damage
goes down by 1 every 2 seconds, and only blocks changing form. The window lists every power: those not owned can be evolved with points, and
those owned used with chemicals.

| Power | Points | Chemicals | What it does |
|---|---|---|---|
| Absorb DNA | free | 0 | With someone grabbed in the active hand: three stages of 15 seconds, holding still, 40 brute at the third. They die, unclonable. +10 chemicals, +2 points, their genome |
| Transform | free | 5 | Look like and be named after a genome. 30 genetic damage |
| Regenerative Stasis | free | 20 | Works even dead. Lie still for 80 to 200 seconds, then fully healed if there's still 20 chemicals |
| Hive Channel | free | 10 | Put a genome on the airwaves |
| Hive Absorb | free | 20 | Take one from them. Doesn't count as absorbed |
| Lesser Form | 1 | 1 | Everything falls off, and you're a monkey until you Transform. 30 genetic damage |
| Anatomic Panacea | 1 | 25 | Clears toxins and suffocation |
| Deaf Sting | 1 | 5 | 30 seconds of hearing no one speak |
| Blind Sting | 2 | 20 | A flash of white. See below |
| Silence Sting | 2 | 10 | 100 seconds unable to speak, with no warning |
| Mimic Voice | 3 | 1 per 4 s | Speak as a name you type |
| Extract DNA Sting | 4 | 40 | Their genome, without harm |
| Transformation Sting | 3 | 40 | They look like one of your genomes |
| Paralysis Sting | 3 | 30 | Knocked down for 40 seconds |
| Death Sting | 10 | 40 | Silent and out for a while, with lexorin in their blood |
| Ranged Sting | 2 | 10 | The next sting reaches two tiles |
| Epinephrine Sacs | 4 | 45 | Up at once, with synaptizine in the blood |
| Rapid / Advanced Chemical-Synthesis | 4 / 8 | - | Each doubles recovery |
| Engorged Chemical Glands | 4 | - | 25 more storage |
| Digital Camouflage | 3 | 1 per 4 s | Toggled. There's no AI yet to hide from |
| Rapid Regeneration | 8 | 30 | 10 of each damage every 2 seconds, 10 times |

Stings don't warn the victim, unless it says so above, and do nothing to another changeling but give them a prick. Absorbing another changeling
takes all their genomes, powers, points and chemicals.

Starting a line with `:g` speaks on the hive channel, under a name like "Ms. Theta": all changelings and ghosts hear it.

## Dying

A dead changeling stays in their body, since stasis can bring them back, and the window offers to give up and leave it instead. Dead for 20 minutes
in a changeling round, and the round ends.

## Differences from Luna

| Luna | Here |
|---|---|
| Hallucination Sting | Left out: nothing here hallucinates yet |
| Blind Sting blinds and blurs for a while | Whites out the screen once: no blindness yet |
| Digital Camouflage changes how you look when examined, and hides you from the AI | Only costs chemicals: neither's built yet |
| Lesser Form is a real monkey, which can't do what a human can | Looks like one, but keeps a person's abilities, and the powers it can use aren't limited |
| The grab must be "tighter", but the check is written so that any will do | Any will do |
| Silence is a countdown of life ticks and its own flag | Uses the same mute as an admin's, so it clears an admin's mute when it ends |
