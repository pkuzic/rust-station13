# Eyes

Sources: `luna:code/modules/mob/living/carbon/life.dm` and `WorkInProgress/NewMobs and etc/human_life.dm`
(`eye_blind`, `eye_blurry`, `disabilities`, `sdisabilities`, the blindfold, the `blind`, `blurry`
and `vimpaired` screen overlays), `luna:code/modules/mob/living/carbon/human/hud.dm` (the overlays'
shapes), `luna:code/datums/spells/inflict_handler.dm`, `genetic.dm` and `wizard.dm` (Blind, Mutate),
`luna:code/game/gamemodes/changeling/changeling_powers.dm` (Blind Sting),
`luna:code/game/gamemodes/wizard/spellbook.dm` (the scrying orb).

Code: `game/classic/src/eyes.rs`; the server's sight in `apps/server/src/sight.rs`; the screen in
`apps/client/src/world_view.rs` and `world_view.wesl`. Tests: `game/classic/tests/eyes.rs`,
`apps/headless/tests/vision.rs`.

## How someone sees

Worked out on the server every tick, from:

| Cause | Effect |
|---|---|
| The Blindness gene (`genetics.md`) | Blind |
| Blinded for a while: the Blind spell, Blind Sting | Blind, until it wears off |
| A blindfold over the eyes | Blind, while it's worn |
| The X-ray gene, the Mutate spell, the scrying orb | X-ray, unless blind: blindness wins |
| Blurred for a while: the Blind spell, Blind Sting | Blurred |
| Nearsighted for a while: the Blind spell, Blind Sting | Nearsighted, unless wearing glasses (`glasses`, Luna's `glasses/regular`) |

Luna counts blindness and blur in life ticks, one off every 2 seconds; here they last that long.

- **Blind**: the server sends nothing but the player and what they carry, and the client draws the
  map black (the HUD stays). Nothing they'd see happen reaches their chat (Luna's
  `visible_message`).
- **X-ray**: the server sends what a ghost would see, through walls, in the dark and all round,
  minus ghosts and what's under the floor. The client turns its darkness, shadows and back arc
  off. What they'd see happen reaches them through walls too, as far as anyone sees.
- **Blurred**: the whole view is blurred.
- **Nearsighted**: only the middle of the view, about a third of it across, is clear: the rest is
  dim and blurred (Luna's half-dithered border).

The player's own vision and eye trouble ride in their status, as health does.

## What causes it

| Cause | Blind | Blurred | Nearsighted |
|---|---|---|---|
| Blind spell (`spells.md`) | 10 ticks (20 s) | 20 ticks (40 s) | 30 s |
| Blind Sting (`changeling.md`) | 20 ticks (40 s) | 40 ticks (80 s) | 30 s |

A second dose keeps whichever runs out later.

## Differences from Luna

| Luna | Here |
|---|---|
| The blind still get a visible message's `blind_message`, where it has one | Nothing: none of ours has one |
| Nearsightedness is a dithered border of whole tiles | A soft-edged dim blur, sized to the view |
| A welding helmet, the flash and other things blind or blur too | Only what's listed above so far |
