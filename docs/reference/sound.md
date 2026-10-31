# Sound

Sources: `luna:code/game/sound.dm` (`playsound`, `playsound_local`) and the `playsound`
calls in `luna:code/game/machinery/doors/airlock.dm`, `luna:code/game/objects/closets.dm`,
`luna:code/modules/power/lighting.dm` and the tool uses across `luna:code/`.

Code: `game/classic/src/sound.rs` (who hears what), `apps/server/src/lib.rs`
(`send_sounds`), `apps/client/src/audio.rs` (playing). Tests: `game/classic/tests/sound.rs`
and the unit tests in both modules. Design: `docs/adr/0007-positional-sound.md`.

## Luna's behaviour

- `playsound(source, file, volume, vary, extrarange)` sends the sound to every mob with a
  client within `world.view + extrarange` tiles (7 by default) of the source, as a square
  (BYOND `range`).
- The volume is the same for everyone in range: there is no falloff, nothing in the way
  matters, and neither does air.
- A sound from a turf is panned by `dx / 8`: fully to one side eight tiles away.
- `vary` plays it at a random pitch.
- `"shatter"` and a few other names pick one of several files.

## Ours

### Who hears

- Gameplay makes a sound at a tile with a volume (0–100, Luna's scale) and a range, which
  is 7 tiles plus Luna's `extrarange`. It may leave out one mob.
- At the end of each tick, every **conscious** mob within range (a square, as in Luna) on
  the same deck may hear it. Unconscious and dead mobs hear nothing, as with speech.
- The server sends each player only the sounds their mob hears, already at the volume they
  hear them. Quieter than 1 is not sent.

### How loud

The volume at the source is multiplied by:

| Factor | Value |
|---|---|
| Distance (straight line, up to the range) | 1 − 0.6 × distance / range: 40 % at the edge |
| Each obstruction on the way | 35 %; three or more and nothing gets through |
| Air | √((p − 10) / (101.325 − 10)), p the lower pressure of the two ends in kPa, clamped to 0–1 |

- **Obstructions** are counted on the Bresenham line from the source to the listener (the
  same line as sight). Each wall tile counts one; so does each closed airlock (closed,
  opening or closing, as for walking through). Every crossed edge between two tiles with a
  closed barrier on it counts one too: this is how firedoors join in
  (`sound::SoundBarriers`). The tiles at either end never count, so a door is not muffled
  by itself.
- **Air:** below 10 kPa at either end, no sound carries. A sound on a door tile uses the
  better side of the door. Without a simulated atmosphere, air is assumed.
- **Contact:** someone on the very tile of a sound still feels it through the floor or the
  tool in their hand at 50 % at least, even in vacuum.

### What is heard

| Sound id | When | Volume, range | Luna file |
|---|---|---|---|
| `airlock` | an airlock starts opening or closing, for any reason (hand, card, crowbar, autoclose, air alarm seal) | 30, 7 | `machines/airlock.ogg` |
| `door-deny` | an airlock refuses someone without access | 30, 7 | `machines/buzz-two.ogg` |
| `closet-click` | a closet, locker or crate opens or closes, for any reason | 15, 4 | `machines/click.ogg` |
| `glass-hit` | a tube or bulb breaks, for any reason | 75, 7 | `effects/Glasshit.ogg` |
| `ratchet` | a wrench connects or disconnects a canister, starts taking a table or rack apart, or turns parts into a sheet | 50, 7 | `items/Ratchet.ogg` |
| `crowbar` | a crowbar pries up floor tiles or forces an airlock | 50, 7 | `items/Crowbar.ogg` |
| `screwdriver` | a screwdriver opens or closes an airlock's panel | 50, 7 | `items/Screwdriver.ogg` |
| `wirecutter` | wirecutters cut or mend a door wire or a cable, or cut an air alarm | 100, 7 | `items/Wirecutter.ogg` |
| `welder` | a lit welding tool is used on anything in reach | 50, 7 | `items/Welder.ogg`, `items/Welder2.ogg` in turn |

Airlock, closet and light sounds come from watching what changed during the tick, so any
cause makes them. Tool sounds are made where the tool is used, at the target's tile.

### On the client

- Each sound plays once at the volume the server sent.
- It is panned by how far left or right of the player's mob it came from: the near side
  stays full, the far side fades to nothing at eight tiles (Luna's `dx / 8`).
- The files are embedded in the client until the server's asset manifest delivers them.
  Attribution: `client-assets/sounds/ATTRIBUTION.md`.

## Differences from Luna

- **Distance, walls, doors and air matter.** Luna plays every sound at full volume to
  everyone in range, through walls and in vacuum. Ours falls off with distance, is muffled
  or stopped by walls, closed airlocks and closed edges, and does not carry through thin air
  or vacuum.
- **Who decides:** the server decides who hears a sound and how loud; Luna sends it to
  every client in range and lets BYOND play it.
- **Unconscious mobs** hear nothing. Luna sends sounds to every mob with a client.
- **Airlock deny** buzzes (`buzz-two.ogg`). Luna only flashes the deny light.
- **Pitch:** Luna's `vary` (random pitch) is not done; clips play at their own pitch.
- **Variants:** where Luna picks a random file, we play them in turn.
- **Tool sounds** are made when the click did something with the tool. Luna plays them
  only on success; a few of our cases (a cable click that is refused, say) may still click.
- **Not implemented:**
  - footsteps;
  - ambience and area music;
  - other Luna sounds (`sparks`, `punch`, `rustle`, explosions and the rest);
  - sound across decks (one deck per map for now; see the ADR);
  - per-player volume settings.
