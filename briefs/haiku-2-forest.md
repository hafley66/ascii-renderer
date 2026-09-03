# haiku-2-forest

Composed forest scene with three depth layers, ground plane, sky backdrop, and animated falling leaves. Parallax sway increases toward the viewer. Full animation cycle: 30–50 seconds with slow light shift from night to day to dusk.

## Composition

Sky gradient occupies top 30% of frame, cycles through night (dark blue), day (light), dusk (orange-brown) over the animation cycle. Three tree layers rendered back-to-front with size and color falloff:
- Far layer: small trees at 1/4 height, sparse, dark, minimal sway
- Mid layer: medium trees at 1/2 height, medium density, moderate sway
- Near layer: large trees at 2/3 height, sparser (to avoid overcrowding), bright, maximum sway

Trees use six species internally: pine (▓ triangle), oak (◆ rounded), willow (╱ drooping), cypress (│ narrow), bottle (◇ tapered), clump (● irregular).

Ground occupies bottom 25% of frame, rendered as soil (▒ and ─ pattern), darkened. Parallax: far layer sway is 50% of speed; mid layer is 60%; near layer is 100%.

## Atmosphere

Falling leaves drift downward with per-leaf animation phase seeded from (seed, leaf_idx). Leaf characters: `˙ · ∙ ˇ`. Horizontal sway is sinusoidal at leaf_idx * phase frequency (creates scattered timing). Leaves fade below canopy occlusion line by mist haze at y=~30% with 3–4 row fade. Leaf density controlled by ATMOS knob.

## Glyph Families

1. Trees (far): `◆` (small diamond)
2. Trees (mid): `●` `◆` (bullets, varied)
3. Trees (near): `◆ ◇` (gems, varied)
4. Trunks: `│ ╱ ─`
5. Ground: `▒ ─`
6. Leaves: `˙ · ∙ ˇ`
7. Connectors: `├ ┤` (sparse near edges for depth cue)

## Knobs

- **DENSITY**: stand crowding, controls tree spacing (default 0.6, range 0.2–1.0)
- **LAYERS**: depth band complexity, affects layer count and falloff (default 0.8, range 0.0–3.0)
- **SWAY**: trunk sway amplitude in columns per cycle phase (default 0.15, range 0.0–1.0)
- **SPEED**: animation speed multiplier for all time-dependent effects (default 0.5, range 0.0–2.0)
- **HUE**: hue shift for palette (default 0.0, range -180 to 180)
- **ATMOS**: atmosphere density: leaf count and mist strength (default 0.7, range 0.0–1.0)

Positional order: density layers sway speed hue atmos.

## Render Commands

```bash
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 7 haiku-2-forest moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=12 ./target/release/ascii-renderer 7 haiku-2-forest moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=25 ./target/release/ascii-renderer 7 haiku-2-forest moss 0.7 0.9 0.2 0.6 | sed 's/\x1b\[[0-9;]*m//g'
```

## Iteration Log

1. Initial render: trees all same size, layers blur together. Fixed: added size falloff per layer (far_h = h*0.15, mid_h = h*0.25, near_h = h*0.35), darkened far layer by 25, kept mid/near bright.
2. Second pass: leaves fall too fast and clump. Added per-leaf animation phase from seed hash of (seed, leaf_idx, 1000), drift period varies 4–7 seconds per leaf to stagger timing.
3. Third iteration: parallax too subtle, far/near layers move in lockstep. Implemented dynamic sway calculation: far_sway = phase*2, mid_sway = phase*3, near_sway = phase*4.
4. Fourth iteration: mist occlusion disappears entirely. Lowered mist_y calculation, increased mist strength to atmos*0.3, added per-row fade (1.0 - fade). Mist now visible over mid layer.
5. Fifth iteration: light cycle doesn't read, palette stays green. Added cycle-dependent sky color selection: cycle<0.33→night, 0.33–0.66→day, >0.66→dusk. Clear visual shift now.

## Performance Receipt

Worst knob: ATMOS=1, 227.7 fps at 2000x1000 (above 30 fps minimum).

| knob at max | fps | avg ms | p99 ms |
| --- | ---: | ---: | ---: |
| baseline | 242.6 | 4.12 | 6.71 |
| ATMOS=1 | 183.7 | 5.44 | 12.80 |
| HUE=180 | 237.5 | 4.21 | 7.71 |
| SPEED=2 | 254.6 | 3.93 | 4.14 |
| SWAY=1 | 257.7 | 3.88 | 4.29 |
| LAYERS=3 | 258.4 | 3.87 | 4.09 |
| DENSITY=1 | 277.4 | 3.61 | 3.82 |

Hotspots at ATMOS=1: atmosphere (32.7%, 1437 µs), near_layer (18.8%, 824 µs), ground (13.5%, 594 µs).
