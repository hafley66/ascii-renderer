# knob sweep: volute 2000x1000, 2s per run, dt 0.06, theme deep

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 36 | 17.9 | 55.80 | 55.71 | 57.09 | 57.72 | 1.00x |
| SCALE=1.25 | 35 | 17.4 | 57.38 | 57.27 | 58.35 | 58.54 | 1.03x |
| TIDE=1 | 36 | 17.8 | 56.11 | 56.04 | 56.90 | 57.37 | 1.01x |
| CHAMBERS=32 | 36 | 17.8 | 56.09 | 55.99 | 57.02 | 58.16 | 1.01x |
| SPEED=2 | 36 | 17.9 | 55.90 | 55.72 | 57.39 | 58.82 | 1.00x |
| NACRE=1 | 36 | 17.9 | 55.80 | 55.74 | 57.08 | 57.53 | 1.00x |
| COIL=0.22 | 37 | 18.1 | 55.24 | 55.22 | 55.87 | 56.48 | 0.99x |

worst: SCALE=1.25

## hotspots at SCALE=1.25: 35 frames, 17.3 fps

no measure_layer timers fired for volute; wrap its painters in crate::_0_profile::measure_layer

