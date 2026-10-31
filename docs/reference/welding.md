# Welding tools and fuel tanks

Sources: `luna:code/game/objects/items/weapons/tools.dm` (`/obj/item/weapon/weldingtool`:
`attack_self`, `afterattack`, `examine`), `luna:code/game/machinery/doors/airlock.dm`
(`attackby`, `open`, `update_icon`) and `luna:code/modules/Chemistry/reagent_dispenser.dm`
(`fueltank`).

Code: `game/classic/src/welding.rs`, welded doors in `doors.rs`.
Tests: `game/classic/tests/welding.rs`.

## The welding tool

- **Item:** size 2. Unlit it hits for 3 brute. A new one holds 30 units of fuel, and that is
  all it holds.
- **Examined:** "It contains N units of fuel left!"
- **Lighting it:** use it in the active hand (Z, or click the active hand).
  - Unlit, with fuel: it lights. "You will now weld when you attack."
  - Unlit and empty: "Need more fuel!", and it stays out.
  - Lit: it goes out. "Not welding anymore."
- **Lit:** it shows its flame and hits for 15 burn instead.
- **Fuel:** every click with a lit tool on something in reach uses one unit: a tile, an
  item, a machine or a person. When the last unit goes, it goes out: "Need more fuel!"
- In the toolboxes and belts: the mechanical toolbox and the loaded utility belts carry
  one, and utility belts take them (`storage.md`).

## Airlocks

- A lit welding tool on a **closed** door (not while it moves) welds it shut, or open again
  if it is welded.
  - It needs more than 2 units and uses 2, plus the 1 every click uses.
  - Short of fuel: "Need more welding fuel!", and nothing changes.
- A welded door opens for nothing: not by hand or card, not by a crowbar on an unpowered
  door, not by the wires. It shows the weld over the door.
- An unlit welding tool is just an item to the door: it uses it like a hand.

## Fuel tanks

- A fuel tank is a dense structure with 1000 units of welding fuel.
- **Examined:** "It contains N units of welding fuel."
- An unlit welding tool used on it is filled up to 30 from it: "Welder refueled."
- A lit one takes nothing.

## Differences from Luna

- **Lit welding tool on a fuel tank:** Luna blows the tank up. We have no explosions yet,
  so nothing happens (the click still uses a unit of fuel).
- **Not implemented:**
  - eye damage and welding protection;
  - lit welding tools setting things on fire (Luna's hot spots);
  - the screwdriver and rods flamethrower assembly;
  - the mini, industrial and high-volume variants (imported as the ordinary ones);
  - sounds other than welding itself (`sound.md`).
