# knob sweep: ferris 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 261 | 260.5 | 3.84 | 3.82 | 4.15 | 4.71 | 1.00x |
| GONDOLAS=14 | 259 | 258.9 | 3.86 | 3.83 | 4.28 | 4.51 | 1.01x |
| SPEED=3 | 262 | 261.0 | 3.83 | 3.81 | 4.11 | 4.19 | 1.00x |
| RADIUS=12 | 262 | 261.4 | 3.83 | 3.80 | 4.06 | 4.09 | 1.00x |

worst: GONDOLAS=14

## hotspots at GONDOLAS=14: 259 frames, 258.7 fps

no measure_layer timers fired for ferris; wrap its painters in crate::_0_profile::measure_layer
