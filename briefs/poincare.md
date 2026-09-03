# poincare

Subject: a regular hyperbolic tessellation {p,q} drawn in the Poincare disk or the upper half-plane, the whole plane sliding under a slow Mobius flow so tiles stream through the focus, brighten, and shrink away into the rim.

What moves and why: the flow is `R(theta) . T_perp(sigma) . T_axis(s)`. `s = SPEED * t` is a hyperbolic translation along a mirror axis of the tiling, wrapped modulo the tiling's own translation period along that axis (`4 om` for even p, `4 (om + ov)` for odd p and even q, `2 (om + ov + mv)` for both odd), so the slide never jumps and never repeats against the twist. `sigma = SWAY * sin(2 pi t / 37..48 s)` is a cross translation with a seed-picked period. `theta = TWIST * t` spins the whole disk. Each cell is mapped through the inverse flow, then reflected into the fundamental triangle (real axis, the `pi/p` spoke, the edge geodesic) with a DEPTH cap; the reflection word is replayed on the origin to find the tile center, which sets the tile's brightness by hyperbolic distance from the focus (FADE) and its breath phase (PULSE). Three to eight geodesics fixed in the tiling frame glow as they pass the focus (ARCS, GLOW). Local scale drives level of detail: big tiles get a full rosette fill, mid tiles keep only their edge mesh and a center dot, tiles under HAZE rows dissolve into the rim haze.

Glyph families:
1. Edge mesh, drawn by view-frame tangent: `-`, `/`, `|`, `\`, solid `#`, `%` when wider than THICK cells, `.` when dim
2. Ink tiles (family A) fill ramp: ` .:-=+*#%@`
3. Bloom tiles (family B) fill ramp: ` ·:o•O●◆`
4. Vertex stars and far-tile dots: `*`, `+`, `o`, `•`
5. Rim haze and horizon: `:`, `.`, `░`, `▒`, `=`, `-`; glowing geodesics: `~`

Seed picks: tessellation from {7,3}, {5,4}, {3,7}, {4,5}, {6,4}; disk or half-plane; slide direction; start angle; sway phase; palette hue rotation; post positions of the geodesic arcs. Even q gives a strict polygon checkerboard of the two families; odd q assigns families per tile from the tile hash.

Knobs:
- P, Q: tessellation, 0 lets the seed pick (defaults 0, 0)
- DEPTH: reflection cap per cell (default 24)
- SPEED: slide in hyperbolic units per second (default 0.05, about 35 s per tile)
- TWIST: spin in degrees per second (default 2)
- HUE: palette hue shift in degrees (default 0)
- MODEL: 0 seed, 1 disk, 2 half-plane (default 0)

Added while building:
- EDGE: edge band width in hyperbolic units (default 0.07)
- HAZE: tile rows under which a cell is rim haze (default 2.5)
- ARCS: glowing geodesics (default 3)
- GLOW: geodesic glow width (default 0.25)
- PETALS: angular frequency of the rosette (default 1)
- FADE: tile brightness falloff distance (default 3)
- ASPECT: cell columns per row (default 2)
- THREADS: worker threads, 0 auto (default 0)
- SWAY: cross translation amplitude (default 0.4)
- LABEL: show {p,q} (default 1)
- SPAN: half-plane half width (default 2.6)
- STAR: vertex star radius (default 0.16)
- PULSE: tile breath depth (default 0.15)
- LINE: minimum edge width in cells (default 0.8)
- DETAIL: inscribed tile rows for a full fill (default 10)
- DOTS: far tile center dot radius (default 0.2)
- FOCUS: half-plane focus height as a grid fraction (default 0.55)
- THICK: edge width in cells before it goes solid (default 1.8)
- RINGS: rings per tile in the rosette (default 1.5)
- DITHER: fill grain (default 0.08)

Positional order: p q depth speed twist hue model edge haze arcs glow petals fade aspect threads sway label span star pulse line detail dots focus thick rings dither.

Render commands:
```bash
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 7 poincare moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=12 ./target/release/ascii-renderer 7 poincare moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 9 poincare moss 0 0 24 0.05 2 0 1   # {7,3} forced to the disk
```

Measured frame time, release, 200x60, 200 frames: avg 0.223 ms, worst 0.406 ms (before the tile memo; lower now).

