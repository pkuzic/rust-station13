# Airlock maintenance panels and wires

Sources: `luna:code/game/machinery/doors/airlock.dm` (`pulse`, `cut`, `mend`, `attack_hand`,
`attackby`, `loseMainPower`, `Bumped`) and `luna:code/modules/power/Electrocution.dm`.

Code: `game/classic/src/wires.rs`, `game/classic/src/doors.rs`.
Client: `apps/client/src/wire_panel.rs`.
Tests: `game/classic/tests/wires.rs`.

## Panel

- **Opening:** a screwdriver on an airlock opens or closes its maintenance panel. A closed door
  shows Luna's `panel_open` overlay, and the opening and closing animations use the
  `o_door_*` states.
- **Using:** with the panel open, using the door by hand or with any item except a
  screwdriver or crowbar shows the wire panel instead of opening the door. Walking into the
  door does nothing.
- **Closing:** the panel window closes when you step out of reach, the panel is screwed
  shut, or you are knocked out.

## Wires

There are eleven colours: orange, dark red, white, yellow, red, blue, green, grey, black,
pink and brown. Which colour does what is shuffled once per round from the game's seed,
and is the same for every airlock, as in Luna.

| Wire | Pulse (multitool) | Cut (wirecutters) | Mend |
|---|---|---|---|
| ID scanner | Door flashes "deny" if powered | The door ignores every ID card: doors that need access stay shut | Scanner works again |
| Main power ×2 | Main power off for 60 s; the door is dead for 10 s until backup takes over | Main power off; the 60 s timer stops while either main wire is cut | With both whole: main power back at once |
| Backup power ×2 | Backup off for 60 s | Backup off; the timer stops while either backup wire is cut | With both whole: backup back at once |
| Bolts | Drops the bolts; raises them only if the door has power | Drops the bolts; mending does not raise them | – |
| Open door | Opens or closes the door if it needs no access or the ID wire is cut | – | – |
| Electrify | Electrified for 30 s | Electrified until mended | Not electrified |
| AI control | Nothing yet (no AI) | Turns off the "AI control" light | Light back on |
| Crush | Toggles the hydraulics light (crushing is not modelled) | – | – |
| Bolt light | Toggles whether a bolted door shows the bolted sprite | – | – |

**Power:**
- A door has power when its area has power and either its main or backup power is up.
- Without power it can't be opened or closed normally and its bolts can't be raised, but a
  crowbar can prise it open unless it is bolted.

**Tools:**
- **Wirecutters:** cutting needs them ("You need wirecutters!").
- **Multitool:** pulsing needs one ("You need a multitool!"). A cut wire can't be pulsed.

**Lights on the panel:**
- bolts down or up;
- test light (whether the door has power);
- "AI control allowed";
- safety light (flashing while electrified);
- hydraulics.

## Shocks

| Trigger | Chance |
|---|---|
| Cutting a power wire, or mending a pair back together | 50% |
| Touching an electrified door by hand, or walking into it | 100% |
| Using a tool on an electrified door | 75% |

A shock does `min(rand(20, 80), rand(20, 80))` burn damage, spread over the body, times the
gloves' conductivity: 1 bare-handed, 0 with insulated gloves. You see "You feel a powerful
shock…", and everyone else who can see you sees "X was shocked!".

## Differences from Luna

- **Second wire of each pair:** Luna's `switch` writes `if (A || B)`, which in DM only matches
  `A`. So in Luna the second main and backup power wires do nothing when pulsed or cut. Here
  both wires of a pair work, as Luna's own comments describe.
- **Dead doors don't shock:** a door whose area has no power cannot shock anyone. Luna's
  `Electrocute` doesn't check power.
- **No stun:** Luna also stuns and knocks down the victim. There are no stuns yet.
- **One shock per bump:** Luna can shock twice when you walk into an electrified door
  (`attack_hand`, then `Bumped`). Here it is once.
- **Not implemented:** signallers on wires, AI hacking, and crushing.
