# Implants

Sources: `luna:code/game/objects/items/weapons/implants.dm` (implants, the implanter, case and pad),
`luna:code/game/machinery/computer/prisoners.dm` (the prisoner management console),
`luna:code/defines/obj/storage.dm` and `code/WorkInProgress/Abi79/uplink_kits.dm` (the kits),
`luna:code/modules/surgery/implant_removal.dm`, `luna:code/game/machinery/teleporter.dm` (tracking
implants as targets), `luna:code/modules/mob/living/carbon/human/emote.dm` and `mob/living/say.dm`
(what sets implants off). `NSV_Luna.dmm`: two prisoner management consoles; no implants lie about.

Code: `game/classic/src/implants.rs`. Tests: `game/classic/tests/implants.rs`.

## The tools

- **Implanter** (`implanter`). With an implant in it, used on someone in reach or on yourself:
  "X is trying to implant Y with an implanter!"; 6 seconds later, if both are where they were and
  the implanter is still in hand, the implant goes into a random body part that has none ("X can't
  find anywhere to implant Y." when all eleven do). The implanter is then empty.
- **Glass case** (`implant-case`). An implanter used on a case moves the implant whichever way it
  can: from a loaded implanter into an empty case, or from a loaded case into an empty implanter. A
  syringe used on a case holding a chemical implant puts 5 units in ("The glass case is full." at
  50).
- **Implant pad** (`implant-pad`). A case used on it goes in. In the hand, it opens its window;
  clicked with the other hand, the case comes out into it. The window shows what's in the case: a
  tracking implant's frequency (144.1–148.9, odd tenths, in steps of 0.2 and 1.0) and id (1–100, in
  steps of 1 and 10) can be set there; a freedom implant is described; anything else is "not in
  database".

## The implants

| Implant | What it does |
|---|---|
| Tracking (`implant-tracking`) | Frequency 145.1 and id 1 to start. A teleporter computer lists it as a target, under its host's name, while the host lives or has been dead under ten minutes, and the teleporter sends you to the host. The prisoner console shows its id and the host's area |
| Chemical (`implant-chem`) | Holds 50 units, filled through its case. The prisoner console injects 1, 5 or 10 at a time ("You hear a faint \*beep\*."); when its host dies it injects all it has. Empty, it's gone ("You hear a faint click from your chest."). An EMP sets it off six times in ten: 20 units close in, 5 further out |
| Death alarm (`implant-death-alarm`) | When its host dies, every radio listening to the common channel says "X's death alarm: X has died in AREA!", once |
| Freedom (`implant-freedom`) | One to five uses. Once in, its host picks one of 17 emotes from a window. Doing that emote ("You feel a faint click.") drops their handcuffs to the floor and gives them 10 units of synaptizine |
| Explosive (`implant-explosive`) | Once in, the next thing its host says becomes its phrase, unheard by anyone. Whenever anyone within hearing of the host says something containing the phrase (whispers aside), the host dies and it goes off as an explosion of 1, 3, 4 and 6 |
| Compressed matter (`implant-compressed`) | Comes in its own implanter, which first takes in an item it's used on ("Please scan an object with the implanter first."). Once in, its host picks an emote; doing it brings the item back at their feet ("The air glows as the item uncompresses.") and the implant is gone |

## The prisoner management console

`prisoner-console`, powered. Locked, its only line unlocks it for someone whose held or worn card
has armoury access ("Unauthorized access." otherwise). Unlocked, it lists every chemical implant in
someone, with the host's name and units left and buttons to inject 1, 5 or 10; then every tracking
implant in someone, with its id and the host's area; and a line to lock it again.

## Taking one out

**Implant removal** is a surgery on any body part (`surgery.md`): incise, clamp, retract, extract,
close. The extract step takes a hemostat (100%), wirecutters (85%) or a crowbar (65%), 6.4 s. If the
part has an implant it comes out onto the floor; otherwise "X can't find anything in Y's part."

## Where they are

| Kit | Contents | From |
|---|---|---|
| Tracking implant kit | four tracking cases, an implanter, a pad | Tracking implants crate, 20, armoury access |
| Chemical implant kit | five chemical cases, an implanter, a pad | Chemical implants crate, 20, armoury access |
| Death alarm implant kit | five death alarm cases, an implanter, a pad | Death Alarm implants crate, 15, medical |
| Freedom implant box | an implanter with a freedom implant | Uplink, 3 crystals |
| Compressed matter implant box | the compressed matter implanter | Uplink, 5 crystals |
| Explosive implant box | an implanter with an explosive implant | Uplink, 6 crystals |

## Differences from Luna

| Luna | Here |
|---|---|
| The explosive implant's phrase is typed in a box | The next thing the host says, kept from everyone else: there's no text box to ask with |
| A case can be labelled with a pen | Not built (custom names, 4.7) |
| The freedom implant also gives hyperzine | Hyperzine isn't built |
| The explosive implant gibs its host | The host dies where they are (there's no gibbing yet) |
| Only the first implant in someone hears speech | Every implant in them does |
| The prisoner console can send a message into a tracked prisoner's head | Not built: there's no text box to type it in |
| The console only lists implants on the station's decks | Every implant |
| The chemical traitor, mind control, viral, alien and teleport implants | Not built: none are on the station or for sale |
| The hand-held locator picks up tracking implants on its frequency | There's no locator yet |
| A body scanner lists implants | Not shown |