Perf receipt, `perf/knob_sweep.sh poincare 2000 1000 1`, Apple M2 Pro, 12 threads:

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 52 | 51.8 | 19.29 | 19.40 | 20.28 | 20.53 | 1.00x |
| SWAY=2 | 48 | 47.6 | 21.02 | 20.88 | 22.77 | 23.09 | 1.09x |
| SPEED=0.5 | 49 | 48.1 | 20.78 | 20.57 | 23.50 | 24.06 | 1.08x |
| PETALS=4 | 49 | 48.8 | 20.48 | 20.16 | 21.86 | 35.58 | 1.06x |
| RINGS=6 | 50 | 49.0 | 20.40 | 20.06 | 23.50 | 27.40 | 1.06x |
| ARCS=8 | 50 | 49.9 | 20.03 | 19.91 | 22.94 | 24.23 | 1.04x |
| DEPTH=64 | 51 | 50.2 | 19.93 | 19.73 | 21.93 | 22.20 | 1.03x |
| DOTS=1 | 51 | 50.5 | 19.82 | 19.26 | 22.43 | 39.11 | 1.03x |
| FADE=8 | 51 | 50.6 | 19.77 | 19.57 | 23.02 | 23.56 | 1.02x |
| GLOW=1 | 51 | 50.8 | 19.70 | 19.71 | 21.44 | 22.36 | 1.02x |
| DETAIL=30 | 52 | 51.1 | 19.56 | 19.26 | 22.65 | 24.28 | 1.01x |
| THREADS=16 | 52 | 51.2 | 19.55 | 19.46 | 21.79 | 21.89 | 1.01x |
| FOCUS=0.95 | 52 | 51.2 | 19.52 | 19.27 | 21.72 | 22.14 | 1.01x |
| STAR=0.5 | 52 | 51.4 | 19.47 | 19.32 | 21.67 | 21.69 | 1.01x |
| DITHER=0.5 | 52 | 51.4 | 19.45 | 19.44 | 21.17 | 21.61 | 1.01x |
| SPAN=8 | 52 | 51.5 | 19.42 | 19.31 | 22.01 | 23.38 | 1.01x |
| THICK=6 | 52 | 51.5 | 19.41 | 19.40 | 20.80 | 21.63 | 1.01x |
| LABEL=1 | 52 | 51.6 | 19.36 | 19.32 | 21.27 | 21.47 | 1.00x |
| PULSE=0.5 | 52 | 51.7 | 19.33 | 19.34 | 21.04 | 21.97 | 1.00x |
| HUE=360 | 52 | 51.8 | 19.31 | 19.31 | 20.52 | 20.66 | 1.00x |
| TWIST=12 | 53 | 52.1 | 19.21 | 19.09 | 20.60 | 20.98 | 1.00x |
| EDGE=0.3 | 53 | 52.6 | 19.01 | 18.98 | 20.19 | 20.49 | 0.99x |
| LINE=2 | 55 | 54.3 | 18.41 | 18.36 | 19.84 | 19.91 | 0.95x |
| HAZE=8 | 55 | 54.5 | 18.36 | 18.18 | 19.95 | 21.22 | 0.95x |
| MODEL=2 | 58 | 57.1 | 17.51 | 17.32 | 20.26 | 20.42 | 0.91x |
| Q=12 | 64 | 63.7 | 15.70 | 15.67 | 17.61 | 18.02 | 0.81x |
| P=12 | 81 | 80.6 | 12.41 | 12.27 | 14.42 | 17.52 | 0.64x |
| ASPECT=4 | 96 | 95.0 | 10.52 | 10.48 | 11.70 | 12.39 | 0.55x |

Hotspots at SWAY=2, 47.0 fps:

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| field | 1.0 | 19695.5 | 22111.8 | 92.5% |
| clear | 1.0 | 547.6 | 865.2 | 2.6% |
| setup | 1.0 | 6.4 | 9.9 | 0.0% |
| label | 1.0 | 1.8 | 17.0 | 0.0% |
| rim | 1.0 | 0.0 | 0.2 | 0.0% |

Perf notes: the field pass runs on `std::thread::scope` workers pulling 4-row chunks from a shared queue (no new dependency; rayon would do the same job with less code but was not added). Per cell: one Mobius apply with derivative, the reflection word, and a per-thread memo that skips the tile-center replay when the word matches the previous cell. Single-thread cost is about 72 ns per cell at 2000x1000.
