# Meteors and the blob

Luna's `code/game/gamemodes/meteor` and `blob`, in `game/classic/src/meteor.rs` and `blob.rs`.

## Meteor

A wave of three comes every 60 seconds, each 1 to 10 seconds into it. Each starts at a random open-space tile on an edge of a random deck, and flies at 10 tiles a second towards a random tile on the
opposite edge. Whatever it first meets, a wall or anything dense, it hits as a heavy blast, only on that tile, and it's gone. One time in seven or so it also explodes (2, 5, 8, 10). The round ends when the
pods get home. It's not on the vote list, as `votable = 0` in Luna, but it comes up by chance.

## Blob

Starts as one cell on a random floor of the station. Every 2 seconds, 25 of its cells at random (often the same one again) get a chance to grow, in percent, of their health plus 3 on a floor. One that
grows goes into every open tile beside it, giving the new cell its own health, and each tile that's blocked has an even chance of a light blast. It won't go into arrivals. A meteor can fall as it grows.

A cell starts with 30 health. Weapons do a quarter of their force, a lit welding tool 15. Light and heavy blasts do 15 to 30, and a devastating one destroys it.

| When | What happens |
|---|---|
| 20 to 90 seconds in | A biohazard alert |
| 5 to 10 minutes later | The outbreak's confirmed |
| Then each minute | Over 500 cells, the directive to destroy the station |
| A minute later | The station's destroyed, and the staff has lost |
| Any time | With no cells left, the staff has won |

It's only ever forced, as `enabled = 0` and its probability is 0 in Luna.

## Differences from Luna

| Luna | Here |
|---|---|
| Meteors heat the air they pass through | Not yet |
| Screens shake near an impact | Not yet |
| Each object has its own `meteorhit`, and most do nothing | A heavy blast on the tile, for all of them |
| Each object has its own `blob_act` | A light blast on the tile, for all of them |
| A meteor round says who survived and how | The usual roll of who escaped |
| The blob starts at a `blobstart` landmark, which Luna's map doesn't have | A random floor |
| Guns hurt the blob | Not yet: the blob's cells aren't hit by shots |
| The AI gets a quarantine law, and comms consoles print the alert | No AI yet, and the alert's a line of text |
| The blast's "AI has won" ending | No AI yet |
