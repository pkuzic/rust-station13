# Client assets

Sprites the client draws, imported from NSV Luna's `.dmi` icon files, and the sounds it
plays, copied from Luna's `sound/` folder.

| File | What |
|---|---|
| `sprites.ron` | **Hand-edited.** Which Luna icon and state each of our sprites uses. |
| `sprites/atlas.png` | Generated. Every frame we use, packed into 32×32 cells. |
| `sprites/atlas.ron` | Generated. Sprite → state → frames, directions, delays, cells. |
| `sprites/ATTRIBUTION.md` | Generated. Every imported sprite with its source file and state. |
| `sounds/` | Luna `.ogg` files, copied unchanged, in Luna's subfolders. |
| `sounds/ATTRIBUTION.md` | **Hand-edited.** Every sound with its source file and sound id. |

The client embeds `atlas.png` and `atlas.ron` at build time, until the server's asset
manifest delivers them instead.

## Updating the sprites

Edit `sprites.ron`, then regenerate (with a Luna checkout next to this repository):

```sh
cargo run -p rs13-dmi-import -- ../luna client-assets/sprites.ron client-assets/sprites
```

Commit `sprites.ron` and the three generated files together.

## Licence

Luna's art and that of its upstreams (/tg/station, Baystation12) is **CC-BY-SA 3.0**,
and so are the atlas and anything derived from it. `sprites/ATTRIBUTION.md` must list
every imported sprite and `sounds/ATTRIBUTION.md` every sound. Never import from Luna's `icons/Goon*` folders: Goonstation art is
non-commercial (CC-BY-NC-SA). See `docs/adr/0003-licensing.md`.
