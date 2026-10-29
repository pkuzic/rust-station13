# Art direction

## Reference look

NSV Luna / tg / Bay circa 2012–2013: flat, restrained 32×32 top-down sprites,
industrial 1950s–1990s retrofuturism (CRT monitors, physical buttons, beige offices, heavy
machinery, pipes, corporate bureaucracy).

## Sprites

- **The starting asset base is Luna's own art** (`icons/*.dmi`, about 330 files),
  imported into `client-assets/` under CC-BY-SA 3.0 with attribution. See
  `docs/adr/0003-licensing.md`.
- New sprites match that style: 32×32 grid, 4 directions (some objects 1 or 8),
  overlays for worn and held items, the same restrained palette.
- No 3D models and no 3D-to-sprite pipeline. All art is 2D.
- `.dmi` files are PNGs with direction and state metadata in a `zTXt` chunk. A
  converter in `tools/` turns them into our atlas format plus metadata; we never ship
  `.dmi` files directly.

## UI

- Windows 98/2000/XP-inspired panels: bevelled controls, tabs, dialogs, compact
  information-dense layouts.
- Text must fit both Russian and English. Russian strings run about 20–30% longer;
  layouts must not assume English widths.

## Modernise without changing the look

- Allowed: crisp integer scaling, better lighting and darkness, smoother animation of
  existing frames, readable fonts, high-DPI support.
- Not allowed: neon, holograms, bloom-heavy effects, redrawn "HD" pixel art, sleek
  minimalist sci-fi.
