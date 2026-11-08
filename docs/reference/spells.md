# Spells

Luna's `code/datums/spell.dm` and `code/datums/spells/*`, in `game/classic/src/spells.rs`, with the
list in `game/classic/content/spells.ron`. The wizard's things (Luna `gamemodes/wizard`) are in
`game/classic/src/wizard_items.rs`. Client: `apps/client/src/spell_panel.rs`,
`apps/client/src/spellbook_panel.rs`. Tests: `game/classic/tests/spells.rs`,
`game/classic/tests/wizard_items.rs`.

Luna's wizard game mode isn't compiled, so nobody starts as a wizard. Spells come from:
- an admin's **Give Spell** (here the admin line's `spell WHO SPELL`, at Administrator rank);
- the **Mime** job, which starts with Invisible Wall.

- the wizard's things below: the spellbook, the magic scrolls. Luna keeps them at its wizard
  station, which isn't on the decks imported here, and in the admin's wizard crate
  (`crate-wizard`: the teleportation scroll, the spellbook, the magic staff, sandals, robe and hat).

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
| Blind | 30 s | no | STI KALY (whispered) | Someone living in sight within 7, picked from a list: "Your eyes cry out in pain!" Blind for 20 s, blurred for 40 s, nearsighted for 30 s (`eyes.md`). With nobody picked in sight, the charge comes back. |
| Mutate | 40 s | yes | BIRUZ BENNAR | The caster only: "You feel strong!" Hulk, X-ray, cold resistance and no breathing for 30 s (`genetics.md`), on top of their own genes. |

## The wizard's things

Used in the hand unless it says otherwise.

**Spellbook** (Luna `spellbook`). Opens its window: spells left to learn (6 to start), each spell
with its recharge in seconds and what it does, the artefacts, and Re-memorize. A line is taken by a
person who is awake, not cuffed, and holding the book or beside it:
- **A spell** uses one. One they know already costs nothing: "You already know that spell."
- **Summon Guns** uses one for good (the most it can teach goes down too). It puts a gun at the feet
  of every living person who has a player and isn't already an antagonist, the wizard included.
  One in four of them also becomes a survivor: a traitor whose only objective is to stay alive.
  The gun is one of those built so far: taser, energy gun, laser gun, revolver, syndicate revolver,
  pistol, double-barrel, combat shotgun.
- **Mastercrafted Armour Set** uses one for good: sandals, purple gloves and the gem-encrusted
  hardsuit and its helmet, at their feet. The hardsuit and helmet count as the robes and hat for
  spells, are a space suit, and make the wearer a little faster (slowdown −1).
- **Scrying Orb** uses one for good: the orb at their feet.
- **Re-memorize** works only inside the wizard's sanctuary (an area named with `wizard`): every
  spell is forgotten and the book can teach as many as it could before. Elsewhere: "You can only
  re-memorize spells in the wizard's sanctuary."
- With none left, a line does nothing.

**Magic scrolls** (Luna `spellbook/oneuse`): fireball, blink and forcewall. Read once, the spell is
learned; read again, nothing. Smoke's waits on smoke.

**Teleportation scroll** (Luna `teleportation_scroll`). Opens a list of places, the same as
Teleport's. Picking one, by an awake, uncuffed person holding it or beside it, uses one of its four
jumps: to a random floor of that place with nothing dense on it. No words are said and no robes
are needed.

**Wizard's staff and broom** (Luna `staff`): something to hit with, force 3. One lies on the
costume rack with the wizard's robe and hat.

**Magic staff** (Luna `staff/magical/test`), used on something further than reach, at no charge and
with no words:
- an airlock is unbolted and opened, if it has power (Knock on that tile);
- a floor that isn't solid gets a forcewall for 5 seconds;
- a person: Luna hands its fireball the person instead of their floor, so nothing flies. Here too.
- used in the hand, Luna's ethereal jaunt: waits on passing through walls.

**Scrying orb** (Luna `scrying`). Buying it also gives X-ray vision for good ("The walls suddenly
disappear."), unless they have it. "You can see... everything!", and those around see them stare
into it. They leave their body as a ghost. Clicking their own body, while it lives, puts them
back in it. It hits like a heavy object: force 15.

## Not built yet

These spells wait on systems that don't exist yet:

| Spell | Waits on |
|---|---|
| Disintegrate | Gibbing |
| Smoke | Smoke clouds |
| Ethereal Jaunt, Phase Shift | Passing through walls unseen |
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
| The spellbook lists every spell | Only the ones built |
| The spellbook says Mutate gives telekinesis, and that it recharges in 60 s | It says what it does: X-ray, 40 s |
| The spellbook's answers replace its window until Clear | They go to chat, and the window stays |
| A magic scroll teaches a spell they know again, as a second copy | They just know it |
| Smoke when the teleportation scroll jumps | No smoke yet |
| The scrying orb burns | Brute, like anything thrown or swung |
| The hardsuit reflects a fifth of shots | Nothing reflects yet |
| A ghost from the orb uses Re-enter Corpse, even into a dead body | Clicks its body; only while it lives, as death is final here |
| The wizard crate has a blue jumpsuit | Not built yet |
