# Hacking machines

Sources: `luna:code/game/machinery/vending.dm` and `vending_tg.dm` (`attackby`, `cut`, `mend`, `pulse`,
`throw_item`, `shock`), `luna:code/datums/wires/vending.dm`, `luna:code/game/machinery/autolathe.dm`
(`wires_win`, `interact`, `Topic`), `luna:code/modules/power/apc.dm` (`attackby`, `interact`, `cut`, `mend`,
`pulse`).

Code: `game/classic/src/hacking.rs`. The window is the airlock's wire panel (`apps/client/src/wire_panel.rs`,
`wires.md`). Tests: `game/classic/tests/hacking.rs`.

## Opening a panel

- **Screwdriver:** opens or shuts the panel.
  - On a vending machine: "You open the maintenance panel."
  - On the autolathe: its maintenance hatch, and it shows the `autolathe_t` sprite while open.
  - On an APC: exposes its wires, but not while its cover is open ("Close the APC first").
- **Wirecutters or a multitool on an open panel:** shows the wire panel. Each wire has Cut or Mend, and
  Pulse. Cutting needs wirecutters, pulsing a multitool, and a cut wire can't be pulsed, as on airlocks.
  - With the panel shut, the tool does what an empty hand would.
  - On the autolathe, anything but a crowbar or screwdriver shows the wires while the hatch is open, an
    empty hand included.
- **The window closes** when you step out of reach, the panel is shut, or you're knocked out.

Which colour does what is shuffled:

- for vending machines and APCs, once per round, the same for every machine;
- for each autolathe, on its own.

## Vending machines

Wires: violet, orange, goldenrod and green.

| Wire | Pulse | Cut | Mend |
|---|---|---|---|
| Contraband | Shows or hides the contraband | Hides it | – |
| ID scanner | Turns the scanner off or on | Off: anyone can buy from a machine that wants access | On |
| Shock | Electrified for 30 s | Electrified until mended | Not electrified |
| Throw | Starts or stops throwing | Starts throwing | Stops |

**Contraband** is added to the end of the machine's list while it shows:

| Machine | Contraband |
|---|---|
| Robust Softdrinks (both sprites) | 5 Blastwave Beers |
| Cigarette machine | 4 Zippo lighters |

**Electrified:** using one by hand while it has power shocks you, every time, as an electrified door does
(`wires.md`, Shocks). The window still opens.

**Throwing:** every 2 s, a machine with power has a 2% chance of throwing something.
- It picks someone alive within 5 tiles.
- The first product in stock comes off the count and flies at them, each way 15% of the time a tile off.
- Everyone around sees "The Getmore Chocolate Corp launches a candy bar at John!"

**Lights:**
- orange: on while electrified;
- red: blinking, off while throwing;
- green: on while contraband shows;
- purple while the scanner works, yellow while it doesn't.

## The autolathe

Ten wires: light red, dark red, blue, green, yellow, black, white, grey, orange and pink. Three of them
do something; the other seven do nothing.

| Wire | Pulse | Cut or mend |
|---|---|---|
| Hack | Shows or hides the hidden list for 10 s | Shows or hides it |
| Disable | Kills or revives the buttons for 10 s; 50% shock | Kills or revives them; 50% shock |
| Shock | Live or safe for 10 s; 50% shock | Live or safe; 50% shock |

- **Hidden list:** handcuffs (500 metal) and a box of .357 (50 000 metal), at the end of its list.
- **Dead buttons:** using it by hand gives "You press the button, but nothing happens." Sheets still go in.
- **Live shock wire:** a 50% shock every time it's used by hand, or its wires opened.
- **Lights:** red off while disabled, green off while live, blue off while hacked.

## APCs

Wires: orange, dark red, white and yellow.

| Wire | Pulse | Cut | Mend |
|---|---|---|---|
| ID scanner | Unlocks the interface for 30 s, then it locks | – | – |
| Main power ×2 | Shorted for 120 s | Shorted; 50% shock | With both whole: not shorted; 50% shock |
| AI control | Light off for 1 s | Light off | Light on |

- **A shorted APC** powers nothing in its area, as with its breaker off. It still charges its cell.
- **ID cards:** with its wires exposed, swiping one says "You must close the panel."
- **Lights:** locked or unlocked, shorted or working, and "AI control allowed".

## Differences from Luna

| Luna | Here |
|---|---|
| Vending machines show their wires inside their own window | A separate wire panel, opened with the tools |
| `vending_tg` machines only spark when they shock | Every machine shocks, as the older `vending` ones do |
| A machine shocking you keeps its window shut | The window opens anyway |
| Thrown products hit for their throwing force | They land where they're thrown |
| A pulsed main wire unshorts an APC after 120 s even with the other wire cut | It stays shorted while either is cut |
| APC shocks scale with its cell | The airlock shock: `min(rand(20, 80), rand(20, 80))` |
| The autolathe's hidden list also has a flamethrower, an RCD, an electropack, an industrial welder, magazines, an infrared emitter and shotgun shells | Not in the game yet |
| More contraband: ice, Syndi-Cakes, Thirteen Loko, and pills and bottles on the medical machines | Not in the game yet |
| A remote signaller can be attached to a wire | Not built |
| The emag | Not built yet |
| The APC sprite shows its exposed wires, and a vending machine its open panel | Their sprites don't change |
