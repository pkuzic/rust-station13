# Sleepers and cryo cells

Sources: `luna:code/game/machinery/Sleeper.dm` (`sleep_console/Topic`, `sleeper/alter_health`,
`inject_*`), `luna:code/game/machinery/cryo.dm` (`process_occupant`); `NSV_Luna.dmm`: 5 sleepers with
their consoles and 3 cryo cells.

Code: `game/classic/src/medbay.rs`, `apps/client/src/sleeper_panel.rs`. Tests:
`game/classic/tests/medbay.rs`.

Pull someone and click one of these machines to lay them inside. Someone inside who is awake and walks
gets out, and so can someone in a DNA scanner now (`cloning.md`). Clicking a sleeper with nobody pulled
lets out whoever is in it.

## Sleeper (`sleeper`) and its console (`sleeper-console`)

Every 2 seconds someone lying in a sleeper is topped up to 5 units of inaprovaline, and if they have
health left, loses 1 oxygen damage, or 12 once it's below 10. It needs no power.

Clicking the console opens its window. It works the nearest sleeper within 3 tiles, shows who's inside and
how they are, and injects inaprovaline (30), sleep toxin, dermaline, bicaridine or dexalin (20 each), up
to 60 units of each. It needs power. Someone in crit can only be given inaprovaline, and someone past
saving nothing.

## Cryo cell (`cryo-cell`)

It takes gas from the pipe to its south. Clicking with an empty hand turns it on or off. Clicking with a
beaker loads it, and every 24 seconds it gives its occupant 1 unit.

While on and powered, with at least 10 moles in its pipes, the gas temperature decides what it does to a
living occupant every 2 seconds:

| Gas below | Effect |
|---|---|
| 273 K | 1 less oxygen damage |
| 225 K | also a 5% chance per part of 1 brute and 1 burn healed, and of 1 less toxin damage |
| 170 K | cryoxadone in them heals 3 of each, as well |

## Body scanner (`body-scanner`) and its console (`scanner-console`)

Source: `luna:code/game/machinery/adv_med.dm`. Lay someone inside as with a sleeper. It does nothing to
them, and needs no power. Clicking the console, with power, opens a window on the nearest scanner within 3
tiles: who's inside, their health, oxygen and toxin damage, and each damaged part's brute and burn.

Luna's window also shows radiation, viruses, broken bones, bleeding and implants, and can print the
results. None of that is here yet.

## Differences from Luna

| Luna | Here |
|---|---|
| The cell cools its occupant, whose body temperature then decides | Goes by the gas temperature directly |
| The cell keeps them asleep | They can walk out |
| The cell's window, and its gas slowly warming and venting | Clicks, and its gas stays as it is |
| A console is linked to what stands next to it | Nearest within 3 tiles |
| A window showing brute, burn and sleep readings | Health, oxygen and toxin damage, and the reagents it can inject |
