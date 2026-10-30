# Reference spec: doors and airlocks

Sources:
- `luna:code/game/machinery/doors/door.dm`: `CanPass`, `attackby`, `open`, `close`, `autoclose`
- `luna:code/game/machinery/doors/airlock.dm`: `Bumped`, `open`, `close`, `attack_hand`

Times are in deciseconds (ds); 1 tick = 0.5 ds.

## Luna behaviour

**Passing.** A door blocks movement while `density` is set. Glass doors let light through but
still block movement.

**Using (`attack_hand` → `attackby`).** While `operating`, nothing happens. Otherwise, if the
user is allowed (ID access): a closed door opens and an open door closes. If the user is not
allowed, a closed door plays the `door_deny` animation. There is no chat message either way.

**Bumping (airlock).** A mob with a client that walks into a closed airlock acts as if it
used the airlock with its hand. This is ignored if the panel is open, the door is operating
or already open, the mob is handcuffed, or the mob bumped something within the last 5 ds.

**Opening (airlock).** Refused if welded, bolted (`locked`), unpowered or the open-door
wire is cut. Uses 50 W, plays `airlock.ogg`, and closes the paired `closeOther` door.
Then (door): it is `operating`, plays the "opening" animation, waits **6 ds**, clears
`density` and opacity, and stops operating. If `autoclose` is set, it calls `autoclose()`
**150 ds** later.

**Closing (airlock).** Refused for the same reasons. Then (door): it is `operating` and
sets `density` **immediately**. After **4 ds**, every living mob on the tile is pushed to a
random adjacent free tile with "The airlock forces you out of the way!" (or crushed, if
`forcecrush`), and loose items are pushed the same way. At **6 ds** it becomes opaque
and stops operating.

**Autoclose.** It closes only if the door is open, not operating, not bolted and not
welded. There is a single check, 150 ds after opening.

## Implemented (`game/classic/src/doors.rs`)

| Behaviour | Status |
|---|---|
| Closed doors and closing doors are dense; opening doors stay dense until done | keep |
| Use with a hand or with any held item toggles; ignored while operating | keep |
| Bolted or unpowered: refuse to open or close, no message | keep |
| Open: 6 ds, then not dense; autoclose check at +150 ds | keep |
| Close: dense at once; at +4 ds push mobs and items out; done at +6 ds | keep |
| Player-controlled mobs open a closed airlock by walking into it, 5 ds bump cooldown | keep |
| Pushed-out destination: **first free tile in the order N, S, E, W** instead of random | **fix**: deterministic for tests and replays |
| "The airlock forces you out of the way!" notice to the pushed mob | keep |
| A lit welding tool welds a closed door shut, or open again; welded, it opens for nothing, not even a crowbar (`welding.md`) | keep |

## Not implemented yet

Wires, the maintenance panel, electrification and bolts are covered in `wires.md`. Still
missing: emag, crushing, `closeOther` pairs and sounds.
