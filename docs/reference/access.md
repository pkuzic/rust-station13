# Reference spec: access and ID cards

Sources:
- `luna:code/game/jobs/access.dm`: access constants, `allowed`, `check_access`, `get_access`
- `luna:code/game/machinery/doors/door.dm`: `attackby` (deny animation)

## Luna behaviour

**Access ids** are numbers (`access_engine = 10`, `access_engine_equip = 11`, ...). Objects
list the ids they require in `req_access`; ID cards list the ids they grant in `access`.
Jobs get fixed lists from `get_access(job)` (Engineer: engine, engine_equip, tech_storage,
maint_tunnels, external_airlocks, and more; Unassigned: maint_tunnels).

**`allowed(mob)`.** With no requirements, anyone is allowed. Silicons always are. A human is
allowed if the item in the **active hand** or the card in the **ID slot** (`wear_id`)
passes `check_access`. Monkeys and aliens can only use a held card.

**`check_access(card)`.** A PDA counts as its inserted ID. Then the card needs **any one**
of the required ids. A comment in Luna says this was deliberately changed from needing all
of them, so it is a local Luna rule, not upstream /tg/.

**Doors.** Using a closed door without access plays `door_deny` and nothing else happens. No
chat message is shown. Bumping goes through the same path.

## Implemented (`game/classic/src/access.rs`, `doors.rs`)

| Behaviour | Status |
|---|---|
| Access ids as readable names (`engine_equip`) with the Luna number in `content/access.ron` | keep (data format) |
| No requirements: anyone | keep |
| Any one matching id is enough | keep (Luna-specific rule) |
| Card in the active hand counts, for use and for bumping | keep |
| Unknown access ids on the map are reported at load time | keep (validation) |
| Denied closed door: `deny` appearance for 3 ds, no message | keep; **3 ds is an estimate** until the sprite's frame delays are imported |
| Map objects carry `access`: required (airlocks) or granted (ID cards) | keep |

## Not implemented yet

The ID slot (`wear_id`), PDAs, silicons, job access lists assigned at spawn, card owner
names and job titles, the ID computer, emags.

## The identification computer (`id-computer`)

Source: `luna:code/game/machinery/computer/card.dm`. Code: `game/classic/src/id_computer.rs`. Tests:
`game/classic/tests/id_computer.rs`.

Clicking one with a card in hand puts it in the slot. The window, which needs power, shows whose the card is
and its job, and lists all the jobs. Pressing one gives the card that job's access and title, if the card
you wear has `change_ids`, as the Head of Personnel's and captain's do. Anyone can eject the card.

Examining a job's card says whose it is and what job it's for.

Differences from Luna: you can only give a card a whole job's access, not tick off single accesses, or type a
custom title or name. Your own card has to be worn, not put in a second slot. It doesn't change what the
person's job is in the records, only their card. A crew manifest isn't built.
