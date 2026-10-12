# Nightveil polar-night aurora

Mode: `nightveil`, owned by `src/modes/_65_nightveil.rs`. Generated registry, `AnimKind::Iterate`.

Six knobs: CURTAINS (1-8), DRIFT (0-3), SHIMMER (0-1), WAVES (0-3), STARS (0-1), MOTES (0-2).
Five layers: sky, stars, curtains, ridge, motes. Sky, stars, and curtains run row-parallel
rayon passes above 20480 cells. Curtains are hash-driven gaussian bands with per-curtain
sway, vertical falloff, and shimmer; all phases come from splitmix of (seed, layer, index),
never the rng stream, so time 0 is byte-identical to the static render.

## Layer coverage

```
# layer coverage: 400x120, theme moss, seed 42, dt 0.06, 3 reps, release

| mode | layers | calls/frame | attributed | nested | thin |
| --- | ---: | ---: | ---: | --- | --- |
| nightveil | 5 | 5.0 | 99.4% | no | no |
```

## Knob sweep (2000x1000)

Pre-rayon baseline was 176.7 ms avg (5.7 fps); rayon row passes cut it to the table below.

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 86 | 43.0 | 23.26 | 23.20 | 24.14 | 24.26 | 1.00x |
| CURTAINS=8 | 48 | 23.8 | 41.99 | 41.87 | 43.59 | 43.96 | 1.81x |
| WAVES=3 | 86 | 42.7 | 23.39 | 23.21 | 25.34 | 26.39 | 1.01x |
| STARS=1 | 86 | 42.8 | 23.36 | 23.30 | 24.74 | 25.87 | 1.00x |
| SHIMMER=1 | 86 | 42.8 | 23.36 | 23.25 | 25.39 | 25.67 | 1.00x |
| MOTES=2 | 86 | 42.9 | 23.28 | 23.17 | 24.91 | 27.76 | 1.00x |
| DRIFT=3 | 87 | 43.3 | 23.12 | 23.06 | 23.79 | 24.21 | 0.99x |

Worst knob: CURTAINS=8 at 42 ms avg, still ~24 fps at 2000x1000. Curtains dominate
(O(w*h) per curtain pass); hotspot breakdown in perf/results/nightveil.md.
