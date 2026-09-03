# knob sweep: tree-of-life-5 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 118 | 117.4 | 8.51 | 8.46 | 9.10 | 9.35 | 1.00x |
| DEPTH=10 | 115 | 114.4 | 8.74 | 8.55 | 9.53 | 14.58 | 1.03x |
| SEAM=0.5 | 117 | 116.0 | 8.62 | 8.58 | 9.27 | 9.29 | 1.01x |
| WIND=1 | 117 | 116.5 | 8.59 | 8.46 | 9.42 | 10.66 | 1.01x |
| SPEED=4 | 117 | 116.9 | 8.55 | 8.54 | 8.88 | 9.13 | 1.00x |
| SPIN=1 | 117 | 117.0 | 8.55 | 8.49 | 8.96 | 9.22 | 1.00x |
| RINGS=1 | 118 | 117.4 | 8.52 | 8.46 | 9.05 | 9.20 | 1.00x |
| GLOW=1 | 118 | 117.5 | 8.51 | 8.46 | 8.97 | 9.15 | 1.00x |
| MOTES=300 | 118 | 117.6 | 8.51 | 8.48 | 8.89 | 9.14 | 1.00x |
| SPREAD=1.3 | 118 | 117.6 | 8.51 | 8.46 | 9.01 | 9.34 | 1.00x |
| LEN=2 | 118 | 117.9 | 8.48 | 8.45 | 8.96 | 9.13 | 1.00x |

worst: DEPTH=10

## hotspots at DEPTH=10: 117 frames, 116.5 fps

no measure_layer timers fired for tree-of-life-5; wrap its painters in crate::_0_profile::measure_layer
