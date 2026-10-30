# Containers: toolboxes and belts

Sources: `luna:code/game/objects/storage/storage.dm` (`attackby`, `attack_hand`,
`show_to`, `orient_objs`), `toolbox.dm` and `belts.dm`.

Code: `game/classic/src/storage.rs`. Content: `game/classic/content/items.ron`.
Client: `apps/client/src/storage_panel.rs`. Tests: `game/classic/tests/storage.rs`.

## Sizes

Every item has a size, Luna's `w_class`, from 1 (tiny) to 5 (huge). Luna's default is 3.

| Size | Items |
|---|---|
| 1 | screwdriver, ID card |
| 2 | crowbar, wrench, wirecutters, multitool, cable coil (Luna 1.5), emergency oxygen tank, gloves, breath mask |
| 3 | most other things, e.g. oxygen tank, metal sheet, jumpsuit, utility belt |
| 4 | toolboxes, space suit |

## Containers

| Container | Holds | Starts with |
|---|---|---|
| Mechanical toolbox (size 4, force 10) | anything smaller than itself | screwdriver, wrench, crowbar, wirecutters |
| Electrical toolbox (size 4, force 10) | anything smaller than itself | screwdriver, wirecutters, crowbar, three cable coils |
| Utility belt (belt slot) | crowbar, screwdriver, wirecutters, wrench, multitool, cable coil, at any size, never another container | nothing; the loaded belt has the four tools and a cable coil, and the full one adds a multitool |

Luna's welders, analyzers and scanners have no prototype yet, so they are left out of the
starting contents. Luna's electrical toolbox swaps its third coil for insulated gloves one
time in ten; ours always has the coil.

**Limits:** a container holds at most 7 things.
- If it is full, or the thing is too big, nothing happens, as in Luna.
- If a belt refuses something, you see "The utility belt can't hold X."
- When something goes in, everyone who can see you sees "X has added Y to Z!".

## Clicking

| Click | With | Does |
|---|---|---|
| A container you hold or wear | empty hand | opens it |
| A container on the floor, in reach | empty hand | picks it up |
| A container anywhere in reach | an item | puts the item in |
| An item in the open container | empty hand | takes it out into the hand |
| Something you wear | empty hand | takes it off into the hand |
| The item in your other hand | empty hand | moves it to the active hand |

**On the HUD:**
- Clicking a worn slot or your other hand with an empty active hand clicks the item in it.
  This opens a worn belt or a toolbox in your other hand.
- Clicking a slot while holding something still puts that thing on.

**The open container:**
- It shows as a column of cells at screen x 4, from row 10 down, one per slot (Luna's
  `orient_objs(4, 10, 4, 3)`), with the close button under them.
- It closes when you put the container down, take it off into a place other than your
  hands, or are knocked out.

## Differences from Luna

- **Size rule:** Luna also refuses two size-1 things in each other. Here that is already
  covered by "smaller than itself".
- **Pockets:** containers in pockets can't be opened in Luna; there are no pockets yet.
- **Not implemented:**
  - dragging a container onto yourself to open it;
  - folding boxes;
  - mousetraps;
  - the "rustle" sound;
  - other containers (backpacks, boxes, first-aid kits, closets).
