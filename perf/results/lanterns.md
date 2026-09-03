# knob sweep: lanterns 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 255 | 254.8 | 3.92 | 3.90 | 4.24 | 4.39 | 1.00x |
| RISE=3 | 254 | 254.0 | 3.94 | 3.90 | 4.37 | 4.52 | 1.00x |
| COUNT=24 | 256 | 255.2 | 3.92 | 3.90 | 4.22 | 4.35 | 1.00x |
| SWAY=3 | 256 | 255.3 | 3.92 | 3.89 | 4.23 | 4.30 | 1.00x |

worst: RISE=3

## hotspots at RISE=3: 256 frames, 255.2 fps

no measure_layer timers fired for lanterns; wrap its painters in crate::_0_profile::measure_layer
