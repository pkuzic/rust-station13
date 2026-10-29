# Reference spec: hands, items and clicking

Sources:
- `luna:code/game/objects/items/item.dm`: `/obj/item/attack_hand`
- `luna:code/modules/mob/mob.dm`: `swap_hand`, `drop_item`
- `luna:code/game/atom_procs.dm`: click handling (`DblClick`)

## Luna behaviour

**Hands.** A human has a left and a right hand; one is active (`mob.hand`). Swapping toggles
the active hand. Each hand holds at most one item.

**Clicking something within reach** (on the same tile or one of the 8 neighbours):
- **Active hand empty:**
  - on an item: it is picked up into the active hand;
  - on a machine (e.g. a door): its `attack_hand` runs.
- **Active hand holding an item:** `target.attackby(item, user)` runs. For a door, any item
  is treated like using it by hand.
- **Clicking the item you are holding** in the active hand runs its `attack_self`.

Picking up shows no chat message. A destroyed hand refuses the pickup with a joke message.

**Dropping** places the active-hand item on the mob's own tile.

**Examining** (shift-click, in reach or not) shows the item's description, starting with
"That's a crowbar."

## Implemented (`game/classic/src/hands.rs`, `actions.rs`)

| Behaviour | Status |
|---|---|
| Two hands, active hand, swap | keep |
| Pick up an item in reach with an empty active hand, no message | keep |
| Use a door in reach by hand or with any item: toggles it | keep |
| Drop the active-hand item onto the mob's tile | keep |
| Examine any visible object: "That's a {item}." | keep (descriptions come later) |
| Reach: same tile or the 8 neighbours | keep |

## Not implemented yet

Item-on-item `attackby` (e.g. using wirecutters on cables), `attack_self`, throwing,
storage, equipment slots, organ checks on hands, examine descriptions, intents
(help/disarm/grab/harm).
