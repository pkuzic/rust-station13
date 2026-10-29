# Atmospherics feature specification

Source of truth for atmos, from the project owner's *SS13 Rust Atmospherics Feature
Specification & Inspirations*. Decision record: `docs/adr/0006-hybrid-atmos.md`. What
Luna itself did is recorded separately in `docs/reference/atmos.md`.

**Design target:** classic 2012–2015 tg/Bay/Luna atmosphere for 1–15 players, as a hybrid
ZAS/LINDA simulation.

**Approach:**
- ZAS-inspired bulk room zones for cheap, stable areas.
- LINDA-inspired local gas cells for leaks, combustion and gradients.
- SS14-inspired engine modularity.
- Simplified physics for pressure, thermodynamics and momentum.

Prefer convincing, testable gameplay over full computational fluid dynamics.

**Space must be robust, deadly and dangerous.**

Priority: **P0** playable prototype, **P1** full atmos gameplay, **P2** advanced extension.

## 1. Core simulation

| Feature | How it should work | Inspiration | P |
|---|---|---|---|
| Atmospheric zones | Airtight connected spaces share a bulk gas mixture; walls, doors and seals define connectivity | ZAS | P0 |
| Tile-level simulation | Activate individual gas cells around leaks, fires and significant gas or heat gradients | LINDA | P1 |
| Dynamic zone splitting | Closing/opening doors updates zone connectivity; transfer gas conservatively through openings | ZAS | P0 |
| Gas mixtures | O₂, N₂, CO₂, plasma, N₂O, total moles and temperature | LINDA / SS13 | P0 |
| Ideal gas law | P = nRT/V using known gas volume | Physics | P0 |
| Conservation | Preserve moles and thermal energy across transfers except modelled reactions/sinks | Physics | P0 |
| Pressure equalisation | Move gas between unequal-pressure regions with flow limits from opening geometry | ZAS + physics | P0 |
| Gas diffusion | Species gradients and local clouds in active cells | LINDA | P1 |
| Sleeping regions | Skip stable regions until an external event wakes them | LINDA | P0 |
| Fixed simulation steps | Deterministic tick scheduling for gas, heat and mechanical effects | SS14 / ECS | P0 |

## 2. Decompression and structural damage

| Feature | How it should work | Inspiration | P |
|---|---|---|---|
| Hull breaches | Open a connection from an area to vacuum when the hull fails | ZAS / Bay | P0 |
| Decompression | Vent in proportion to opening size, pressure and gas available | ZAS + physics | P0 |
| Space wind | Forces on characters and unsecured objects from flow direction and magnitude | ZAS | P0 |
| Flow obstruction | Narrow gaps, furniture and open doors constrain routes | LINDA + physics | P1 |
| Door resistance | Pressure-rated doors hold a difference until a failure threshold | Bay + engineering | P1 |
| Window failure | Crack or break under pressure or impacts | Physics | P2 |
| Structural damage | High-energy pressure events damage doors, walls, equipment | SS13 | P1 |
| Emergency shutters | Isolate zones after sensor-detected pressure failure | Bay | P1 |
| Vacuum exposure | Unprotected characters suffer asphyxiation and thermal exposure | Classic SS13 | P0 |
| Atmos alarms | Report dangerous pressure, temperature and gas composition | Bay | P0 |

## 3. Thermal and fire

| Feature | How it should work | Inspiration | P |
|---|---|---|---|
| Gas temperature | Temperature and internal energy per gas volume | LINDA | P0 |
| Heat exchange | Energy transfer among adjacent active regions and surfaces | LINDA / physics | P1 |
| Material temperature | Heat capacity and conductivity for walls, floors, equipment | Physics | P1 |
| Combustion | Consume fuel and oxygen above ignition; produce heat and products | LINDA | P1 |
| Fire propagation | Spread through hot, fuel-rich, oxygenated areas | LINDA | P1 |
| Flash fires | Fast but finite propagation across flammable local regions | SS13 | P1 |
| Fire suppression | Extinguishers, sprinklers, atmospheric removal | Bay | P1 |
| Thermal damage | Damage beings and structures under extreme heat | SS13 | P1 |
| Thermal expansion | Heating a sealed volume raises pressure | Physics | P0 |
| Cooling systems | Exchangers and refrigeration | SS13 | P2 |

