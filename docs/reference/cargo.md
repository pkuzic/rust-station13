# Cargo

Sources: `luna:code/game/supplyshuttle.dm`, `luna:code/datums/supplypacks.dm`,
`luna:code/game/objects/structures/crates_lockers/crates.dm` (secure crates).

Code: `game/classic/src/cargo.rs`. Packs: `game/classic/content/supply_packs.ron`. The shuttle:
`game/classic/maps/luna_supply_shuttle.ron`. Window: `apps/client/src/cargo_panel.rs`.
Tests: `game/classic/tests/cargo.rs`.

## Supply points

- **Start:** 50 at round start.
- **Over time:** one more every 30 s.
- **Sending crates back:** every crate on the shuttle when it is sent back to the dock earns 4,
  and every plasma sheet inside one earns 1 more. The crate and everything in it are gone.

## The consoles

Both need power and show the shuttle's whereabouts and the points.

- **Supply ordering console** (`order-console`; Luna `ordercomp`): anyone can use it.
  - Request items: pick a category, then a pack. The request goes on the list with the
    requester's name: "Thanks for your request. The cargo team will process it as soon as
    possible."
  - View approved orders, and view requests.
- **Supply shuttle console** (`supply-console`; Luna `supplycomp`): needs cargo access
  ("Access Denied." otherwise).
  - Order items: only while the shuttle is at the dock. A pack is paid for at once, from the
    points ("Not enough supply points." if they don't cover it), and joins the orders.
  - View requests: approve one (paid for as an order) or remove it, while the shuttle is at
    the dock; clear the list.
  - Send the shuttle to the station, or back to the dock.
- **Categories,** in Luna's order: Emergency, Security, Engineering, Medical, Science,
  Materials, Miscellaneous. A pack may be under several.

## The shuttle

- **At round start** it waits at Centcom's dock. The supply hangar's berth on deck 4 (Luna's
  `/area/supply/station`, 13 × 7 tiles) is open space.
- **Sent to the station:** "The supply shuttle has been called and will arrive in 2 minutes."
  The hangar's blast doors (`qmshuttle`) shut. Two minutes later the shuttle is laid into the
  berth as Luna's map has it at the dock: its floor and walls, window panes, doors, conveyor
  belts (`qmshuttlebelts`), with air. Things left aboard last time come back where they were.
  Every order is put down on a free crate marker, the last free one first, or a random one
  if all are taken:
  - in its crate, if the pack comes in one, with everything in it and its manifest;
  - loose on the floor otherwise (canisters, tanks, the barrier, sheets), with the manifest
    beside it.
- **Sent back to the dock:** at once, and never with anyone aboard, alive or dead ("The supply
  shuttle can not transport ship employees."). The crates aboard are sold, the orders list is
  cleared, the hangar's blast doors shut, and the berth is open space again. Anything else
  left aboard goes with it and comes back next time.

## Crates and manifests

- **Secure crates** (`crate-secure`, `-weapon`, `-plasma`, `-gear`): locked, with a red light,
  to the access their pack names. A card with that access unlocks them, as a secure locker's
  lock (`closets.md`); unlocked, the light is green.
- **Other crates** for packs: `crate`, `crate-internals` (which also brings what an internals
  crate always holds), `crate-medical`, `crate-electrical`, `crate-engineering`.
- **The manifest** (`paper`, named "Supply manifest"): "Centcom Shipping Manifest", the
  destination, and each thing in the pack by name, in the reader's language
  (`[noun:ID]` on a printed sheet).

## Packs

31 of Luna's 46 packs, with their costs and contents as Luna has them
(`supply_packs.ron`). Some come without parts that aren't built yet:

| Pack | Without |
|---|---|
| Medical crate | the sleep toxin bottle |
| Riot gear crate | flashbangs and tear gas (grenades, 3.20) |
| Standard costume crate | the bottle of nothing |
| Janitorial supplies | the cleaner grenades |
| Bureaucracy crate | the camera |

## Differences from Luna

- **No comment on orders:** Luna asks for a comment when an order is placed or approved.
  There is no text box for it yet.
- **High-volume tanks:** the high-volume water and fuel tanks come as ordinary ones.
- **Crate names:** Luna names a pack's crate after the pack ("Internals crate"). Custom
  names on objects aren't built yet (4.7), so crates keep their kind's name.
- **Stamped manifests:** Luna gives 4 points for a stamped manifest sent back. Stamps aren't
  built yet (4.8).
- **Not implemented:**
  - packs whose contents are mostly not built: Emergency crate (bots, metal foam), Virus
    crate, Solar pack crate, Death Alarm, tracking and chemical implants, Party equipment,
    MULEbot, Collectable hats, Toner cartridges;
  - "Experimental weapons" (stun gloves) and the two mech circuit crates;
  - the emag's special supplies and the hacked console's contraband (their contents aren't
    built);
  - the status displays' `supply` signal.
