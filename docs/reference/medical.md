# Medical

Sources:
- `luna:code/game/objects/items/stacks/medical.dm` (bruise packs, ointment);
- `luna:code/game/objects/items/devices/scanners.dm` (`healthanalyzer/attack`);
- `luna:code/game/objects/storage/firstaid.dm` (what the kits hold).

Code: `game/classic/src/medical.rs`; damage in `game/classic/src/health.rs` (`health.md`).
Tests: `game/classic/tests/medical.rs`.

## Packs

Use one on a person within reach, yourself included, with it in the active hand.

| Prototype | Luna type | Heals | Stack |
|---|---|---|---|
| `bruise-pack` | `stack/medical/bruise_pack` | 30 brute | 10 |
| `ointment` | `stack/medical/ointment` | 20 burn | 10 |

- **Where:** on the chest, Luna's default target, since there is no zone selector yet.
  Damage never goes below zero.
- **Critical condition:** a pack does nothing for someone below 0 health (Luna
  `check_health`).
- **Use:** each application uses one from the stack; the last one is gone.
- **Seen:** everyone who can see it is told "X has been applied with the bruise pack by Y",
  or "Y applied the ointment on themselves".

## Health analyzer

`health-analyzer`, worn on the belt. Used on a person, it tells its user, as in Luna:
- overall status: health as a percentage, or dead;
- suffocation, toxin, burn and brute damage;
- body temperature in °C and °F;
- brute and burn on each damaged body part, or "Limbs are OK.";
- one line for each kind of damage: severe over 50, otherwise normal.

Others who can see it are told "Y has analyzed X's vitals." Numbers are rounded to whole
units.

## First aid kits

| Prototype | Luna type | Holds |
|---|---|---|
| `first-aid-kit` | `storage/firstaid/regular` | 2 bruise packs, 2 ointments, a health analyzer |
| `first-aid-kit-fire` | `storage/firstaid/fire` | 2 ointments, a health analyzer |

Luna's engineering section has two regular kits, imported with the map.

## Differences from Luna

- Luna's kits also hold a band-aid, an inaprovaline syringe and, in the fire kit, kelotane
  pills. There are no reagents or bleeding yet, so they are left out.
- The analyzer's colours, bold "severe" numbers and bleeding, blood, brain, fracture,
  disease and reagent lines are left out.
- The clumsy "analyzing the floor" joke is left out.
- **Not implemented:** advanced trauma and burn kits, the toxin and oxygen kits.
