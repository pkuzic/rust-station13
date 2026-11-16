# Surgery

Procedures done step by step on someone lying down, and the operating table. Behaviour
reference: Luna `modules/surgery` and `OpTable.dm`. Code: `game/classic/src/surgery.rs`;
tests: `game/classic/tests/surgery.rs`.

## Starting a procedure

- Use **surgical drapes** on a person, on help intent, aiming at a body part (the zone
  selector). They must be lying down: resting on a table or bed, knocked down, out cold or
  dead.
- The part must be bare: no worn clothing covers it (the item's `covers` bits, as Luna's
  `body_parts_covered`). Otherwise: "You need to expose their … first."
- The drapes open a list of the procedures that part allows and that aren't already under
  way there. Picking one starts it on that part.

## Steps

Each step is done by using a tool on the patient (or an empty hand, where the step wants
one), on help intent, aiming at the procedure's part. Anything else used on them lands as an
ordinary blow, as in Luna.

The step takes its time, and it's lost if the surgeon or the patient moves or the surgeon
changes what's in hand. It then works by the tool's chance times the place's:

| Patient lies on | × |
|---|---|
| An operating table | 1.0 |
| A roller bed | 0.9 |
| A table | 0.8 |
| A bed | 0.6 |
| Anything else | 0.4 |

A failed step usually has to be done again ("… screws up!").

| Step | Time | Tools (chance) |
|---|---|---|
| Incise | 2.4 s | scalpel 100, knife 85, shard 65 |
| Clamp bleeders | 4.8 s | hemostat 100, wirecutters 70, cable coil 15 |
| Retract skin | 3.2 s | retractor 100, screwdriver 65, wirecutters 35 |
| Close | 3.2 s | cautery 100, lit welding tool 70 |
| Saw bone | 6.4 s | circular saw 100, hatchet 55 |
| Extract appendix | 6.4 s | empty hand |
| Extract brain | 6.4 s | hemostat 100, crowbar 55 |
| Fix eyes | 6.4 s | hemostat 100, screwdriver 65, pen 45 |
| Reshape face | 6.4 s | scalpel 100, knife 50, wirecutters 35 |
| Reshape genitals | 6.4 s | scalpel 100, hatchet 50, wirecutters 35 |
| Cavity | 3.2 s | empty hand, or anything |

## Procedures

| Procedure | Part | Steps | Effect |
|---|---|---|---|
| Appendectomy | groin | incise, clamp, retract, incise, extract appendix, close | Drops an appendix (once) |
| Brain removal | head | incise, clamp, retract, saw, extract brain | Drops a brain; the patient dies |
| Eye surgery | head | incise, retract, clamp, fix eyes, close | Cures blindness and nearsightedness; blurry for 35 s. A failure does 20 brute to the head |
| Plastic surgery | head | incise, retract, reshape face, close | A new random name |
| Gender reassignment | groin | incise, clamp, reshape genitals, close | Flips male and female. A failure leaves it to chance but still moves on |
| Cavity implant | any | incise, clamp, retract, incise, cavity, close | An item goes in (see below), or an empty hand takes one out |

A cavity holds one item: up to size 1 in the head, hands and feet, 2 in the arms and legs,
and 3 in the chest and groin. Organs don't go in. A step that can't fit the item, or finds
nothing to take out, has to be done again.

## Operating table

- A grab used on the table lays whoever it holds on it, lying down ("… has been laid on the
  operating table by …").
- They stay lying until they leave the table's tile.
- The table shows `table2-active` while someone is on it and `table2-active-dead` if they're
  dead.

## Not done

- Luna's "eyes" location is part of the head here, since the zone selector has no eyes.
- Bone fixing, limb removal and replacement: there are no bones or severable limbs yet.
- Implant removal and xenomorph or slime procedures: those systems don't exist yet.
- Appendicitis (it's commented out in Luna too). Brain loss: eye surgery failure does head
  damage instead.
- A brainless body can still be cloned.
