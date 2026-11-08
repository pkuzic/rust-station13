# Random events

Sources: `luna:code/WorkInProgress/Events/Event.dm`, `EventsMain.dm` and `Events/*.dm`;
`luna:code/game/gamemodes/gameticker.dm` (the timer), `luna:code/defines/procs/command_alert.dm`,
`luna:code/game/objects/portals.dm` (the wormholes), `luna:code/modules/admin/admin_verbs.dm` (the
admins' switches).

Code: `game/classic/src/events.rs`. Tests: `game/classic/tests/events.rs`.

## When

Once a round is under way, every 1000 seconds one of the events below is picked at random, all
equally likely. Nothing starts while one is still running. Each runs for its lifetime, acting
once a second, then ends. Most are announced to everyone from Central Command (Luna's
`command_alert`), under a heading, as they start and as they end.

Admins (`admin.md`): `toggleevents` turns them off or on; `event NAME` starts one now; `event`
alone, for the Host, starts one at random.

## The events

| Event (`event` name) | Lifetime | What happens |
|---|---|---|
| Electrical storm (`electrical-storm`) | 30 to 120 s | Announced, start and end. Each second, twice: half the time a working light fixture blows. One time in five, a working area power controller is picked, and 40% of the time its breaker trips. One time in five, an airlock is picked: its bolt wire is pulsed, then two others at random (`airlock.md`), so it may bolt, lose power, open or electrify |
| Gas leak (`gas-leak`) | 10 s | No announcement. One to three canisters of air, oxygen or nitrogen (never plasma, nitrous oxide or carbon dioxide) open their valves to between one and ten atmospheres. One time in ten, one bursts instead and lets everything out |
| Ion storm (`ion-storm`) | 30 to 120 s | Nothing: in Luna it changes the AI's laws, and there's no AI yet (3.9) |
| Meteor shower (`meteor-shower`) | 30 to 120 s | Announced, start and end. Each second, 40% of the time, a wave of three meteors as in a meteor round (`meteor-blob.md`), no more than one a minute |
| Portal storm (`portal-storm`) | over at once | Announced. A fifth of the station's floors each get a wormhole, opening at a random moment over the next five minutes and lasting 30 seconds. Whoever and whatever loose comes onto one, or clicks it, comes out on or beside a random floor anywhere on the station |
| Grid check (`power-offline`) | 30 to 120 s | Only the announcement: Luna turned off what it did, after bug reports |
| Radiation belt (`radiation-belt`) | 10 s | Announced, start and end. Each second, every person and monkey gets 1 to 5 rads, and 5% of the time 1 to 10 more (`genetics.md`). A person, 2% of the time, has a gene changed, for the worse or the better as often |
| Epidemic (`viral-infection`) | over at once | Nothing yet: it needs viruses (3.4) |

## Differences from Luna

| Luna | Here |
|---|---|
| A storm's blown light flares up first | It just breaks |
| A storm sometimes breaks an area power controller for good | Only the breaker trips: APCs can't break yet |
| A wormhole is dense: walking into it throws you through | Stepping onto it does |
| A wormhole is pictured with Luna's anomaly icon | The same |
| The portal storm's code ends in an error, so the event never closes properly | It ends cleanly |
