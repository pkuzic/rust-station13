# 0007: Server-decided positional sound with obstruction and air

- Status: accepted
- Date: 2026-10-31

## Context

Doors, tools, closets and lights had no sounds. Luna's `playsound` sends a sound to every
client within seven tiles at full volume, panned by the horizontal offset; walls, doors and
air do not matter. We want sounds that a player can use to tell where things are and what
is between them, that do not carry through vacuum, and that do not leak information the
player's character could not have.

## Decision

- **The server decides who hears what.** Gameplay pushes a `SoundEvent` (sound id, tile,
  volume, range, optional excluded mob) into the `SoundEvents` resource, as visible events
  work. At the end of each tick `deliver_sounds` computes, for each conscious mob, whether
  it hears each sound and how loud, and leaves it in the mob's `Earshot`. The server sends
  each client only what its mob heard, already attenuated.
- **Volume = source × distance × obstructions × air.** Distance falls linearly to 40 % at
  the range. Each obstruction passes 35 %, and more than two block the sound. Air uses the
  lower pressure of the two ends; below 10 kPa nothing carries, except to a listener on the
  very tile (contact, 50 %).
- **Obstructions come from the tile map**, on the same Bresenham line as sight: solid tiles
  and dense airlocks count one each, and crossing a closed edge counts one. Edges come from a
  `SoundBarriers` resource that whatever owns edge barriers (firedoors) fills in.
- **A `SoundMedium` trait** abstracts the world (`obstruction(tile)`, `edge_closed(a, b)`,
  `pressure(tile)`). Positions carry `z`, so multi-deck sound only needs the path to step
  between decks and the medium to count a floor as an obstruction; today different decks
  never hear each other.
- **State changes make sounds.** Airlock, closet and light sounds come from systems that
  compare each entity with the previous tick, so every cause (hands, crowbars, seals,
  autoclose, future explosions) is covered without touching each code path. Tool sounds are
  made where the tool is used.
- **Network:** a new `PlaySound { sound, at, volume }` message on an unordered, unreliable
  `SoundChannel`: a late sound is worse than a lost one. `PROTOCOL_VERSION` goes to 12.
- **Client:** Bevy's audio plays each clip through a small custom `Decodable` that pans it
  with rodio's `ChannelVolume` (rodio `=0.22.2`, the version `bevy_audio 0.20.1` uses). Bevy's
  own spatial audio is not used: it adds its own inverse-square falloff on top of the
  server's. Clips are Luna's `.ogg` files, embedded until the asset manifest exists.

## Consequences

- Players hear nothing they could not: a sound behind three walls, or in vacuum, never
  reaches their client.
- Every listener costs one line walk per sound per tick. With a handful of sounds per tick
  and a few dozen players this is negligible; if it is not, chunk-based interest management
  can prune listeners first.
- Volume rules live in one place, `sound.rs`, and are tested without a network.
- Firedoors and later barriers only need to fill `SoundBarriers`.
- A sound reaches listeners at the end of the tick of its cause.
