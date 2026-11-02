# 0008: Soft coloured tile lighting, worked out on the client

- Status: accepted
- Date: 2026-10-31

## Context

Light fixtures switched, burned out and drew power, but nothing got dark. Luna's
UltraLight gives each light a red, a green and a blue strength and lights each tile it can
see by that strength less the distance, in seven steps per colour, drawn as one of 512
overlay icons per tile. We want the same rules, coloured light, soft edges instead of
whole-tile steps, and something that keeps working once maps have several decks.

## Decision

- **The rules live in the engine.** `rs13_simulation::lighting` works out a `LightField`
  from `Emitter`s (position and RGB strength) over a `LightWorld` trait (`is_opaque`,
  `is_open_below`). It has no Bevy, game or rendering code, so the server can use it too
  when light starts to matter for gameplay.
- **Luna's arithmetic, without the rounding.** Per colour: strength less straight-line
  distance, summed over the lights in sight, capped by the strongest, over 7. Not rounding
  the distance is what makes light fade smoothly. Sight is a line to the tile's centre or
  near a corner, so walls cast shadows but are lit on the side facing the light.
- **Decks.** Light passes through open floors, losing one tile of reach per deck, and keeps
  going the way it went (down or up), so it can't bounce back and forth between decks.
- **The client works it out.** The server replicates `LightSource([u8; 3])` on lit fixtures,
  and `Opaque` on closed airlocks. The client recomputes the field only when those or the map
  change. Light is only looks today, and a full field per player each tick would cost
  bandwidth for nothing. Protocol 12 → 13.
- **Drawn as one multiply pass.** The client writes one texel per tile into an sRGB texture.
  A `Material2d` quad over the map samples it with smoothstep-weighted interpolation and
  blends with `src × dst`, so coloured light tints what is under it. It sits above every map
  sprite, and the HUD camera is not affected. No Bevy version change; the shader has no
  imports, so it does not depend on Bevy's shader modules.

## Consequences

- Darkness is cosmetic: a modified client can turn it off (F4 does). When gameplay needs
  light (stealth, cameras, plants), the server runs the same engine code and decides; the
  client keeps drawing.
- One texel per tile is soft, but a wall's lit face bleeds half a tile into the dark side.
  A finer texture or per-edge occlusion can come later without changing the rules.
- Recomputing everything on a change is cheap at the current map sizes (tens of lights,
  about a thousand tiles). Larger maps may need per-light caching or computing by chunk.
- Light from a lower deck lights the open tile above it, but the lower deck itself isn't
  drawn through the hole until the client draws several decks.

## Update (2026-11-02)

The blend between tiles is gone: each tile now shows its own light flat, right up to its
border, so shadows have hard tile edges, as in BYOND. Playing on the full station, the
softened borders read as smudges rather than as tiles. Light is still worked out by
straight-line distance and coloured as above; only the drawing between tiles changed.
