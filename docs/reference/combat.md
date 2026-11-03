# Combat

Sources: `luna:code/WorkInProgress/NewMobs and etc/Damagesystem.dm` (`attack_hand`), `newitem.dm` (`attack`),
`luna:code/modules/mob/mob.dm` (`ran_zone`, `zone_sel`). Code: `game/classic/src/combat.rs`, `apps/client/src/hud.rs`. Tests:
`game/classic/tests/combat_blows.rs`.

## Aiming

The doll at the top of the right bar chooses a body part. A blow aimed at the chest lands there 90% of the time, and one
aimed anywhere else about two times in three, or else on the chest. Medical packs and guns don't follow it yet.

## Intents

The intent button goes round them, or 1 help, 2 disarm, 3 grab, 4 harm. Anything in your hand hits on any intent. These
are for an empty hand.

- **Help:** on someone else, shakes them, taking 6 seconds off being down or out. On yourself, tells you how each part of
  your body is.
- **Disarm:** on someone standing. 25%: pushed down for 4 seconds. 35%: drops what's in their active hand. Otherwise it
  misses.
- **Grab:** takes hold of them with that hand. See below.
- **Harm:** a punch for 1 to 9, which misses 10% of the time.

## What blows do

| Where | With a weapon | With a fist |
|---|---|---|
| Head | A helmet stops it `80 - force`% of the time, and 1 in 5 of those still hurt. Otherwise `force + damage already there`% it knocks them out for up to 24 seconds | Anything over the head stops 99%, and 1 in 5 of those still hurt. Over 7 damage: down for 16 seconds |
| Chest | Armour stops it `90 - force`% of the time. Otherwise, the same chance as for the head, down for 10 seconds or stunned for 4 | A suit or uniform stops 85%. Over 7 damage: down for 10 seconds |
| Groin | Armour stops it `90 - force`% of the time | A suit or uniform stops 75%. Over 7 damage: down for 6 seconds |
| Elsewhere | Just the damage | Just the damage |

Someone protected is told so. A lit welding tool burns, and nothing stops it.

## Grabs

Sources: `luna:code/modules/mob/mob.dm` (`obj/item/weapon/grab`, `client/Move`), `tables_racks.dm`. Code:
`game/classic/src/grabs.rs`. Tests: `game/classic/tests/grabs.rs`.

The grab sits in your hand like an item, and dropping it lets go. Click the person, or the grab itself, to tighten it.

| Hold | How | While it lasts |
|---|---|---|
| Passive | Grab intent, on someone, with an empty hand. Not if they're buckled in | They come with you. If they walk, it ends |
| Both hands | 75%. Otherwise you lose hold. Not with your other hand holding someone else | They come with you, and drop what they hold. Each attempt to walk, once a second, has a 25% chance of breaking free |
| Neck | Not with a helmet or gas mask in the way | They're on your tile, and drop what they hold. 5% to break free |
| Choking | Click again to start or stop | Each second they miss breaths and 45% of the time go down for 6 seconds |

A grab ends if you go down or lose them. Clicking a table with a grab puts them on it, down for 10 seconds.

## Differences from Luna

| Luna | Here |
|---|---|
| Armour by each clothing's `body_parts_covered` | By kind: helmets, armour, and any suit or uniform for the body |
| Stunned: can't act, still standing | Stunned is the same as knocked down |
| Help on someone in crit starts CPR | Not built |
| Blood on the weapon, the attacker and the floor | Not built (3.3, 3.19) |
| Broken and robotic arms change a punch, hulks and stun gloves | Not built (3.3, 3.5) |
| Throwing someone from a grab, into closets, onto spikes and into gibbers | Not built (4.2, 3.7) |
| A grab on someone at both hands stops anyone else tightening theirs | It doesn't |
| Blows at the groin can't knock anyone down unless they're a zombie, which looks like a bug | Kept: no knockdown |
