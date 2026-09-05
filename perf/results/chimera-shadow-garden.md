# knob sweep: chimera-shadow-garden 2000x1000, 2s per run, dt 0.06, theme deep

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 472 | 235.7 | 4.24 | 4.14 | 5.34 | 5.90 | 1.00x |
| GROWTH=1.5 | 411 | 205.1 | 4.88 | 4.42 | 9.79 | 57.79 | 1.15x |
| CROWN=1.4 | 475 | 237.2 | 4.22 | 4.19 | 4.67 | 6.65 | 0.99x |
| SWAY=1 | 475 | 237.2 | 4.22 | 4.08 | 6.28 | 27.20 | 0.99x |
| TIDE=1 | 475 | 237.2 | 4.22 | 4.13 | 6.44 | 8.71 | 0.99x |
| SPEED=2 | 491 | 245.0 | 4.08 | 4.06 | 4.47 | 4.97 | 0.96x |
| SPORES=1 | 498 | 248.6 | 4.02 | 4.00 | 4.42 | 4.77 | 0.95x |

worst: GROWTH=1.5

## hotspots at GROWTH=1.5: 472 frames, 235.6 fps

no measure_layer timers fired for chimera-shadow-garden; wrap its painters in crate::_0_profile::measure_layer
