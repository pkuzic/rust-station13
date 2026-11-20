# The emag

Sources: `luna:code/defines/obj/weapon.dm` (`card/emag`), `luna:code/game/machinery/doors/door.dm` and
`windowdoor.dm` (`attackby`, `open`), `luna:.../closets/secure/secure_closets.dm` and `crates.dm`
(`attackby`), `luna:code/game/machinery/vending.dm` (`attackby`), `luna:code/modules/power/apc.dm`
(`attackby`, `update_icon`), `luna:code/game/gamemodes/game_mode.dm` (the uplink list).

Code: `game/classic/src/emag.rs`. Tests: `game/classic/tests/emag.rs`.

The cryptographic sequencer is a small card. A traitor buys it from the uplink for 3 crystals
(`traitor.md`). Swiped on something within reach, it does the following, and nothing to anything else.

| Target | What happens |
|---|---|
| A shut airlock | Its controls burn out for good. It opens, if it could open anyway: not bolted, not welded, and with power. From then on it opens or shuts for nothing: no hand, card, crowbar or timer. |
| A shut window door | The same: it swings open and stays open. |
| A shut secure locker or crate | Its lock breaks open. A locker shows everyone around "The locker has been broken by John with an electromagnetic card!" A crate tells you "You unlock the crate." From then on a card does nothing to the lock: "The locker appears to be broken." |
| A vending machine | "You short out the product lock on the machine." It sells to anyone, as with its ID wire cut, and mending the wire doesn't fix it. |
| An APC | With its cover and its panel shut, half the time the interface breaks unlocked ("You emag the APC interface."), and the APC shows `apcemag`. The other half: "You fail to unlock the APC interface." An emagged APC's screwdriver and ID card say "The interface is broken". |

## Differences from Luna

| Luna | Here |
|---|---|
| An emagged door flicks its `door_spark` animation for 0.6 s before it opens | It opens at once, with the spark sound |
| The emag also works on bots, turrets, the arcade, borgs, the RD console, the mech fabricator and more | Only what's listed above, for now |
