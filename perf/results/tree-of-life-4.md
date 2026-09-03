# knob sweep: tree-of-life-4 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 68 | 67.3 | 14.85 | 14.73 | 15.57 | 17.12 | 1.00x |
| DEPTH=11 | 67 | 66.5 | 15.05 | 14.92 | 15.96 | 16.02 | 1.01x |
| SPIN=1 | 68 | 67.1 | 14.90 | 14.72 | 15.89 | 18.46 | 1.00x |
| TILE=1 | 68 | 67.3 | 14.86 | 14.74 | 16.06 | 16.11 | 1.00x |
| SPREAD=1.3 | 68 | 67.4 | 14.85 | 14.78 | 15.56 | 15.59 | 1.00x |
| GLOW=1 | 68 | 67.6 | 14.79 | 14.72 | 15.63 | 15.69 | 1.00x |
| MOTES=400 | 68 | 67.7 | 14.78 | 14.73 | 15.31 | 15.39 | 1.00x |
| LEN=2.5 | 68 | 67.9 | 14.73 | 14.67 | 15.32 | 15.35 | 0.99x |
| SEAM=0.5 | 69 | 68.0 | 14.70 | 14.64 | 15.27 | 15.36 | 0.99x |
| DRIFT=0.85 | 69 | 68.0 | 14.70 | 14.63 | 15.47 | 15.63 | 0.99x |
| SPEED=4 | 69 | 68.1 | 14.69 | 14.62 | 15.26 | 15.37 | 0.99x |

worst: DEPTH=11

## hotspots at DEPTH=11: 67 frames, 66.8 fps

no measure_layer timers fired for tree-of-life-4; wrap its painters in crate::_0_profile::measure_layer
