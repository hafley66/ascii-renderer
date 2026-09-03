# fable-2-forest

Subject: a forest composed from the fable-2 species (see `briefs/fable-2-trees.md`) in up to five depth layers over a rolling ground line, under a graded sky with stars and a moon, with one seed-chosen atmosphere drifting through it.

What moves and why: the scene (sky and ground planes, grown tree sprites, tufts, particle lists) is built once per key and cached; a frame is a table lookup per cell. Wind is the sum of two sines (22 s and 9.5 s) with a phase that travels across the screen, so trees lean in a wave rather than in lockstep; front layers sway `0.3 + 0.7 * layer` times more than the back. A 48 second light cycle lightens then darkens every interned color by LIGHT. The moon crosses the sky in 90 seconds. Atmosphere kinds: mist (a drifting band of `░ ▒` from three sines, over the ground line), fireflies (blinking at 0.12 to 0.37 Hz, drifting on slow sines), falling leaves (0.6 to 1.4 rows per second, tumbling through `, · ∙ "`), rain (`| :` at 8 to 14 rows per second), snow (`* · .` at 0.8 to 1.6 rows per second), birds (V flocks of `v ~ -` crossing in 45 seconds). At `t == 0` the frame is the static composition.

## Composition rules

- Horizon at `GROUND * height`; ground line `gl[x]` is the horizon plus two sines (amplitudes 2 and 1 percent of height, seeded phases). Sky rows use 48 lightness bands of the theme background hue; ground rows use 24 bands darkening with depth, textured at 14 percent (40 percent on the two rows under the line) with `· , ~ " ∙`.
- Layer `li` of LAYERS: `lfrac = li / (LAYERS - 1)`, tree height `height * ENERGY * (0.3 + 0.48 lfrac) * 0.72..1.0`, plot width `1.2..1.8 * height`, root row `gl[x] + 0.7 * lfrac * (height - horizon)`. Colors: hue from the theme primary plus HUE plus a per-tree jitter of 40 degrees, saturation `0.3 + 0.35 lfrac`, lightness `0.18 + 0.22 lfrac`, then faded toward the sky haze color by `HAZE * (1 - lfrac) * 0.7`.
- Packing: per layer, `width / (1.5 * tree_h) * DENSITY * 0.75` slots; a candidate center is rejected when its plot overlaps an accepted plot by more than `0.05 + 0.4 * (DENSITY - 0.2) / 1.8` of the narrower plot; twelve attempts per slot. Layers paint back to front, each sorted by root row.
- Species mix: the seed picks a dominant species (50 percent) and a secondary (28 percent); the rest share 22 percent.
- Occlusion: a sprite row with at least three cells and 30 percent fill blanks the cells between its first and last glyph before painting, so a front canopy hides the branches behind it while a lone trunk does not.
- Undergrowth tufts (`, " ; w ∙`) on the back line at 18 percent of columns and on the front root line at 30 percent, cycling glyph on a slow sine.

## Glyph families

1. Trees: as in fable-2-trees (`│ ┃ ╱ ╲ ─`, `● • ∙ · ◆ ◇`, `○ ◦ ╷`)
2. Sky: `·` stars in two brightnesses, moon `▓ ▒ ░`
3. Ground: `╌` line, `· , ~ " ∙` texture, tufts `, " ; w ∙`
4. Atmosphere: mist `░ ▒`, fireflies `· • ✦`, leaves `, · ∙ "`, rain `| :`, snow `* · .`, birds `v ~ -`

## Knobs

- DENSITY: trees per layer scale and allowed overlap (default 1.0)
- LAYERS: depth layers, 1 to 5 (default 3)
- SWAY: sway amplitude (default 0.6)
- SPEED: clock multiplier for wind, light, moon and atmosphere (default 1.0)
- HUE: hue rotation in degrees for trees and sky (default 0)
- ATMOS: 0 seed picks, 1 mist, 2 fireflies, 3 leaves, 4 rain, 5 snow, 6 birds (default 0)
- HAZE: aerial perspective strength on back layers (default 0.5)
- MOON: moon radius scale, 0 hides it (default 1.0)
- ENERGY: tree height scale (default 0.9)
- GROUND: horizon as a fraction of height (default 0.64)
- FRUIT: fruit chance per tip (default 0.15)
- LIGHT: depth of the 48 second light cycle (default 0.5)

Positional order: density layers sway speed hue atmos haze moon energy ground fruit light.

## Render commands

```bash
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 7 fable-2-forest moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=12 ./target/release/ascii-renderer 7 fable-2-forest moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=30 ./target/release/ascii-renderer 3 fable-2-forest moss 1 4 0.8 1 0 2 | sed 's/\x1b\[[0-9;]*m//g'
```

## Perf receipt

`perf/knob_sweep.sh fable-2-forest 2000 1000 1`, theme moss:

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 137 | 137.0 | 7.30 | 5.73 | 18.33 | 23.51 | 1.00x |
| DENSITY=2 | 101 | 100.1 | 9.99 | 8.33 | 22.95 | 24.92 | 1.37x |
| LAYERS=5 | 183 | 182.8 | 5.47 | 5.38 | 7.59 | 8.16 | 0.75x |
| SWAY=2 | 208 | 207.2 | 4.83 | 4.79 | 5.34 | 7.37 | 0.66x |
| SPEED=3 | 210 | 209.3 | 4.78 | 4.77 | 5.15 | 5.38 | 0.65x |
| MOON=2 | 216 | 216.0 | 4.63 | 4.58 | 5.11 | 5.20 | 0.63x |
| HUE=360 | 222 | 221.6 | 4.51 | 4.49 | 5.08 | 5.28 | 0.62x |
| GROUND=0.9 | 224 | 223.4 | 4.48 | 4.41 | 5.02 | 5.11 | 0.61x |
| ATMOS=6 | 229 | 228.3 | 4.38 | 4.31 | 5.10 | 5.38 | 0.60x |
| LIGHT=1 | 229 | 228.5 | 4.38 | 4.30 | 4.96 | 5.14 | 0.60x |
| FRUIT=1 | 231 | 230.5 | 4.34 | 4.28 | 4.82 | 5.15 | 0.59x |
| HAZE=1 | 231 | 230.5 | 4.34 | 4.29 | 4.82 | 4.94 | 0.59x |
| ENERGY=1.2 | 257 | 256.6 | 3.90 | 3.87 | 4.36 | 4.53 | 0.53x |

Hotspots at DENSITY=2, 134 fps:

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| atmos | 1.0 | 2424.8 | 5501.3 | 32.6% |
| trees | 1.0 | 1826.2 | 4305.8 | 24.5% |
| backdrop | 1.0 | 1754.3 | 2015.2 | 23.6% |
| sky | 1.0 | 473.4 | 569.3 | 6.4% |
| undergrowth | 1.0 | 11.1 | 28.1 | 0.1% |
| light | 1.0 | 1.4 | 5.5 | 0.0% |

The build layer only fires on a cache miss (first frame of a key). Seed 42 picks mist, which is the priciest atmosphere at this size: a 140 row band evaluating three sines per cell.
