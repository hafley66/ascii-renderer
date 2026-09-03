# knob sweep: tree-of-life-6 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 36 | 35.6 | 28.09 | 27.92 | 29.35 | 29.84 | 1.00x |
| WARP=1 | 36 | 35.3 | 28.32 | 28.02 | 29.88 | 30.03 | 1.01x |
| SPEED=4 | 36 | 35.5 | 28.13 | 27.95 | 28.90 | 30.19 | 1.00x |
| SEAM=0.6 | 36 | 35.6 | 28.12 | 27.86 | 29.77 | 29.95 | 1.00x |
| FLOW=1.5 | 36 | 35.6 | 28.06 | 27.92 | 28.93 | 29.19 | 1.00x |
| MOTES=300 | 36 | 35.6 | 28.06 | 27.94 | 28.84 | 28.92 | 1.00x |
| SPREAD=1.4 | 36 | 35.7 | 28.04 | 27.96 | 28.73 | 28.93 | 1.00x |
| DEPTH=10 | 36 | 35.7 | 28.03 | 27.93 | 28.74 | 29.12 | 1.00x |
| LATTICE=1 | 36 | 35.7 | 28.01 | 27.92 | 28.52 | 29.05 | 1.00x |
| ZOOM=2 | 36 | 35.7 | 28.00 | 27.89 | 28.75 | 29.20 | 1.00x |
| GLOW=1 | 36 | 35.7 | 27.99 | 27.94 | 28.49 | 28.64 | 1.00x |

worst: WARP=1

## hotspots at WARP=1: 36 frames, 35.5 fps

no measure_layer timers fired for tree-of-life-6; wrap its painters in crate::_0_profile::measure_layer
