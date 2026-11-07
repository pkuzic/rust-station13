# Genetics

Sources: `luna:code/game/dna.dm` (`/datum/dna`, the block helpers, `updateappearance`,
`domutcheck`), `luna:code/game/machinery/dnaconsole_new.dm` (`computer/scan_consolenew`, the console
on the map), `luna:code/defines/obj/injector.dm` and `items/weapons/misc.dm` (`dnainjector`),
`luna:code/game/gamemodes/setupgame.dm` (`setupgenetics`), `luna:code/modules/mob/living/carbon/life.dm`
and `WorkInProgress/NewMobs and etc/human_life.dm` (radiation and what the genes do);
`NSV_Luna.dmm`: one DNA modifier console beside two scanners in research, deck 3.

Code: `game/classic/src/genetics/` (`dna.rs` the strings, `powers.rs` the genes, `mod.rs` the
console and injectors), `apps/client/src/genetics_panel.rs`. Tests: `game/classic/tests/genetics.rs`,
and the unit tests in `dna.rs`.

## Genes

Everyone who is a person or a monkey has genes, made the first time something reads them:

- **Unique identifier** (UI), 13 blocks of three hex digits: how they look. Hair colour (red, blue,
  green, as Luna stores them), the same again for facial hair, skin tone, eye colour, gender, beard
  and hair style. A block of 2050 or more (shifted, below) is a woman's.
- **Structural enzymes** (SE), 25 blocks: their powers and defects. Each block starts between 1 and
  1024, so off. Block 14 makes a monkey; a monkey's starts on.
- **Unique enzymes** (UE): 32 hex digits from their name; the same name always gives the same.

At the start of the round each gene below is given a block of its own at random, and half the time
every threshold is shifted by -300 to +100 (Luna `BLOCKADD`). A block is on at 2050 or more, Hulk
and Telekinesis at 3500, X-ray and Cold resistance at 3050.

When the enzymes are read again (Luna `domutcheck`), every defect whose block is on is had, with
its message, every time. A power newly on takes hold only at a chance (Hulk 15%, Telekinesis 20%,
the rest 30%) unless it was had already, or radiation's mutation forces it. One that doesn't take
hold can on a later read.

