# sonnet-2-clifford

Subject: a Clifford strange attractor, `x' = sin(ay) + c*cos(ax)`, `y' = sin(bx) + d*cos(by)`,
iterated fresh every frame while the four map constants breathe on slow independent sine cycles.

What moves and why: at build time the seed drives a search over (a, b, c, d) that scores each
candidate by an approximate box-counting dimension between two bin scales, favoring lacy,
structured fills over both blank periodic cycles and solid clouds. Each frame nudges a, b, c, d
away from that seeded home with `DRIFT * sin(phase * k)` at four different `k` multipliers, so the
whole fractal slowly reshapes instead of oscillating in lockstep. A short retrace (`aim` layer)
re-centers the viewport on the live attractor every frame, since a bounded map can still drift far
from its resting bbox; if a breath lands on a narrow periodic window the retrace detects the
collapse (few bins of its own bbox visited) and holds last frame's healthy constants instead of
rendering a near-blank frame. A trailing comet walks the tail of the same orbit so the generating
motion reads as a single point tracing the cloud, not just a static fractal poster.

Glyph families:
1. Attractor dust, faint to core: `\u{b7}` `\u{2219}` `\u{2022}` `\u{25cf}` `\u{2b24}`
2. Comet trail, glyph variant A (odd seeds): `\u{2219}` `\u{2022}` `\u{25c6}` (head)
3. Comet trail, glyph variant B (even seeds): `.` `*` `@` (head)
4. Frame corners and ticks: `\u{250c}` `\u{2510}` `\u{2514}` `\u{2518}` `\u{2500}` `\u{2502}`
5. Background vignette: plain space over a radial-gradient background color

Knobs:
- SPEED: overall clock multiplier for the breathing phase (default 1.0)
- HUE: additive hue rotation in degrees on top of the seed's base hue (default 0)
- SPREAD: hue swing in degrees from dense core to faint mist (default 60)
- DRIFT: absolute amplitude of the a/b/c/d breathing (default 0.5)
- PERIOD: seconds for one full breathing cycle (default 36, within the 20-60s ask)
- DENSITY: orbit points computed per frame (default 180000, max 600000)
- COMET: length of the trailing comet in points (default 220)
- GLOW: gamma on density-to-brightness, higher pulls contrast into the core (default 1.0)
- SCALE: zoom factor fitting the live attractor into the grid (default 1.0)

Positional order: speed hue spread drift period density comet glow scale.

Seed-driven choices: the (a, b, c, d) configuration and starting point (dimension-scored search),
the base hue (`seed_hue`), and the comet's glyph variant (`seed & 1`).

Perf receipt (`perf/knob_sweep.sh sonnet-2-clifford 2000 1000 2`), release, 2000x1000:

| knob at max | fps | vs baseline |
| --- | ---: | ---: |
| baseline | 153.2 | 1.00x |
| DENSITY=600000 | 75.9 | 2.02x |
| SPEED=4 | 149.8 | 1.02x |
| SCALE=2 | 152.7 | 1.00x |
| PERIOD=90 | 152.9 | 1.00x |
| GLOW=3 | 153.0 | 1.00x |
| DRIFT=1.5 | 153.1 | 1.00x |
| COMET=600 | 153.3 | 1.00x |
| HUE=360 | 153.4 | 1.00x |
| SPREAD=180 | 153.5 | 1.00x |

worst: DENSITY=600000, 75.7 fps (well above the 30 fps floor)

| layer | share of frame |
| --- | ---: |
| orbit | 71.6% |
| clear | 12.2% |
| field | 7.6% |
| aim | 0.5% |
| frame | 0.0% |
| comet | 0.0% |

Render commands:
```bash
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 42 sonnet-2-clifford moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=12 ./target/release/ascii-renderer 7 sonnet-2-clifford moss | sed 's/\x1b\[[0-9;]*m//g'
```
