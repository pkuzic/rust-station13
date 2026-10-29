# Vision

Recreate the atmosphere, gameplay depth and visual identity of Space Station 13 as it
was around 2012–2013, using **NSV Luna** as the reference build, on a new Rust engine
that adds community modding, autonomous NPCs, multi-z stations, deeper simulation and
persistent worlds.

**Modern technology underneath, classic SS13 on the surface.**

## What "classic" means here

- **Interaction depth is the game.** Hands, intents, tools used on things, wires,
  internals, paperwork, improvisation. Simulation should be able to surprise players.
- **Systems, not scripts.** Atmos, power and damage are simulations that interact. A
  breach is the result of pressure, not a scripted event.
- **The station is a workplace.** Bureaucracy, departments, jobs, access levels, chain
  of command. The fun comes from people doing (and failing at) their jobs.
- **2012-era look and feel.** Flat 32×32 top-down sprites, Windows-classic panels,
  dense information, CRTs and beige retrofuturism.

## What we modernise

- Legibility, lighting, responsiveness, input latency and stability.
- Server authority, sandboxed mods, validated asset downloads.
- Reproducible bugs: headless scenarios and regression tests for simulations.
- Localisation: Russian and English as first-class languages from day one.

## What we refuse

- Simplifying interactions to make the game more approachable.
- Modern-style pixel art, neon, holograms or sleek sci-fi minimalism.
- Running untrusted code on players' machines.
- Depending on a central hub to play.

## Principles

1. The engine is distinct from the classic game build; community systems use published
   extension interfaces.
2. The server owns all gameplay state; clients send intentions.
3. Explicit interfaces and testable subsystems; architecture review for cross-module
   changes.
4. Start with one playable room. Prefer a working vertical slice over an empty framework.
5. Direct server connections always work, with or without the hub.
