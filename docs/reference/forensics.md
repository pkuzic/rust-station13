# Forensics

Fingerprints and fibres left on what people touch, the detective's scanner and fingerprint
cards, and the forensic scanning computer. Behaviour reference: Luna `add_fingerprint`
(`atom_procs.dm`), `DetectiveWork.dm` and `detective_scanner` (`scanners.dm`).
Code: `game/classic/src/forensics.rs`; tests: `game/classic/tests/forensics.rs`.

## Prints

- A person's fingerprint is a 32-digit hex hash of their genes' unique identifier (the
  part of their DNA that sets how they look), so changing how someone looks through
  genetics changes their prints too.
- Whenever a person uses their hand on something in reach, including picking it up, it
  keeps their print. Using a held item on something leaves prints on both. The scanner is
  the exception: it leaves none on what it reads.
- A thing keeps the last **three** different prints. A print that's already there moves to
  the end, and the oldest is dropped.
- Gloves leave no prints, and neither does the **no-prints** gene. Mobs themselves never
  keep prints.

## Fibres

Items (not machines or furniture) keep a trace of what the toucher wore, by chance on each
touch. Each trace is kept once.

| Worn | Fibre | Chance |
|---|---|---|
| A suit | the suit | 10% |
| A suit that leaves the arms bare (no 32 in `covers`) | the uniform | 12% |
| A suit that leaves the hands bare (no 64 in `covers`) | the gloves | 20% |
| No suit | the uniform | 25% |
| No suit | the gloves | 20% |

## Forensic scanner

- It holds **20** blank cards. Used in the hand, it turns printing on or off (its sprite
  shows `forensic1` while printing).
- **On a person**: tells you their fingerprint, or that none were found if they wear gloves
  or have no prints.
- **On a thing**: lists the prints on it ("Isolated N fingerprints"), or says none were
  found.
- While printing is on, each reading that finds prints uses one blank card and drops a
  **fingerprint card** with those prints at your feet. With no blanks left, printing turns
  off.
- A stack of blank fingerprint cards (10) used on the scanner tops it up to 20. Whatever
  doesn't fit stays in the stack.
- A fingerprint card used in the hand reads out its prints, and whose they are if they were
  taken from a person.

## Forensic scanning computer

The high-res computer and the detective's older "PowerScan Mk.I" behave the same. The window
is a pick list:

- **Log in** needs a held card with `forensics_lockers` access.
- **No object inserted** takes the item in the active hand. Clicking the inserted object
  ejects it onto the computer's tile.
- **Scan** takes three stages of 5 s (25%, 50%, 75%) and can be cancelled. It then lists the
  item's prints, numbered, and its fibres.
- **Print** puts the result on a sheet of paper, in the reader's language.
- **Erase data** clears the result. Short notes ("Scan data printed.", "Print failed: no
  data.", "Invalid object rejected.") stay until cleared.

## Not done

- Blood: Luna also reports blood DNA and types, and tracks bloody footprints. There is no
  blood here yet.
- Evidence bags and the fingerprint case's card list (the case is an inert item).
- Security records don't show fingerprints yet; that needs a protocol change to
  `RecordRow`.
- The admin-only log of everyone who touched something (Luna `fingerprintshidden`).
