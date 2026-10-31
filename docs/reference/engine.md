# Reference spec: the engine

Luna's engine room turns heat into power. The supermatter crystal, lit by four lasers,
heats the gas in the core chamber. Thermal plates in the chamber carry that heat into a
hot pipe loop. A thermo-electric generator (TEG) between the hot loop and a cold loop
makes power from the difference.

Sources: `luna:code/modules/power/generator_type2.dm`,
`luna:code/ATMOSPHERICS/components/unary/generator_input.dm`, `thermal_plate.dm`,
`luna:code/WorkInProgress/ZeroPoint/SuperMatter.dm`, `Laser2.dm`, `LaserComputer.dm`.

## Thermo-electric generator (Luna `generator_type2`)

- **Inputs:** a generator input on the tile east of it and one west; each is the generator's connection to a pipe loop. Without both it is broken.
- **Each cycle:** of the two inputs' gas (each a unit's own 200 L), the hotter is the hot loop.
  - Heat moved: `ΔT × Ch × Cc / (Ch + Cc) × transferpercent / 100` (70 on Luna's map).
  - Efficiency: `(1 − Tc / Th) × 0.75`.
  - Power: heat × efficiency × `outputpercent` (1 on the map). The rest of the heat warms the cold loop.
- **Lights:** 11 output lights, `teg-op1` … `teg-op11`, at `round(11 × output / 100 000)`.
- **Window:** output, and each loop's temperature and pressure.

## Thermal plate (Luna `unary/thermal_plate`)

- **With air around it:** each cycle, a quarter of the plate's gas and a quarter of its tile's air even out their temperatures.
- **With less than 10 mol around it** (space): a quarter of its gas evens out with a radiation sink of 30 000 J/K at 6.4 K, so it cools the loop.

## What we build (`game/classic/src/teg.rs`, `machines.rs`)

| Item | Ours | From Luna |
|---|---|---|
| Generator inputs, thermal plates | unary pipe devices of 200 L, dense, facing the map's way | keep |
| Thermal plate | as above, every machine cycle | keep |
| TEG | as above, every machine cycle, with Luna's map settings (`ObjectSpec.vars`); its power goes into the cable network on its tile like a generator's (`power.md`) | keep. Heat leaves the whole hot loop and enters the whole cold loop; Luna changes the inputs' 200 L, which then mixes into the loop. |
| Power units | the electrical energy made in a cycle, divided by the cycle's 2 s, in W | **changed**: Luna adds the per-cycle energy number as power |
| Lights | `teg-op1` … `teg-op11` overlay | keep |
| Window | examining it shows output and both loops | **simplified**: no window yet |

## Not yet

The supermatter, its lasers and their control computer, freezers and heat exchangers. A
fire in the core chamber (`fire.md`) already heats the thermal plates.
