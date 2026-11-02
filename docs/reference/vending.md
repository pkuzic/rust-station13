# Vending machines

Sources: `luna:code/game/machinery/vending_tg.dm` (`attack_hand`, `Topic`, `power_change`,
and the `coffee`, `snack`, `cola`, `cola/soda` and `cigarette` subtypes); `NSV_Luna.dmm`:
15 hot drinks machines, 15 snack machines, 15 soda machines and 12 cigarette machines.

Code: `game/classic/src/vending.rs`, the window in `apps/client/src/vending_panel.rs`.
Tests: `game/classic/tests/vending.rs`. What they sell: `docs/reference/food.md`.

## The machines

Dense, on the equipment channel of their area's APC, and full when the round starts:

| Prototype | Luna type | Sprite | Delay | Sells |
|---|---|---|---|---|
| `vending-coffee` | `coffee` | `coffee` | 3.4 s | Robust Coffee, Duke Purple Tea, Dutch Hot Coco: 25 each |
| `vending-snack` | `snack` | `snack` | 1 s | candy, Cup Ramen, chips, Scaredy's Private Reserve Beef Jerky, 4no raisins, space twinkie, cheesie honkers: 6 each |
| `vending-cola` | `cola` | `Cola_Machine` | 1 s | Space Cola, Space Mountain Wind, Dr. Gibb, Star-kist, Space-Up: 10 each |
| `vending-soda` | `cola/soda` | `soda` | 1 s | as `vending-cola` |
| `vending-cigarette` | `cigarette` | `cigs` | 3.4 s | cigarette packets and matchboxes: 10 each; cheap lighters: 4 |

## Using one

- **Clicking one** within reach, while it has power, opens its window: each product, how
  many are left, and **Vend**, or **Sold Out**.
- **Vend** takes one off the count and, after the machine's delay, drops it on the machine's
  tile. It hands out one thing at a time; a second Vend meanwhile does nothing.
- **Without power** it shows its `-off` sprite and won't open; an open window closes.
- **The hot drinks machine** plays Luna's `coffee-vend` animation while it works.

## Differences from Luna

| Luna | Here |
|---|---|
| Coins unlock premium stock; a multitool or wirecutters on its wires unlock contraband, shock or make it throw things | Not built: only the ordinary stock |
| It speaks its slogans, and some greet a sale | Not built |
| It uses 5 W per sale | It draws nothing, but needs its area's equipment power |
| Explosions and the blob break it (`-broken`) | Not built |
