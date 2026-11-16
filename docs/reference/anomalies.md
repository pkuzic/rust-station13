# Anomalies

Sources: `luna:code/modules/research/anomaly/` (`anomaly.dm`, `effect.dm`, `ano_device.dm`,
`effects/*.dm`, `machinery/analyser.dm`, `machinery/harvester.dm`); `NSV_Luna.dmm`: the
anomalist's lab on deck 2, with the artifact in its storage room, the analyser and harvester, and a
table with two utilizers and four batteries.

Code: `game/classic/src/anomalies.rs`. Tests: `game/classic/tests/anomalies.rs`.

## The artifact (`alien-artifact`)

A large alien device, dense, in one of six looks, each dull or lit (`ano00` to `ano51`). It can be
pulled, except while the analyser or the harvester is working on it.

It has a **primary effect**, and three times in four a **secondary** one. Each effect is rolled
from Luna's seventeen compiled kinds, with:

- **A way of working**: on **touch** (whoever touches it), as an **aura** (everyone in range,
  every machine cycle while it's on), or in **pulses** (everyone in range, each time it has
  charged, while it's on).
- **A range and a charge time**, one of three kinds: short (1–3 tiles, 3–20 cycles) twice as often
  as medium (5–10, 15–40), four times as often as long (10–20, 20–120).
- **A trigger** that turns it on, and again off.
- **A signature**, a Greek letter and a number such as `sigma-417`, which the harvester shows.

Some kinds change what they roll: an EMP always pulses; a force field always works by touch;
heat and cold work by touch or as an aura.

### Triggers

| Trigger | Set off by |
|---|---|
| Touch | A bare hand; one walking into it bare-handed, half the time |
| Water | A container with water or hydrogen used on it |
| Acid | Acid, polytrinic acid or diethylamine |
| Volatile | Plasma or thermite |
| Toxin | Toxin, cyanide, amanitin or neurotoxin |
| Force | A blow of 10 force or more; a bullet; a blast |
| Energy | A stun baton that's on, a multitool; a laser or a taser shot |
| Heat | A lit welding tool, lighter or match; a blast; air over 375 K |
| Cold | Air under 225 K |
| Plasma, oxygen, carbon dioxide, nitrogen | 10 moles of that gas on its tile |

Every trigger flips the effect, so the second time turns it off again. Air triggers follow the
air instead: on while it holds, off once it doesn't. Station air holds well over 10 moles of
oxygen and of nitrogen, so those two wake as soon as the air reaches them.

A trigger always reaches the primary effect. It reaches a secondary one only a quarter of the
time, and quietly: nothing shows it waking.

When the primary wakes or sleeps, the artifact lights or dims and those near see it: it
"momentarily glows brightly!" or "grows dull!", among other lines.

Touching it with gloves on does nothing. Bare-handed, a touch-mode primary works on you whether
or not it's on, and a touch-mode secondary only while it's on.

### Susceptibility

Most effects on people work at a chance, and with a strength, of their **susceptibility**. It
starts at 1. Gloves take 0.1 off and science goggles another 0.1; energy armour, averaged over
the body, takes a fiftieth of its value. A full anomaly suit and hood, at 90 each, protects
completely. Anyone other than a person is fully susceptible.

### Effects

| Kind | Touch | Aura | Pulse |
|---|---|---|---|
| Heal | Heals 25 of each kind on every part, then more brute, burn, toxin and oxygen damage; feeds and takes radiation away | Heals 1 of each | Heals 5 of each |
| Hurt | 5–25 of each kind, radiation, hunger, knocked down | 1 of each | 3 of each |
| Stun | Knocked down 45 counts | Sometimes knocked down 2 | Knocked down 4 |
| Teleport | Thrown to a random floor within 50 tiles, in sparks | Everyone within 30 | Everyone within 15 |
| Bad and good feelings | Dread or euphoria, or a milder line | Now and then | Often |
| DNA switch | Genes scrambled, look or powers | Sometimes | People within 200 tiles |
| Radiate | Radiation | Radiation to all in range | 25 times as much |
| Sleepy | Blurred sight | A little blur | More blur |
| Heat, cold | The air on its tile warmed or cooled 5–50 K | Towards a set temperature, a degree at a time | |
| Cell charge, drain | | Every APC cell within 200 tiles, and SMES in range | More |
| EMP | | | An EMP of half its range |
| Force field | Sixteen dense field tiles in a ring two tiles out, which follow it; off again takes them down | | |
| Robot heal, hurt | Nothing: there are no cyborgs or robotic limbs yet | | |

## The analyser (`anomaly-analyser`)

Clicked with power, it opens a list: "Begin scanning." or, while it scans, "Halt scanning."

- **Begin**: it scans what stands on the tile east of it, preferring an artifact. With nothing
  there: "Unable to isolate scan target." An artifact the harvester is using gives "Too much
  interference." Otherwise "Scanning begun.", and the artifact is held still.
- After **10 s** (Luna's 16 s, less its stock parts) it says "Scanning complete." and prints
  "Anomaly analyser report #N". For an artifact the report reads its primary effect: what it gives
  off, how, and what kind of trigger wakes it (physical, energetic or the air). If a secondary
  effect is on, a warning and the same for that one follow. The supermatter crystal gets its own
  line. Anything else is "mundane". If the thing was moved off the pad: "Unable to locate scanned
  object."

What it gives off is the effect's emission, by kind: concentrated energy, a psionic wavefront,
electromagnetic energy, particles, organically reactive particles, bluespace phasing or atomic
synthesis. "How" follows the emission too, as Luna has it, not the way the effect works.

## The harvester (`anomaly-harvester`)

A battery used on it goes in ("There is already a battery" if one is). Clicked with power, its
list shows the charge and the battery's signature ("???" while it holds nothing), and offers
Eject, Drain and Begin harvesting. While it works it shows its progress and Halt early.

- **Harvest**: it wants one artifact alone on the tile east of it, with nothing and no one else
  there ("Error isolating energy signature"; nothing at all: "No noteworthy energy signature
  isolated"). It copies the artifact's primary effect into an empty battery, or tops up one that
  holds the same signature and kind ("Incompatible energy signatures detected" otherwise). It
  then charges it by 1 a cycle, holding the artifact still, until the battery is full (200). It
  draws 750 W while working and 50 W idle, and stops without power.
- **Drain**: with charge in the battery, it turns the battery's effect on and dumps 4 a cycle,
  working the effect where the harvester is. A touch effect works on anyone next to it with its
  list open. Empty, the effect goes off: "Battery dump completed."
- **Eject** stops whatever it's doing and puts the battery on the floor.

## Batteries and the utilizer

**Batteries** (`anomaly-battery`) hold an effect and up to 200 charge. Their look shows the
charge in quarters. A battery that's run empty forgets its effect the next time it's harvested.

**The utilizer** (`anomaly-utilizer`), worn on the belt or carried. A battery used on it goes
in. Used in the hand, its list shows the battery's signature, its charge and the timer, with
buttons to change the timer by 10 or 100 (never below 0 or over the battery's capacity). Start
runs the effect where the utilizer is, for as long as there's charge. Start in timed mode runs it
for as many cycles as the timer says, and the timer reads as Luna shows it, a tenth of that.

While running, the effect works every cycle, and a touch effect works on whoever holds it. Each
cycle takes 1 charge. When it stops ("buzzes"), it needs half its timer to cool down before it
"chimes" and can start again. Eject stops it and drops the battery.

## Not built

- Dizziness, drowsiness, stuttering and stuns are knocked down or left as words: there is no
  dizziness, drowsiness or stutter yet. A DNA switch doesn't change species: there are no mutant
  races.
- Thrown things don't set off force (there is no throwing yet).
- The analyser and harvester can't be opened or taken apart, and have no part upgrades.
- Bluespace "interdimensional" artifacts, excavation and the gas effects Luna leaves uncompiled.
