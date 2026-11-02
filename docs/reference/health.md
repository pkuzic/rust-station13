# Reference spec: health, attacks, unconsciousness and death

Sources:
- `luna:code/modules/mob/mob.dm`: `health_full = 100`, `updatehealth`
- `luna:code/WorkInProgress/NewMobs and etc/organs.dm`: external organs, `max_damage`, `take_damage`
- `luna:code/WorkInProgress/NewMobs and etc/newitem.dm`: `/obj/item/proc/attack`
- `luna:code/WorkInProgress/NewMobs and etc/Damagesystem.dm`: `human/attack_hand` (punching)
- `luna:code/WorkInProgress/NewMobs and etc/human_life.dm`: stat thresholds
- `luna:code/modules/mob/mob_helpers.dm`: `ran_zone`
- Item forces: `luna:code/defines/obj/weapon.dm`, `game/objects/items/weapons/tools.dm`,
  `game/objects/tank.dm`, `game/objects/items/stacks/sheets/sheets.dm`

## Luna behaviour

**Health.** `health = health_full (100) − (oxy + tox + burn + brute + halloss + organ brute)`.
Brute and burn are summed over the external organs.

**Organs and their `max_damage`.** Damage to one organ stops at its limit (brute + burn):

| Organ | Max | Organ | Max |
|---|---|---|---|
| chest | 150 | groin | 115 |
| head | 125 | l_arm, r_arm, l_leg, r_leg | 85 |
| l_foot, r_foot | 51 | l_hand, r_hand | 45 |

**States.** Below 0 health a human is unconscious (`stat = 1`, paralysed). Below −100
(or with no brain) they die. In crit, the inability to breathe makes them lose more health
over time; this needs atmos.

**`ran_zone(zone, 90)`.** For the chest: 90% chest, otherwise head, left arm or right arm
with equal odds. For any other zone: the zone with 67.5% probability, else the chest.

**Hitting with an item.** This happens on **any intent**: in this era, clicking a person
with something in hand hits them. Damage is the item's `force` as brute damage to
`ran_zone(selected zone)`. Viewers see "X has been attacked in the {zone} with the {item}
by Y!". Head and chest hits can knock out or down (chance `force + organ damage`).
Helmets and armour can stop hits.

| Item | force |
|---|---|
| crowbar | 5 |
| wrench | 5 |
| screwdriver | 5 |
| wirecutters | 6 |
| oxygen tank | 5 |
| metal sheet | 5 |
| ID card | 0 (inherits `/obj/item`) |

**Punching (empty hand, harm intent).** Damage `rand(1, 9)` to `ran_zone(selected zone)`.
It lands with 90% probability: "X has punched Y!", or "...in face!" for the head. Otherwise
"X has attempted to punch Y!". Damage over 7 can weaken; clothing can block.

## Implemented (`game/classic/src/health.rs`, `combat.rs`)

| Behaviour | Status |
|---|---|
| Health 100 minus total damage; per-organ limits as above | keep |
| Unconscious below 0, dead below −100; dead stays dead | keep |
| Unconscious or dead mobs cannot move, act or speak | keep |
| `ran_zone` with the probabilities above | keep |
| Target zone is always the chest | **simplification**: no zone selector UI yet |
| Item hit on any intent, force as brute damage, viewer message | keep |
| Punch on harm intent: 1–9, 90% to land, face message for the head, miss message | keep |
| Help intent with an empty hand on a person: nothing | **not yet**: shaking and hugging |
| Messages go to conscious mobs who can see the target (same sight rule as speech) | keep |
| Randomness from a seeded generator, so tests and replays are repeatable | keep |
| The owning client receives its health and state | keep (HUD) |

## Not implemented yet

Knockouts and knockdowns, stuns and weakening, armour and helmets, breathing and
suffocation in crit, healing, burns, toxins, wounds, bleeding, broken and lost limbs, pain,
disarm and grab intents, the zone selector, examine showing injuries.

After death the player walks on as a ghost (`ghosts.md`).
