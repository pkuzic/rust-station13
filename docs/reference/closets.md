# Closets, lockers and crates

Sources:
- `luna:code/game/objects/structures/crates_lockers/closets.dm`;
- `closets/secure/secure_closets.dm` and `closets/secure/engineering.dm`;
- `closets/utility_closets.dm` (the emergency closet);
- `crates.dm`;
- `luna:code/defines/obj/closet.dm` (the tool closet that Luna compiles).

Code: `game/classic/src/closets.rs`; the kinds and what they start with are in
`content/closets.ron`. Tests: `game/classic/tests/closets.rs`.

## The kinds

| Prototype | Luna type | Starts with |
|---|---|---|
| `closet` | `closet` | nothing |
| `closet-emergency` | `emcloset` | 3 emergency oxygen tanks, 3 breath masks |
| `closet-tools` | `toolcloset` | wrench, wirecutters, screwdriver, crowbar; a mechanical toolbox (or, one time in five, an electrical one); maybe a cable coil (50 %), multitool (30 %), utility belt (20 %), insulated gloves (10 %) |
| `locker-chief-engineer` | `secure_closet/engineering_chief` (access `heads`) | insulated gloves, mechanical toolbox, utility belt, multitool |
| `locker-electrical` | `…/engineering_electrical` (`engine_equip`) | 2 insulated gloves, 3 electrical toolboxes, 3 multitools |
| `locker-welding` | `…/engineering_welding` (`engine_equip`) | nothing yet |
| `locker-engineer` | `…/engineering_personal` (`engine_equip`) | utility belt, mechanical toolbox, insulated gloves |
| `closet-coffin` | `closet/coffin` | nothing |
| `locker-personal` | `secure_closet/personal` (`all_personal_lockers`) | nothing yet (Luna: a backpack and a headset) |
| `locker-brig` | `secure_closet/brig` (`security`) | nothing yet (Luna: an orange jumpsuit and shoes) |
| `locker-security` | `secure_closet/security` (`security`) | nothing yet (Luna: an officer's gear) |
| `crate` | `crate` | nothing |
| `crate-internals` | `crate/internals` | an oxygen tank half the time |
| `crate-radiation` | `crate/radiation` | nothing yet |
| `crate-rcd` | `crate/rcd` | nothing yet |

Luna's other contents have no prototype here yet. `content/closets.ron` lists what is left
out.

## Closets and lockers

- **Contents:** a closet keeps what is inside it. A closed closet on the map takes in the
  loose items on its tile when the round starts.
- **Opening:** opening drops everything inside onto its tile. An open closet is out of the
  way, so people can walk onto its tile.
- **Closing:** closing takes in up to 50 things from its tile: items, and the people
  standing there.
- **People inside:** someone shut in is hidden and can't reach anything outside. Walking
  pushes at the door:
  - a closet opens unless it is welded ("It won't budge!");
  - a locker opens unless it is locked ("The locker is locked!"). In Luna a welded but
    unlocked locker still opens from inside.

### Clicking a closet

| With | The closet | Does |
|---|---|---|
| empty hand or any item | closed | opens. Welded: "It won't budge!" |
| lit welding tool | closed | welds it shut, or unwelds it; everyone around sees "The closet has been welded shut by …" |
| empty hand | open | closes. Another closet on the tile: "It won't budge!" |
| an item | open | puts the item down in it |
| lit welding tool | open | cuts it apart into a metal sheet |

### Lockers

A secure locker has a card lock and starts locked. It reads the card in the active hand,
or else the one in the ID slot (Luna `allowed`).

- **Working the lock:** any item, or an empty hand when the locker won't open (locked or
  welded), works the lock. With access: "The locker has been unlocked by …", seen by
  everyone around. Without: "Access Denied".
- **Opening:** an empty hand opens an unlocked, unwelded locker.
- **Welding:** a lit welding tool welds lockers as it does closets.

## Crates

A crate holds items like a closet, with these differences:

- it stays in the way when open;
- it never takes people in;
- welding doesn't seal it, and doesn't cut it apart;
- it opens and closes on any click when closed.

## Differences from Luna

- **The tool closet:** its 80 % mechanical toolbox roll is the same as Luna's. Luna's
  emergency closet sometimes gets an engineering oxygen tank; ours always gets the plain
  one.
- **Fuel:** a lit welding tool burns fuel on a closet, as on anything it is used on
  (`welding.md`). Luna asks for none.
- **Personal closets:** Luna's also opens for the first ID card swiped on it, and after that
  for its owner's card; ours opens only for its access, since ID cards don't carry names yet.
- **Brig lockers:** Luna bolts them down; ours can be pulled like any locker.
- **Not implemented:**
  - rigging with cable and electropacks;
  - emags and hacktools;
  - breaking lockers;
  - stuffing people in by dragging or grabbing;
  - the "BANG, bang!" heard when someone inside pushes;
  - explosions, blobs and bullets breaking closets;
  - secure crates;
  - the other closet kinds: fire, radiation, bomb suit, wardrobes and the rest;
  - sounds other than the open and close click (`sound.md`).

## The rest of Luna's closets

The wardrobes, lockers and biohazard closets on Luna's decks are all here with their looks and access, and hold what we have of their contents. What they'd hold beyond that isn't built yet, so some are nearly empty.
