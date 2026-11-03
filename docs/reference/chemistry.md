# Chemistry

Sources: `luna:code/modules/Chemistry/Chemistry-Holder.dm` (`trans_to`, `metabolize`),
`Chemistry-Reagents.dm` (`on_mob_life`), `reagent_containers/syringes.dm`, `pill.dm`, `hypospray.dm`,
`glass.dm`, `glass/bottle.dm`.

Code: `game/classic/src/chemistry.rs`. Tests: `game/classic/tests/chemistry.rs`.

Beakers, bottles, buckets, pills, syringes, hyposprays and people's blood all hold reagents. Examining
a container says how many units are in it. Moving reagents takes an even share of everything in it.

## Using them

- **Syringe (15 units):** it shows whether it is drawing or injecting and how full it is. Click it in hand
  to switch. Drawing takes 5 units from a container. Injecting gives 5: at once to yourself or a container,
  and for someone else after 3 seconds if you both stay put, with onlookers told. It switches to injecting
  when full and back when empty.
- **Pill:** click yourself or someone in reach. It's swallowed whole.
- **Hypospray (30 units of tricordrazine):** 5 units, at once, into anyone in reach.
- **Beaker, large beaker, bottle, bucket:** click another container in reach to pour 10 units.

## Chem dispenser (`chem-dispenser`)

Click it with a beaker to load it, or with an empty hand to open its window. The window gives 21 pure
chemicals into the beaker, 5, 10 or 30 units at a time. Each 10 units takes 1 of its 100 energy, which
comes back 1 per half a minute while it has power. Without power it gives nothing.

## Mixing

Whenever reagents arrive in a container or someone's blood, whatever can mix does:

| Makes | From |
|---|---|
| 3 `inaprovaline` | oxygen, carbon, sugar |
| 3 `anti_toxin` | silicon, potassium, nitrogen |
| 2 `kelotane` | silicon, carbon |
| 2 `bicaridine` | inaprovaline, carbon |
| 2 `tricordrazine` | inaprovaline, anti_toxin |
| 3 `dermaline` | oxygen, phosphorus, kelotane |
| 1 `dexalin` | 2 oxygen, 0.1 plasma |
| 3 `dexalinp` | dexalin, carbon, iron |
| 3 `synaptizine` | sugar, lithium, water |
| 3 `cryoxadone` | dexalin, water, oxygen |

## In the blood

Every 2 seconds each reagent in a living person acts and 0.4 units of it are used up.

| Reagent | Each cycle |
|---|---|
| `anti_toxin` | 2 less toxin damage, and clears 2 units each of toxin and sleep toxin |
| `kelotane` | heals 2 burn |
| `dermaline` | heals 3 burn |
| `bicaridine` | heals 1 brute |
| `dexalin` | 3 less oxygen damage |
| `dexalinp` | half the oxygen damage, at least 4 |
| `tricordrazine` | heals 1 brute and 1 burn, 1 less oxygen and toxin damage |
| `synaptizine` | gets you up if knocked down |
| `toxin` | 2 toxin damage |
| `stoxin` | knocks you down while it's in you |
| `sugar`, `nutriment` | feeds you |
| `inaprovaline`, `gold`, `silver`, others | nothing yet |

## Differences from Luna

| Luna | Here |
|---|---|
| About 200 reagents and over 100 recipes, some explosive | Only the medicines above, and the dispenser's 21. Plasma comes from nowhere yet, so dexalin can't be made |
| Reagents splash, and react to skin and floors | Only moved between containers and blood |
| Syringes draw blood, and spread viruses | Not built |
| You choose how much a beaker pours | Always 10 |
| Sleep toxin builds up: blurred eyes, then drowsy, then out | Down at once |
| Inaprovaline stops suffocation in crit | There's no suffocation in crit yet |
