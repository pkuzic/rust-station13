# Spells

Luna's `code/datums/spell.dm` and `code/datums/spells/*`, in `game/classic/src/spells.rs`, with the
list in `game/classic/content/spells.ron`. Client: `apps/client/src/spell_panel.rs`. Tests:
`game/classic/tests/spells.rs`.

Luna's wizard game mode isn't compiled, so nobody starts as a wizard. Spells come from:
- an admin's **Give Spell** (here the admin line's `spell WHO SPELL`, at Administrator rank);
- the **Mime** job, which starts with Invisible Wall.

Luna's other ways in, the wizard station's spellbook and the wizard crate, come with 2.9's items.

## Casting

F7 opens the spell window, if you know a spell. Each line shows the spell and how far it has
recharged, in seconds (Luna's "Spells" panel). A spell that needs a place asks for one.

Casting checks, in Luna's order:
1. It's recharged, or you're told "{spell} is still recharging".
2. You're conscious.
3. **People only** (Luna checks nobody else, and doesn't charge them):
   - a spell with words can't be cast muted or silenced ("You can't get the words out!");
   - a spell that needs the robes wants a wizard's robe, sandals and a wizard's hat, in that order. The
     costume ones count, as in Luna.
4. It starts recharging. If it then finds nothing to act on, the charge comes back.
5. The words are said: shouted, or whispered. Half the time the spaces become backticks.

## The spells

| Spell | Recharge | Robes | Words | What it does |
|---|---|---|---|---|
| Magic Missile | 15 s | yes | FORTI GY AMA | A missile at each living mob in sight within 7. Each homes in a step every half second, through anything, for up to 20 steps. On arrival: knocked down for 5 counts, and 10 burn. |
| Disable Tech | 40 s | yes | NEC CANTIO | An EMP on the caster, 6 heavy and 10 light. |
| Blink | 2 s | yes | — | To a random floor 1 to 6 tiles away, never within 6 of the map's edge. With none, nothing happens. |
| Teleport | 60 s | yes | SCYAR NILA + the place | To a random clear floor of the place chosen. That's any area but shuttles, the syndicate's, the wizard's and space. |
| Forcewall | 10 s | no | TARCOL MINTI ZHERI (whispered) | A wall on the caster's own tile for 30 s. |
| Fireball | 6 s | no | ONI SOMA | A fireball the way the caster faces, a tile a decisecond. It bursts (light 2, flash 3) when it meets something solid or dense, when anyone else is beside it, or after 200 tiles. |
| Knock | 10 s | no | AULIE OXIN FIERA (whispered) | Every airlock within 3 unbolted and opened, if it has power. |
| Invisible Wall | 30 s | no | — | The mime's: a wall nobody can see on the caster's tile, for 30 s. |

## Not built yet

These spells wait on systems that don't exist yet:

| Spell | Waits on |
|---|---|
| Mutate | Genetic powers: hulk, X-ray, cold resistance, no breath (3.5) |
| Disintegrate | Gibbing |
| Smoke | Smoke clouds |
| Ethereal Jaunt, Phase Shift | Passing through walls unseen |
| Blind | Blindness and blurred sight |
| Curse of the Horseman | The horse mask and its voice |
| Mind Transfer | Swapping bodies. Luna's is broken: it ghosts the target and swaps nothing |
| The construct spells | Cult floors and walls (cult isn't compiled) |

## Differences from Luna

| Luna | Here |
|---|---|
| Spells show in a "Spells" tab of the stat panel | A window, on F7 |
| Smoke, steam and sparks come with some spells | Not shown: there's no smoke yet |
| Conjuring a wall takes a moment, and can be interrupted | It's there at once |
| Bumping into the mime's wall shows others they "look as if a wall is in front of them" | Nothing is shown |
| Knock opens every door: windoors, firedoors | Airlocks only |
| A Magic Missile's burn is spread over the body | It's on the chest |
| The mime is also silenced for good | Not yet |
