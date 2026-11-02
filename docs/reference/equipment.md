# Equipment: clothing, gas tanks and internals

Code:
- `game/classic/src/equipment.rs` (slots, rules, internals)
- `game/classic/src/environment.rs` (breathing from a tank, suit protection)
- `game/classic/content/items.ron` (item values)

Client: the HUD slots in `apps/client/src/hud.rs` (`docs/reference/hud.md`) and `apps/client/src/sprites.rs` (`wear_layers`).

Tests: `game/classic/tests/equipment.rs`, `engineering_gear.rs`, and `decompression.rs` (magnetic
boots).

## Slots

Uniform, suit, head, mask, gloves, shoes, back, belt, ID. There are no pockets, ears or
glasses yet.

**Rules (Luna `mob/living/carbon/human/equip_if_possible`):**
- The item must be made for the slot.
- The slot must be empty.
- The ID and belt slots hang off the jumpsuit and need one on.
- Taking off the jumpsuit drops the ID and belt on the floor.
- Taking something off needs an empty active hand.
- Only a conscious person can change clothes.

New crew start in Luna's assistant outfit: a grey jumpsuit and black shoes.

## Movement

Worn suit and shoes add to the step delay (Luna `movement_delay`):

| Item | Slowdown |
|---|---|
| Space suit | +3 ds |
| Black shoes | −1 ds |
| Magnetic boots | −1 ds (Luna shoes' default) |
| Firesuit | +1 ds |

So a running human steps every 1.3 ds in shoes (the 2.3 ds base minus 1) and every 4.3 ds in a space suit.

## Gas tanks

| Tank | Volume | Fill | Slot |
|---|---|---|---|
| Oxygen tank | 70 L | 6 atm (607.95 kPa) O₂ | back |
| Emergency oxygen | 15 L | 2 atm (202.65 kPa) O₂ | belt |

A tank releases breaths at its release pressure. This is one atmosphere for now, as in
Luna `distribute_pressure`, and there is no regulator UI yet. A breath is 0.5 L at the lower
of the release pressure and the tank pressure. The exhaled breath goes into the room.

From full to one atmosphere a 70 L tank gives about 700 breaths, some 47 minutes. The
emergency tank gives about 30 breaths, roughly two minutes, and then keeps giving
thinner breaths.

## Internals

Internals need a breath mask on the face and a tank in a hand, on the back or on the belt. Tanks are
tried in that order: active hand, other hand, back, belt.

Toggle with the internals indicator on the HUD or E. Internals switch off when:
- the mask comes off;
- the tank is taken off the back or belt;
- the tank leaves the wearer in any other way (it is checked every breath).

## Space suits

A space suit plus a space helmet seal the wearer:
- **Pressure:** no pressure damage, at any pressure.
- **Heat:** vacuum cooling and heat exchange with the air are cut to 5%
  (`SUIT_INSULATION`), so the body's own heating wins.

A suit does not supply air. Without internals a sealed person still suffocates.

Measured in the engineering chamber with the outer door open:
- **Internals only:** the person keeps breathing, but takes pressure and cold damage.
- **Full EVA gear** (suit, helmet, mask, emergency oxygen on internals): unhurt and
  conscious after 120 s, with body temperature above 300 K.

## Engineering gear

From Luna's engineering section and tool closets:

| Prototype | Luna type | Slot | What it does here |
|---|---|---|---|
| `magboots` | `clothing/shoes/magnetic` | shoes | Space wind doesn't move the wearer (Luna `Airflow.dm` skips them) |
| `black-gloves` | `clothing/gloves/black` | gloves | Half a shock gets through (Luna gloves' `siemens_coefficient` 0.5) |
| `gas-mask` | `clothing/mask/gas` | mask | Takes air from a tank, like a breath mask |
| `firesuit` | `clothing/suit/fire` | suit | Slows by 1 ds |
| `hardhat` and `-orange`, `-red` (firefighter helmet), `-white`, `-blue` | `clothing/head/helmet/hardhat` | head | A lamp of 5, switched in hand (`flashlights.md`) |
| `meson-glasses` | `clothing/glasses/meson` | glasses | Out-of-sight tiles show dimly: the floor plan through walls (`lighting.md`) |
| `headset-eng` | `device/radio/headset/headset_eng` | ears | A radio: `;` talks into it (`radio.md`) |

Tool closets hold a hard hat three times in ten (Luna `toolcloset`).

**Not implemented:** what makes some of these matter in Luna has no counterpart yet:
- armour (hard hats' melee 30);
- heat and fire protection (firesuits, black gloves, the red and white hard hats, magnetic
  boots' `protective_temperature`);
- slipping (magnetic boots are `NOSLIP`);
- gas masks filtering smoke and gases;

A lit hard hat is drawn as `worn:<id>-on`: an item with a state is drawn as
`worn:<id>-<state>` when that sprite exists.

## Drawing

The client draws each worn item as `worn:<item id>` over the body. The layers, bottom to top:
uniform, ID, shoes, gloves, belt, suit, back, mask, head. Gloves are two sprites, one per
hand. "Lying" uses Luna's `…2` or `_l` states.

## Clothes for their looks

Luna's decks hold about a hundred more kinds of clothing: uniforms, costumes, hats, shoes, gloves,
armour and masks. They can all be picked up and worn in their slot, and shoes make you a little
faster, like the black shoes. For now that's all they do: armour doesn't protect, a muzzle doesn't
stop speech, a straight jacket doesn't bind, and a blindfold doesn't blind. Most coloured and
boxing gloves have no worn sprite in Luna's `hands.dmi`, so they don't show on the hands.
Space suits and helmets outside hardsuits import as the existing `space-suit` and `space-helmet`.
