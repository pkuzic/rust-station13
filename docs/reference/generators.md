# Reference spec: portable generators

## What Luna does

Source: `luna:code/modules/power/port_gen.dm` (`port_gen`, `port_gen/pacman`).

The P.A.C.M.A.N. is a dense, movable generator that burns plasma sheets.

- **Start:** unsecured (`anchored = 0`) and empty. Luna's engine monitoring room has one on a cable knot, next to a stack of 50 sheets.
- **Fuel:**
  - Plasma sheets used on it go in, up to 50 (the matter bin's rating squared × 50, rating 1). "The generator is full!" when nothing fits.
  - It burns one sheet every 100 cycles at output 1, starting the next sheet when one runs out.
- **Running:** each cycle, a generator that is on, has fuel, is secured and sits on a powernet adds `5 000 × output` to the network. Otherwise it switches itself off.
- **Tools, only while off:**
  - A wrench secures or unsecures it, connecting it to the cable on its tile.
  - A screwdriver opens the panel.
  - A crowbar with the panel closed takes it apart into a machine frame.
- **Panel (a browser window, secured generators only):** on/off, eject the fuel, output 1–4 (more when emagged), and readouts of fuel, current sheet, surplus and heat.
- **Heat:** rises with output. Above 250 it may explode, but at outputs 1–4 heat stays at or below 116, so only an emagged generator can get there.
- **Examine:** on or off; the sheets left and the output per cycle.
- **Sprites:** `portgen0` off, `portgen1` running.

## What we build (`game/classic/src/generators.rs`)

| Item | Ours | From Luna |
|---|---|---|
| Start | unsecured, empty, dense | keep |
| Plasma sheets | stack of 50; used on the generator, as many as fit go in | keep |
| Burn rate | one sheet per 100 machine cycles (200 s) | keep (Luna's cycle is about 2.4 s) |
| Output | 5 kW into the network on its tile, through `PowerSupply` | output setting fixed at 1 |
| What isn't used | lost; SMES units only pay for what the generators can't cover | keep |
| Wrench while off | secures or unsecures it; secured, it joins the cable network on its tile | keep |
| Empty hand on a secured generator | switches it on (if it has fuel) or off | **simplified**: Luna's panel window |
| Examine | on or off, sheets left, output | keep |
| Ejecting fuel, output setting, heat, overheating | none | need the panel window; heat can't reach 250 without an emag |
| Screwdriver panel, crowbar deconstruction | none | no machine frames yet |
| S.U.P.E.R. and M.R.S. variants (uranium, diamond) | none | not on Luna's engineering map |
