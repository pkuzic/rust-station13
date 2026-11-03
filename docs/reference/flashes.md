# Flashes and flashers

Sources: `luna:code/game/objects/items/devices/flash.dm` (`attack`, `attack_self`),
`luna:code/game/machinery/flasher.dm` (`flash`, `portable/HasProximity`, `attackby`);
`NSV_Luna.dmm`: 10 flashes, 5 mounted flashers in the brig cells, 3 portable ones in the armory.

Code: `game/classic/src/flashes.rs`. Tests: `game/classic/tests/flashes.rs`.

## Being flashed

Someone flashed is **knocked down**: they lie there and can't move or act until it wears off, and
their view whites out for a moment. They can still talk. Sunglasses or a welding helmet protect
them entirely.

| Source | Knocked down for |
|---|---|
| Flash | 20 s |
| Mounted flasher | 20 s |
| Portable flasher | 16 s |

## Flash (`flash`)

- **On someone in reach:** knocks them down, and everyone who can see is told. If it worked, the
  flash has a 10% chance of burning out.
- **In your hand:** everyone who can see you within 3 tiles has their view whited out, but stays up.
- **Burnt out:** it only clicks.

## Flashers (`flasher`, `portable-flasher`)

When one goes off, it knocks down everyone within 4 tiles who can see it and draws 1 kW for a
moment. It needs power, and rests 15 s between flashes.

- **Wirecutters** disconnect or reconnect its bulb.
- **Portable:** dense. It starts loose and does nothing until wrenched down, shown by a light on
  it. Then it goes off when anyone runs next to it; walking past doesn't set it off.
- **Mounted:** the brig cell's door timer sets these off (`brig.md`).

## Differences from Luna

| Luna | Here |
|---|---|
| The flash checks `!status` where `status` starts at 1, so as written it only ever clicks on people | It works, as the code clearly meant |
| A shot count that refills every time, so it never runs out | No shot count |
| Repeated flashes damage the eyes and can blind for good | Eyes aren't simulated |
| Robots' flash, clumsiness, revolutionaries | Not built |
