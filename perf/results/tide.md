# knob sweep: tide 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 22 | 21.2 | 47.13 | 46.76 | 48.61 | 51.67 | 1.00x |
| WAVES=4 | 16 | 15.1 | 66.40 | 66.28 | 67.64 | 67.81 | 1.41x |
| AMP=2.5 | 22 | 21.1 | 47.36 | 47.40 | 48.32 | 48.35 | 1.00x |
| SPEED=3 | 22 | 21.1 | 47.29 | 47.24 | 48.78 | 48.82 | 1.00x |

worst: WAVES=4

## hotspots at WAVES=4: 15 frames, 15.0 fps

no measure_layer timers fired for tide; wrap its painters in crate::_0_profile::measure_layer
