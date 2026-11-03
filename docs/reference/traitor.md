# Traitors

Sources: `luna:code/game/gamemodes/traitor/traitor.dm`, `game_mode.dm` (`equip_traitor`, `declare_completion`,
`uplink_items`), `newobjective.dm`, `luna:code/WorkInProgress/Abi79/uplinks.dm`. Code: `game/classic/src/traitor.rs`,
`apps/client/src/uplink_panel.rs`. Tests: `game/classic/tests/traitor.rs`.

## Who

When a round in traitor mode starts, one crew member in about ten among those there becomes a traitor: (players + a
roll from 0 to 5) / 10, rounded, from 1 to 4. They're told so, twice, and given their objectives. Ten minutes later
they're reminded. Late joiners are never traitors.

A round with nobody ready at its start runs as extended, as a traitor round would have nobody to pick. In Luna it stays
in the lobby and picks again.

## Objectives

A traitor gets objectives until their weight reaches 100, each picked by chance: theft (45%, weight 20), murder (40%,
weight 60), protection (15%, weight 30), never both murder and protection of one person. With nobody else aboard it's
all theft. Then escaping (90%), or getting away alone (10%).

| Objective | Met at the end of the round if |
|---|---|
| Steal | it's in their hands, worn, or in a container on them |
| Assassinate | the target is dead |
| Protect | the target is alive in a pod |
| Escape | they're alive and uncuffed in a pod |
| Hijack | they're alive in a pod, with nobody else living in any pod |

At the end of the round, everyone is told who the traitors were, how each objective went, and whether they succeeded.

## The uplink

A traitor gets a ship bounced radio, in their backpack, or else in a free hand, or else at their feet. Used in the hand
it opens the uplink: 10 tele-crystals, and a list of what they can afford, bought into a free hand or onto the floor.
Using it also repeats a traitor's objectives in chat.

| Sold | Crystals |
|---|---|
| .357 revolver (7 rounds, 60 damage) | 7 |
| Box of .357 | 3 |
| Fully loaded toolbox | 1 |
| Soap | 1 |

## Differences from Luna

| Luna | Here |
|---|---|
| The uplink is hidden in their own PDA with a passcode, or their radio with a frequency to dial | A separate radio, as ours can't be dialled yet and PDAs don't exist |
| Notes keep the objectives to read later | Using the uplink repeats them |
| Thefts include the captain's laser, the RCD, a toy singularity, a cyborg shell and an AI core, and six reagents | Only the eight items on the station. The rest wait for what makes them (`LUNA_PARITY.md` 3.6, 3.9, 3.10, 1.14) |
| Stealing a full plasma tank wants 28 moles in it | Any plasma tank |
| An intercepted report is printed at communications consoles after a few minutes | Not built |
| Silicons can be traitors | There are no silicons yet |
| Syndicate soap, as ordinary soap with another sprite | Ordinary soap |

Waiting for what makes them work, and not sold yet: implants (3.20), energy crossbow and sword (4.3), hacktool, emag (4.1),
no-slip shoes, chameleon jumpsuit, thermals, voice changer (1.12), binary key (1.10), fake ID, EMP grenades, explosives
(1.13), Detomatix cartridge (3.21), chameleon projector, sleepy pen, freeform AI module (3.9), power sink, cyanide pill.
