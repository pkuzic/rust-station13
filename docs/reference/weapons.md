# Energy guns and stun batons

Sources: `luna:code/modules/projectiles/gun.dm`, `guns/energy.dm`, `guns/energy/laser.dm`, `stun.dm`,
`nuclear.dm`, `projectile.dm`, `projectile/beams.dm`, `projectile/energy.dm`,
`luna:code/game/objects/items/weapons/swords_axes_etc.dm` (`melee/baton`),
`luna:code/game/machinery/recharger.dm`; `NSV_Luna.dmm`: 5 laser guns, 6 energy guns, 4 tasers, 2
batons, 12 rechargers on the decks.

Code: `game/classic/src/weapons.rs`. Tests: `game/classic/tests/weapons.rs`.

## Guns

| | Fires | Shots |
|---|---|---|
| **Laser gun** (`laser-gun`) | beam | 10 |
| **Energy gun** (`energy-gun`) | electrode or beam: click it in hand to switch, starting on stun | 10 |
| **Taser** (`taser`) | electrode | 5 |

Click anything or any floor **out of reach** with a gun in the active hand to fire at it. It rests
0.4 s between shots. With too little charge it only clicks. In reach, a gun is just something to hit
with (force 5), as in Luna. Their sprites show the charge by quarters.

A shot flies at 10 tiles a second, along the line through what was aimed at, for up to 50 tiles. It
goes over tables and racks, past anyone lying down or out cold unless aimed at them, and never hits
its shooter. It stops at the first person, wall or dense thing. Glass panes and window doors stop an
electrode, and beams go through them.

- **Beam:** 20 burn on a random part, mostly the chest.
- **Electrode:** no damage, and knocked down for 20 s (`flashes.md`).

## Stun baton (`stun-baton`)

Force 10. Click it in hand to turn it on or off; with 1000 units or less it stays off. Its cell holds
10,000 units and a hit takes 1000.

On someone in reach:

- **On:** knocked down for 20 s, whatever your intent.
- **Off:** they are only prodded, and onlookers are told.
- **On harm intent:** it's also an ordinary blow.

## Recharger (`recharger`)

Click it with a gun or baton to put it in, if the room has power. Click with an empty hand to take it
out. Every 2 seconds it adds 100 to a gun or 250 to a baton, and shows when it's full.

## Differences from Luna

| Luna | Here |
|---|---|
| Aimed by the zone selector, with accuracy falling off with distance | Aimed at the chest, as for blows |
| Armour blocks shots | Armour doesn't work yet |
| Stutter and blurred eyes | Not built |
| A baton's cell can be swapped, and an EMP drains them | Not built |
| Wrenching a recharger loose | Not built |
| Bullet guns, shotguns, syringe guns and laser tag | Not built |
| Clumsy shooters | Not built |