| Gene | Message | What it does here |
|---|---|---|
| Hulk | Your muscles hurt. | Never knocked down, stunned or knocked out. A punch does 5 more, knocks down for 2 counts and pushes them a step away. Lost at health 25 or below: "You suddenly feel very weak.", knocked down 3 counts |
| Cold resistance (Luna's fire block) | Your body feels warm. | Takes no burns at all. Every 2 s heals 1 burn if burns total up to 50, 5 if 51 to 100 |
| No breathing | You stop breathing. | Doesn't breathe: no air needed, nothing in it harms |
| Regeneration | You feel strange. | Every 2 s, 2 brute and 2 burn off every body part, 2 oxygen and 2 toxin damage |
| Deafness | It's kinda quiet... | Hears nothing |
| Epilepsy (Luna's headache block) | You get a headache. | 1% every 2 s: "You have a seizure!", out for 10 counts |
| Cough | You start coughing. | 5% every 2 s: drops what's in hand, and coughs |
| Tourette's (twitch) | You twitch. | 5% every 2 s: twitches, knocked down 5 counts |
| Fake | You feel strange. | No gene of its own: each read, 95% another defect's block or a look block is changed, else a great power's |
| Running, blending | You feel quick. / You feel alone. | Nothing, as in Luna |
| X-ray | The walls suddenly disappear. | Sees through walls and closed doors and in the dark, all the way round, as far as anyone sees: people, things and the station itself. Not ghosts, and not what's under the floor |
| Blindness | You can't seem to see anything. | Sees nothing: the view is black, and nothing but themself and what they carry is sent to them. Blindness wins over X-ray |
| Telekinesis, clumsiness, stuttering, remote view, project mind, morph, hallucination, no prints, shock immunity, small size | (theirs) | Had, but nothing yet: see Not built |

Block 14 on turns a person into a monkey: all they had falls, they're named `monkey (xxxx)` from
their enzymes, and take 20 toxin and 40 brute. Off again, a monkey becomes a person looking as
their identifier says, under the name they had, or a new one, with the same harm.

## Radiation

Every 2 s (Luna `handle_mutations_and_radiation`):

- over 100: down to 100, knocked down 10 counts, "You feel weak.", collapses;
- 1 to 49: 1 less, and 25% of the time 1 toxin damage;
- 50 to 74: 2 less, 1 toxin; 5% of the time 15 less more, knocked down 3 counts, "You feel weak.";
- 75 to 100: 3 less, 3 toxin; 1% of the time "You mutate!": 90% a defect's block flipped, else a
  great power's, the genes read with powers forced, a gasp and 50 less.

## The DNA modifier console

`genetics-console`, beside a scanner (`dna-scanner`, north, east, south or west of it). It needs
power. Clicking it opens its window; anyone in reach can press its buttons.

- **Main**: who is in the scanner, their health and state, radiation, UE, UI and SE. A husk's genes
  are "ruined beyond recognition". Buttons for the pages below, Pulse Radiation, Inject
  Inaprovaline (10 units, unless they have 50 already), and Lock or Unlock: locked, they can't get
  out until it's unlocked or they're let out by it being emptied.
- **Emitter**: duration 2 to 20 seconds, by twos, and intensity 1 to 10.
- **Modify Unique Identifier**: the string, every digit to click; block, sub-block (digit) and a
  target digit 0 to F to step through. Irradiate: after the duration, 80% + duration/2 of the time
  the digit is pushed towards the target (the target itself, or one either side, likeliest; two
  away less; three away least, more so the longer the beam), and they change to suit. Otherwise
  (20% + intensity) a defect's block flipped, else a look block set anew. Radiation: intensity +
  duration, or twice the intensity + duration when it misses.
- **Modify Structural Enzymes**: the same, with no target. A hit (95% + duration/2) scrambles the
  digit, now and then (5%) the one on the block beside it instead; a miss (80% - duration) flips a
  defect's block, else changes a look block. Then the enzymes are read again.
- **Pulse Radiation**: after the duration, 95% of the time either a defect's block flipped (75%) or
  a look block set anew; else a great power's block flipped (95%) or a look block. Nothing shows
  until the genes are next read. Radiation: 3 × intensity + 3 × duration.
- While the beam is on the window waits and the scanner is held shut.
- **Buffers**, three: save the occupant's UI, UI with UE (and so their name), or SE. Each can be put
  into the occupant (radiation 20 to 50), made into a full injector or one of a single block, or
  cleared. The replicator is ready 25 seconds after the console is built and rests 20 seconds
  after each injector: "Replicator not ready yet."

## DNA injectors

Used on a person, themself included: "X is trying to inject Y with the DNA-Injector!", and after 4
seconds, if neither has moved, it's still in hand and they're awake and uncuffed, it goes in. A
monkey is injected at once. Either way: radiation 20 to 40, and the injector is used up.

- An identifier injector sets their whole identifier, or one block, and they change to suit; one
  saved with its enzymes gives them the name too.
- An enzymes injector sets them, or one block, and the genes are read again, powers at their
  chance.

Ready-made ones (Luna's, for admins: `dna-injector-<name>`): hulk, xray, fire, tele and the defects
cough, clumsy, stutter, blind, deaf, each with an `anti-` one; `h2m` (into a monkey) and `m2h` (back).
Their block is this round's. `box-dna-injectors` holds three of each of the last two.

## Differences from Luna

| Luna | Here |
|---|---|
| Any of the 25 blocks, the monkey's 14 included, can be a gene's | Not 14: in Luna whoever has that gene on is a monkey too |
| A man's gender block can reach the threshold, one time in about two thousand | Never |
| Hair read back can land a style off from what was written | It comes back as written |
| Facial hair has its own colour | It's the hair's: the look has one colour for both |
| The unique enzymes are an MD5 of the name | A hash of the name of the same length |
| Buffers are labelled, and can be relabelled | Labelled by what they hold |
| Data disks to save and load buffers | Not built: none are on the map |
| The iso injector asks for a block in a list | The window shows the blocks to pick |
| A monkey becomes a person under a random name | Under the name they had before, if any |
| Stuns are short pauses standing | Here, as for every stun, being knocked down |
| Tourette's makes them shout swear words two times in three | They only twitch |
| A hulk's punch pushes them two steps | One |
| Hulks are green | They look the same |
| The blind are spared what's seen in messages (a visible message comes as what's heard, or not at all) | They still read what others do in the chat |

## Not built

Telekinesis (reach at any distance), clumsiness (guns, batons, cuffs, grenades turned on themself), stuttering,
Remote View, Project Mind, Morph, hallucinations, no fingerprints (no forensics yet), shock immunity,
small size (under tables), and hulks smashing walls, windows, tables and grilles. Mutagen, the
gene analyser and the wizard's Mutate spell.
