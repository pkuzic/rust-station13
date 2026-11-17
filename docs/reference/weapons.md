# Guns and stun batons

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

## Captain's antique laser (`captain-laser`)

It fires a heavy beam: 40 burn, twice a laser's, through glass as a beam does. It can't be
switched to stun. It charges itself, 100 every 4 seconds, wherever it is. Force 10. It's one of
the traitors' theft objectives (`traitor.md`).

It starts in the **display case** (`display-case`) in the captain's quarters, held in a hover
field behind glass with 30 health:
- an item clicked on it hits the glass for its force;
- an empty hand kicks it for 2 ("X kicks the display case.");
- a shot that stops at it does its damage (a beam 20, a heavy beam 40, a bullet its brute, an
  electrode nothing);
- an explosion takes the case at the heaviest, dropping the gun, and otherwise cracks the glass
  half the time (15, or 5 at the lightest).

The glass rings when hit. At 0 it shatters and can be walked through, and an empty hand then
deactivates the field and takes the gun ("You deactivate the hover field built into the case.").

## Bullet guns

They come loaded and fire the same way, with rounds instead of charge. Click a gun with ammunition to
load as many rounds as fit, whether the gun is on the floor in reach or in your other hand. A gun takes
one kind of round at a time. Rechargers don't take them.

| Gun | Caliber | Holds | Starts with |
|---|---|---|---|
| **Stechtkin pistol** (`pistol`) | 9mm | 8 | 9mm |
| **Revolver** (`revolver`) | .38 | 6 | .38 |
| **Combat shotgun** (`combat-shotgun`) | .12 | 8 | slugs |
| **Double-barreled shotgun** (`double-barrel`) | .12 | 2 | beanbags |

| Ammunition | Caliber | Rounds | Brute | Knocks down for |
|---|---|---|---|---|
| `magazine-9mm` | 9mm | 8 | 30 | |
| `magazine-12mm` | 12mm | 20 | 25 | 6 s |
| `speed-loader-38` | .38 | 6 | 10 | 20 s |
| `slug-box` | .12 | 8 | 60 | |
| `beanbag-box` | .12 | 8 | 10 | 20 s |

No gun on the decks takes 12mm yet.

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
| A gun holds a row of casings, ejects them, and a pistol takes a whole magazine | A count of one kind of round, moved across one click at a time, and the magazine stays in your hand |
| Other bullet guns, syringe guns and laser tag | Not built |
| Clumsy shooters | Not built |
| A shattered display case leaves a glass shard | There are no shards yet |
| The display case breaks to a blob and to a meteor | Not yet |
