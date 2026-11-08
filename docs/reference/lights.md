# Lights

Sources: `luna:code/modules/power/lighting.dm` (`/obj/machinery/light`, `light/small`,
`/obj/item/weapon/light`, `storage/box/light`) and
`luna:code/game/machinery/lightswitch.dm`.

Code: `game/classic/src/lights.rs`. Tests: `game/classic/tests/lights.rs`.

## Fixtures

- **Kinds:** a light fixture (`light`) takes a tube. A small fixture (`light-small`) takes a
  bulb.
- **Facing:** each hangs on a wall and faces it (Luna's default is south). Its sprite shows
  the fitting, its state and the facing, e.g. `tube1-north`.
- **The tube or bulb:** it works, is missing, is burnt out or is broken.
- **Small lights:** one in twenty starts with a broken bulb.

### When a light is lit

A light is lit while all three hold:

- it has a working tube or bulb;
- its area's lighting channel has power (`power.md`);
- its area's light switch is on.

- **Power:** a lit light draws 25 W per unit of brightness on the lighting channel: a tube
  (brightness 8) 200 W, a bulb (5) 125 W.
- **Burning out:** each time a light goes on or off, it may burn out. The chance in percent
  is the square of how many times this tube has switched, times 0.001, up to 60. That's
  about 10 % on the hundredth switch.

## Light switches

- **Clicking:** a light switch flips every light in its area on or off, powered or not.
  Several switches in one area share the setting. Areas start with their lights on.
- **Sprite:** `light1` (on), `light0` (off), or `light-p` when the area's lighting has no
  power.

## Clicking a fixture

| With | The fixture | Does |
|---|---|---|
| empty hand | empty | "There is no light tube in this light." |
| empty hand | lit, no gloves | "You try to remove the light tube, but burn your hand on it!": 5 burn to the active hand. It stays in. |
| empty hand, harm intent | lit, no gloves | "You smash the light tube, but burn your hand on it!": 2 brute and 6 burn to the hand, and it breaks. |
| empty hand | otherwise | "You remove the light tube.": into the hand, as it was (working, burnt out or broken) |
| the right tube or bulb | empty | "You insert the light tube." |
| the right tube or bulb | burnt out or broken | "You replace the light tube.": the old one drops under it |
| the wrong one | empty, burnt out or broken | "This type of light requires a light bulb." |
| anything else | working or burnt out | a (1 + force) × 12 % chance: "You hit the light, and it smashes!", seen by others. Otherwise "You hit the light!". Smashing a lit light with metal may shock (50 %). |
| anything else | empty | "You stick the wrench into the light socket!" Metal in a powered socket may shock (75 %). |

- **Gloves:** any gloves protect from the heat. In Luna every pair of gloves passes a
  quarter of the heat.
- **Metal:** the tools (crowbar, wrench, screwdriver, wirecutters, multitool and welding
  tool) carry a shock, as Luna's `CONDUCT` flag (`conducts` in `content/items.ron`).
- **Shocks:** shocks are rolled as for electrified doors.

## Tubes, bulbs and boxes

- **Tubes and bulbs:** `light-tube` and `light-bulb` are small items (force 2). Their
  sprite state shows whether they are burnt out (`burned`) or broken (`broken`).
- **Boxes:** a box of replacement tubes (`light-tubes-box`) holds 7 tubes. A box of
  replacement bulbs (`light-bulbs-box`) holds 7 bulbs.

## Differences from Luna

- **Light on the tiles:** a lit fixture lights the tiles around it in its colour, worked
  out on the client (`lighting.md`).
- **Instant changes:** lights switch together on the next tick. Luna waits up to 1.5 s at
  random after a power change.
- **Switch count:** a fresh tube starts counting switches from zero. In Luna the count
  travels with the tube.
- **Coloured lights:** Luna's blue and red tubes and red bulbs keep their colours
  (`light-blue`, `light-red`, `light-small-red`); floor lights have no prototype yet.
- **Worn lights and emergency glow:** some lit lights stutter, and working lights glow faintly
  red where the lighting has no power. Both are drawn only on the client (`lighting.md`).
- **Not implemented:**
  - building and taking apart fixtures (the screwdriver, frames, wiring);
  - smashing a held tube;
  - red alert flicker;
  - ghosts flickering lights;
  - rigged tubes exploding;
  - heat and explosions breaking lights;
  - light replacers;
  - sparks (breaking glass is heard, `sound.md`);
  - switches controlling another area.
