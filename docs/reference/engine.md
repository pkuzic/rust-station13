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

## Supermatter (Luna `engine/supermatter`)

- **Gas:** each cycle it takes a quarter of its tile's air (`gasefficency`).
- **Damage:** it builds up `(T − 1100) / 150` a cycle, never below zero. While it is damaged and the gas is above 1 100 K, the core control computer warns on the radio at most once a minute: "CORE OVERLOAD" (rising), "Core returning to safe operating levels." (falling), "CORE EXPLOSION IMMINENT" (within 300 of 3 500). At 3 500 it explodes.
- **Power:** `round((T − 0 °C) / 20)`, plus twice the power of each laser beam on the four tiles beside it.
- **Oxygen factor:** `(O₂ − 4 × N₂) / (one tile of standard air)`, from 0 to 1. Energy is `round(oxygen × power × T / 0 °C × 1.4)`.
- **Effects:** the gas heats by energy / 50, up to 1 500 K. It gains `round((energy + T − 0 °C) / 1500)` mol of oxygen, then mixes back. Everyone within three tiles is gibbed; within six, people without meson scanners may hallucinate.

## Lasers and the laser control computer (Luna `engine/laser`, `computer/lasercon`)

- **Lasers:** each has an id, a power (500 to start, 1–3 000) and starts off. On, it fires a beam tile by tile in its direction until something dense, a wall or a person stops it. A person in the way takes power / 400 burns on every body part each cycle. The beam glows blue.
- **Computer:** it controls the lasers whose ids it lists. Its window shows the first one's power with ±1, ±10, ±100 and ±1000 buttons, which apply to all of them, and an Online/Offline switch. It draws 250 W.

## What we build (`game/classic/src/teg.rs`, `supermatter.rs`, `machines.rs`)

| Item | Ours | From Luna |
|---|---|---|
| Generator inputs, thermal plates | unary pipe devices of 200 L, dense, facing the map's way | keep |
| Thermal plate | as above, every machine cycle | keep |
| TEG | as above, every machine cycle, with Luna's map settings (`ObjectSpec.vars`); its power goes into the cable network on its tile like a generator's (`power.md`) | keep. Heat leaves the whole hot loop and enters the whole cold loop; Luna changes the inputs' 200 L, which then mixes into the loop. |
| Power units | the electrical energy made in a cycle, divided by the cycle's 2 s, in W | **changed**: Luna adds the per-cycle energy number as power |
| Lights | `teg-op1` … `teg-op11` overlay | keep |
| Window | examining it shows output and both loops | **simplified**: no window yet |
| Supermatter | as above, every machine cycle: gas, damage, power from warmth and beams, the oxygen factor, heating, oxygen. It kills everyone within three tiles by destroying every body part | keep. No explosion yet (there are no explosions), so the damage only drives the warnings. No hallucinations. |
| Core warnings | on the common channel, to everyone near a listening radio, at most once a minute | keep |
| Lasers and beams | as above, every machine cycle; beam tiles are drawn, glow and can't be clicked | keep |
| Laser control computer | as above, with its window; ids from the map (`id = list(...)`) | keep. Luna's "advanced" frequency setting is left out. |

## Not yet

Freezers and heat exchangers for the cold loop, the explosion, hallucinations, and filling
the core with oxygen (Luna feeds it through the region's injectors from tanks outside it).
