# fable-1-forest

Subject: a night forest of the fable-1 species (space colonization, banyan, mangrove, baobab, coral) packed in three depth layers under a slow sky, with a hill ridge, textured ground, a moon, and one seeded atmosphere.

What moves and why: the scene is built once per (size, seed, geometry knobs) and cached; every tree becomes a stamp (cells relative to its root, colors as indices). Per frame: the light cycles over 60 s / SPEED (`0.5 - 0.5*cos`), blending the sky from night to dusk colors, fading stars, lifting ground and tree colors; the wind is a shared 30..44 s sine with a 0.35 overtone and a per-tree phase offset, leaning each stamp by `SWAY * amp * wind * (row/height)^1.6` cells; the moon drifts a fifth of the width per light cycle; the atmosphere drifts on its own clock. All motion is gated on `t > 0`.

## Composition

- Sky: rows above the horizon (HORIZON * height) get a top-to-horizon background gradient; stars are seeded points (`·`, `+`, `✦`) that twinkle and fade with the light; the moon is an ellipse of `▓` with a `▒` rim.
- Hills: a per-column ridge from two summed sines fills a darker band up to the horizon, with a `░` ridge line and sparse `·`.
- Ground: rows below the horizon get a far-to-near gradient and hashed `· ∙ ~` litter whose density rises toward the viewer.
- Trees: layer `li` of LAYERS has depth `lf = li / (LAYERS - 1)`; base height `h * (0.14 + 0.42*lf)`, width 0.85..1.35 of height, root rows `horizon + ground_depth * 0.8 * lf` with jitter, species from a seeded weight mix (coral weighted at 0.4), leaf density rising with `lf`, and colors blended toward the fog color by `(1 - lf) * FOG`. Placement walks x with step `1.1 + 0.6*lf` widths divided by DENSITY, 0.7..1.3 jitter, and a 22 percent chance of an extra clearing; each layer is painted back to front and the stamps overlap.
- Atmosphere (ATMOS, 0 picks by seed): 1 fireflies on slow Lissajous loops with 1.5..4 s blinks (`•` bright, `·` dim); 2 falling leaves (`∙ ◦ ~ ·`) sinking 0.9..1.8 rows/s with a sideways sine; 3 mist bands per layer, a separable sine field per column times a row envelope plus hash dither, `▒` above 0.8 and `░` above 0.6, drifting on 40 s and 27 s clocks; 4 rain, slanted `│ ╎ ·` streaks at 6..10 rows/s; 5 snow, `· • ∙` sinking 0.5..1.2 rows/s.

Seed changes: species weights, density multiplier 0.8..1.2, hue jitter +-20 degrees, hill profile, star field, moon position, atmosphere choice, wind period, and every tree.

## Glyph families

1. Trees: everything in `briefs/fable-1-trees.md`
2. Sky: `· + ✦ ▓ ▒`
3. Hills and ground: `░ · ∙ ~`
4. Mist and snow: `▒ ░ · •`
5. Fireflies, leaves, rain: `• · ∙ ◦ ~ │ ╎`

## Knobs

- DENSITY: trees per layer (default 1)
- LAYERS: depth layers (default 3)
- SWAY: sway amplitude (default 1)
- SPEED: clock multiplier for wind, light, moon, atmosphere (default 1)
- HUE: palette hue shift in degrees (default 0)
- ATMOS: 0 seeded, 1 fireflies, 2 leaves, 3 mist, 4 rain, 5 snow (default 0)
- FOG: depth fade strength (default 0.6)
- HORIZON: horizon row fraction (default 0.6)
- MOON: draw the moon, 0 or 1 (default 1)
- FRUIT: fruit probability (default 0.2)

Positional order: density layers sway speed hue atmos fog horizon moon fruit.

## Render commands

```bash
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 7 fable-1-forest moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=12 ./target/release/ascii-renderer 7 fable-1-forest moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=30 ASCII_P_ATMOS=3 ./target/release/ascii-renderer 11 fable-1-forest moss | sed 's/\x1b\[[0-9;]*m//g'
```

Measured frame time, release, 200x60, 200 frames: avg 0.017 ms, worst 0.023 ms.

## Perf receipt

`perf/knob_sweep.sh fable-1-forest 2000 1000 2`, release, Apple Silicon. Every knob at max stays above 100 fps; the layer timers cover 90 percent of the frame.

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 229 | 114.4 | 8.74 | 7.59 | 14.88 | 15.74 | 1.00x |
| LAYERS=5 | 208 | 103.8 | 9.63 | 9.55 | 11.10 | 12.29 | 1.10x |
| SPEED=3 | 225 | 112.2 | 8.92 | 7.41 | 21.00 | 35.36 | 1.02x |
| HUE=360 | 225 | 112.5 | 8.89 | 7.41 | 20.47 | 63.16 | 1.02x |
| DENSITY=2 | 230 | 114.6 | 8.73 | 7.86 | 15.06 | 23.69 | 1.00x |
| SWAY=3 | 240 | 119.8 | 8.35 | 7.52 | 13.39 | 14.91 | 0.95x |
| FRUIT=1 | 265 | 132.5 | 7.55 | 7.22 | 13.17 | 45.16 | 0.86x |
| FOG=1 | 278 | 138.6 | 7.21 | 7.23 | 7.68 | 8.34 | 0.83x |
| MOON=1 | 280 | 139.7 | 7.16 | 7.15 | 7.56 | 7.68 | 0.82x |
| ATMOS=5 | 385 | 192.3 | 5.20 | 4.73 | 10.52 | 12.33 | 0.59x |
| HORIZON=0.85 | 687 | 343.2 | 2.91 | 2.86 | 3.29 | 3.32 | 0.33x |

worst: LAYERS=5

## hotspots at LAYERS=5: 211 frames, 105.4 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| mist | 5.0 | 978.2 | 1999.0 | 51.6% |
| ground | 1.0 | 2475.1 | 3223.6 | 26.1% |
| sky | 1.0 | 542.6 | 1595.3 | 5.7% |
| trees | 5.0 | 76.9 | 379.7 | 4.1% |
| hills | 1.0 | 200.9 | 751.9 | 2.1% |
| atmos | 1.0 | 0.1 | 4.5 | 0.0% |
