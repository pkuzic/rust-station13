# Handcuffs and turrets

Sources: `luna:code/game/objects/items.dm` (`handcuffs/attack`), `luna:code/modules/mob/mob.dm`
(breaking out), `luna:code/game/machinery/turrets.dm` (`turret/process`, `check_target`, `shootAt`,
`turretid`); `NSV_Luna.dmm`: 3 handcuffs and a box of them, 10 turrets and 4 controls around the AI.

Code: `game/classic/src/security.rs`. Tests: `game/classic/tests/security.rs`.

## Handcuffs (`handcuffs`)

Click someone in reach with them in the active hand. It takes 4 seconds, and onlookers are told; if
either of you moves, it stops. Once cuffed, a person can walk but can't use their hands at all. The
cuffs show on them.

- **Taking them off someone:** click them with an empty hand. It takes 4 seconds, in the same way.
- **Breaking out:** press B. It takes 100 seconds of standing still, and everyone who can see is told.

The cuffs end up on the floor either way, ready for use again.

## Turrets (`turret`) and their controls (`turret-control`)

A turret lies hidden in the floor of one of Luna's turret-protected areas. While on and powered, it
rises for anyone in its own area who is up, or anyone awake when it's set to kill, and shoots at one of
them every 3 seconds: electrodes on stun, beams on kill (`weapons.md`). With nobody to shoot at, it
sinks back. They start on stun.

Clicking a control with an AI upload card, in hand or worn, steps its area's turrets through stun, kill
and off.

## Differences from Luna

| Luna | Here |
|---|---|
| A strip window for cuffs and clothes | Clicks |
| Cuffed people are also slowed when pulled | Not built |
| Cable restraints | Not built |
| The control's window, its lock, emags and the AI | One click on the panel, with access |
| Turrets can be shot and destroyed | Not built |
| A turret's cover animates as it rises | It just appears |

## Deployable barriers (`barrier`)

Source: `luna:code/game/objects/structures/barricade.dm`. They block the way. Clicking one with any ID card in
hand locks or unlocks it, which shows on its light. Luna's access check is commented out, so any card works
there too. An unlocked barrier can be pulled; locking one makes whoever is pulling it let go.

A barrier has 100 health. Hitting one takes 3/4 of what you're holding's force, and at none it falls apart. A lit
welder repairs it fully, and uses up the welder's usual fuel for being used on something.

Differences from Luna: shots, explosions and blobs don't damage it, it doesn't drop metal rods when it breaks, and
burn weapons don't do less than brute. An emag doesn't short it out.
