# Jobs

Sources: `luna:code/game/jobs/jobs.dm`, `jobprocs.dm` (`DivideOccupations`, `Equip_Rank`), `access.dm`
(`get_access`).

Code: `game/classic/src/jobs.rs`, `game/classic/content/jobs.ron`. Tests: `game/classic/tests/jobs.rs`.

Each start point on Luna's map is for a job. Whoever starts there gets that job: its ID card worn with the
job's access, its clothes, the tools it starts with in the hand not in use and in a backpack, and a message saying what
they are. The captain's card has every access.

There are 26 jobs, from captain to assistant. Each new player gets the job fewest people have, the earlier
in `jobs.ron` the better, so a small round gets an engineer, a doctor and a security officer first, and
nobody doubles up until every job with a free start point is taken. A card waiting at an empty start point
can't be seen or taken.

A respawn is offered a job the same way. Once every post with a card has been taken, it starts at any free start
point with no job, in a grey jumpsuit and black shoes.

What each job opens is in `jobs.ron` only. The map's start points just name a job, and its card takes that job's
access when it's placed.

Maps without jobs on their start points work as before: a grey jumpsuit, black shoes, and no card.

## Choosing

Someone arriving is offered a job and sees a window listing every job and how many posts each has free, with
those that have none greyed out. Until they choose they aren't in the world: nobody can see them, and they can't
move or do anything, but the post they were offered is held for them. Choosing starts them at a post of that job,
or the offered one, with its card and kit. A job whose last post has just gone leaves the window open. After 60
seconds they start in the job they were offered.

Bots that never answer start after those 60 seconds too. On maps without jobs nobody is asked.

## Differences from Luna

| Luna | Here |
|---|---|
| Players set which jobs they prefer before the round, and it divides them up | Each chooses as they arrive, from what's free |
| Every job has a PDA and most a department headset | Only engineers' headsets exist here |
| Each job has its department's backpack, its own hat or armour, and some have grenades, implants and so on | Only what exists here. See `jobs.ron` |
| A card and PDA carry the person's name and job title | The card only carries access |
| Records and a crew manifest list everyone's job | Not built |
| The AI, and arrivals after the round started | Not built |
