# Reference spec: mob movement

Sources:
- `luna:code/modules/mob/mob.dm` — `/client/Move` (lines ~1906–2030)
- `luna:code/WorkInProgress/NewMobs and etc/Damagesystem.dm` — `/mob/living/carbon/human/movement_delay()`
- `luna:code/Ticklag.dm` — `world.tick_lag = 0.5`

## Luna behaviour

Time is `world.time`, in deciseconds (ds). Luna runs at `tick_lag = 0.5` ds, i.e. 20 ticks
per second, the same as our `TICK_RATE_HZ`.

A move is refused while `world.time < move_delay`. After a successful move from a turf:

```
move_delay = world.time
           + intent delay   (run: 1, walk: 4, run while drowsy: +4)
           + jetpack in no gravity: +5
           + mob.movement_delay()
```

Human `movement_delay()` starts at **1.3** and adds:

| Condition | Extra delay (ds) |
|---|---|
| health deficiency ≥ 40 | deficiency / 40 |
| suit / shoes | their `slowdown` |
| each broken leg or foot | +2 |
| each missing leg or foot | +3 |
| robotic limb | −1 (never below 1.3) |
| body temperature below 283.222 K | (283.222 − T) / 20 × 1.25 |
| zombie | returns 6 |
| nuka cola, hyperzine, on a space turf | returns 0 |

So a healthy running human waits 2.3 ds. Because moves can only happen on 0.5 ds ticks,
the real step period is **2.5 ds (5 ticks): 4 tiles per second**. Walking: 5.3 → 5.5 ds.

Other rules in `/client/Move` not yet implemented: restrained mobs cannot move while being
pulled; broken legs randomly stop a move and stun; grabbing someone sets at least 7 ds.

## Implemented (`game/classic/src/movement.rs`)

- Run / walk intent delays and the 1.3 ds human base. **Keep.**
- A step is refused while `now < next_move_ds`; `now` advances 0.5 ds per tick, so the
  first step happens immediately and then every 5 ticks while running. **Keep.**
- Solid tiles and other mobs block movement. Mobs never share a tile. (Luna lets the
  `help` intent swap places; not implemented yet.)
- A blocked diagonal tries its horizontal component, then its vertical one.
  **To verify** against BYOND's `Move()` for diagonal steps.
- Mobs are processed in entity order each tick, so results are deterministic.

## Not implemented yet

Health, clothing slowdown, temperature, space drift and zero-gravity movement, pulling,
grabs, restraints, place swapping.

Tests: `game/classic/src/movement.rs`, `game/classic/tests/two_mobs_in_a_room.rs`.
