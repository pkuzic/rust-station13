# Tables and racks

Sources: `luna:code/game/objects/tables_racks.dm` (`/obj/structure/table`, `reinforced`,
`woodentable`, `/obj/structure/rack`: `attackby`, `CanPass`) and
`luna:code/game/objects/items/weapons/table_rack_parts.dm`.

Code: `game/classic/src/furniture.rs`. Tests: `game/classic/tests/tables.rs`.

## The structures

- **Tables** (plain, reinforced and wooden) and **racks** are dense: nobody walks through
  them. Gas passes over them.
- **Looks:** Luna's mappers set each table's look by hand: an icon state and a direction,
  such as an edge piece. Maps write it as `(id: "table", dir: West, state: Some("tabledir"))`,
  drawn as `tabledir` facing west. A table without one is the plain middle piece.

## Clicking

| Click | With | Does |
|---|---|---|
| A table or rack | any item but those below | puts the item down on it |
| A table or wooden table | wrench | "Now disassembling table." After 5 seconds, if you are still on the same tile holding the wrench, it becomes table parts. |
| A reinforced table | welding tool, lit or not | "Now weakening the reinforced table.", then after 5 seconds "Table weakened."; again on a weakened one: strengthening, "Table strengthened." |
| A weakened reinforced table | wrench | "Now disassembling the reinforced table.", then after 5 seconds reinforced table parts |
| A strong reinforced table | wrench | the wrench is put down on it, as in Luna |
| A rack | wrench | rack parts, at once |
| Table parts or rack parts | wrench | a metal sheet |
| Rack parts in the active hand | using it (Z) | a rack where you stand |

## Differences from Luna

- **The 5-second checks:** Luna checks that you stayed put and kept the tool only for plain and
  wooden tables. We check it for every timed job.
- **Building tables:** Luna builds a table from parts through a construction window where you
  pick the piece and its direction. That window isn't implemented, so table parts can only be
  turned into metal.
- **Not implemented:**
  - reinforced and wooden parts and a wrench (Luna makes plasteel and wood, which have no
    prototype yet);
  - putting a grabbed person on a table;
  - dropping items onto them by dragging;
  - explosions and blobs breaking them;
  - hulks smashing them, and small creatures hiding under them;
  - sounds.
