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
| About 200 reagents, and recipes that mix them | Only these so far, and no mixing |
| Reagents splash, and react to skin and floors | Only moved between containers and blood |
| Syringes draw blood, and spread viruses | Not built |
| You choose how much a beaker pours | Always 10 |
| Sleep toxin builds up: blurred eyes, then drowsy, then out | Down at once |
| Inaprovaline stops suffocation in crit | There's no suffocation in crit yet |