## 4. Engineering equipment

| Feature | How it should work | Inspiration | P |
|---|---|---|---|
| Pipes | Connected gas networks separate from room atmos | LINDA / Bay | P0 |
| Pumps | Move gas toward pressure targets, limited by power and capacity | SS13 | P0 |
| Valves | Open/close pipe segments | Classic SS13 | P0 |
| Air vents | Inject air into rooms or extract it through pipes | Bay | P0 |
| Air scrubbers | Extract selected species into waste piping | Bay | P0 |
| Gas filters | Separate species in pipe flow | LINDA | P1 |
| Gas mixers | Blend inlets at configured ratios | LINDA | P1 |
| Storage tanks | Finite-volume compressed gas | SS13 | P0 |
| Portable canisters | Connect to pipes/machines or release locally | Classic SS13 | P0 |
| Heat exchangers | Move heat without mixing gases | SS13 | P2 |
| Control consoles | Sensor readings and manual control | Bay | P1 |
| Automation | Sensors drive vents, alarms, shutters | Bay + industrial | P2 |

## 5. Advanced gameplay

| Feature | How it should work | Inspiration | P |
|---|---|---|---|
| Gas stratification | Temporary height-dependent concentrations where useful | Physics | P2 |
| Smoke | Particulates move with airflow, obscure vision, affect breathing | Fire sim | P1 |
| Toxic atmospheres | Gas-specific effects by concentration and exposure | SS13 medicine | P1 |
| Respiration | Characters consume O₂ and produce CO₂ in sealed rooms | SS13 / physics | P1 |
| Equipment leaks | Damage creates flow-limited openings from tanks/pipes | Engineering | P1 |
| Explosions | Rapid energy release, heat, mechanical damage | Physics / SS13 | P2 |
| Atmos audio | Leak hiss, rushing air, alarms, vacuum silence | Classic SS13 | P1 |
| Debug overlays | Visualise pressure, gases, heat, zones, flow vectors | LINDA tooling | P0 |
| Reactive environment | Alarms and failures drive other systems | Bay / SS14 | P1 |
| Vertical airflow | Shafts, vents and floor breaches between z-levels | SS14 / SS13 | P2 |

## 6. Simulation architecture

- **Stable zones:** one gas mixture per airtight connected space. Process only on change
  or scheduled checks.
- **Active local cells:** high-resolution cells around leaks, fires, gas clouds and sharp
  thermal gradients.
- **Zone/cell coupling:** an active cell *replaces*, not duplicates, the same gas volume of
  its parent zone. Exchange through explicit boundary fluxes.
- **Return to sleep:** when local gradients fall below thresholds, merge active cells
  conservatively back into the bulk zone.
- **Correctness:** test gas mass, energy accounting, reproducibility, door transitions and
  long-term numerical stability.

## 7. Roadmap

| Phase | Scope | Acceptance |
|---|---|---|
| 1 Functional atmos | Zones, mixtures, pipes, pumps, vents, tanks, vacuum, basic breaches | Opening an airlock depressurises a room and machinery restores pressure |
| 2 Local simulation | Active cells, local leaks, gradients, space wind, conservative boundary transfer | A hull breach produces directional flow; a minor leak causes a temporary local cloud |
| 3 Heat and chemistry | Combustion, conduction, reactions, filters, fire control | A plasma fire consumes oxygen, heats the room, spreads appropriately and can be extinguished |
| 4 Optimisation | Sleeping regions, adaptive detail, structural effects, smart alarms, vertical flow | Idle station areas cost almost nothing |

## 8. Further reading

tgstation atmos (`code/modules/atmospherics`), Baystation12, Space Station 14, and auxmos
(a Rust atmos implementation). ZAS and LINDA vary by fork and revision; they are
inspirations, not claims about any single build.
