# Jobs

Sources: `luna:code/game/jobs/jobs.dm`, `jobprocs.dm` (`DivideOccupations`, `Equip_Rank`), `access.dm`
(`get_access`).

Code: `game/classic/src/jobs.rs`, `game/classic/content/jobs.ron`. Tests: `game/classic/tests/jobs.rs`.

Each start point on Luna's map is for a job. Whoever starts there gets that job: its ID card worn with the
job's access, its clothes, the tools it starts with in hand and in a backpack, and a message saying what
they are. The captain's card has every access.

There are 26 jobs, from captain to assistant. Each new player gets the job fewest people have, the earlier
in `jobs.ron` the better, so a small round gets an engineer, a doctor and a security officer first, and
nobody doubles up until every job with a free start point is taken. A card waiting at an empty start point
can't be seen or taken.

A respawn starts at the first free start point. If that one's card has gone it has no job, and wears a grey
jumpsuit and black shoes.

Maps without jobs on their start points work as before: a grey jumpsuit, black shoes, and no card.

## Differences from Luna

| Luna | Here |
|---|---|
| Players set which jobs they prefer, and the round divides them up | No choice yet |
| Every job has a PDA and most a department headset | Only engineers' headsets exist here |
| Each job has its department's backpack, its own hat or armour, and some have grenades, implants and so on | Only what exists here. See `jobs.ron` |
| A card and PDA carry the person's name and job title | The card only carries access |
| Records and a crew manifest list everyone's job | Not built |
| The AI, and arrivals after the round started | Not built |
