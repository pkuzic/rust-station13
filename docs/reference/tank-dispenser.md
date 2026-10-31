# Reference spec: tank storage unit

## What Luna does

Source: `luna:code/game/machinery/dispenser.dm` (`structure/dispenser`).

A dense, anchored rack holding up to ten oxygen tanks and ten plasma tanks. The
`/oxygen` variant, on Luna's engineering map, starts with ten oxygen tanks and no plasma.

- **Hand:** opens a window with "Dispense" links. Dispensing gives back the tanks people put in first. Otherwise it makes a new tank, either way dropping it on the unit's tile.
- **Putting tanks back:** oxygen, air and anaesthetic tanks go in while it holds fewer than eight (a bug: it starts with ten). Plasma tanks go in up to ten.
- **Sprite:** `dispenser`, with an overlay `oxygen-5` when full, otherwise `oxygen-(n/2 + 1)` with BYOND's rounding down, and none when empty. The plasma overlay checks the oxygen count by mistake.

## What we build (`game/classic/src/dispensers.rs`)

| Item | Ours | From Luna |
|---|---|---|
| Tanks | ten oxygen tanks, dense, anchored | keep; no plasma tanks (not on the engineering map, and no plasma tank item yet) |
| Empty hand | takes a tank into that hand: one put in earlier (newest first), else a new full one | **simplified**: Luna's window, and it drops the tank on the floor |
| Oxygen tank in hand | goes in while it holds fewer than ten | **fixed**: Luna stops at eight |
| Overlay | `oxygen-5` full, else `oxygen-(n/2 + 1)`, none when empty | keep |
| Examine | how many tanks it holds | ours |
| Air and anaesthetic tanks | none | no such items yet |
