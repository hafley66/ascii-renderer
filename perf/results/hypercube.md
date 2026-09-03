# knob sweep: hypercube 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 608 | 607.9 | 1.64 | 1.59 | 1.79 | 22.60 | 1.00x |
| SPEED=3 | 616 | 616.0 | 1.62 | 1.59 | 1.79 | 9.35 | 0.99x |
| GHOSTS=5 | 622 | 621.3 | 1.61 | 1.59 | 1.73 | 1.85 | 0.98x |
| COPIES=5 | 625 | 624.0 | 1.60 | 1.59 | 1.78 | 1.93 | 0.97x |

worst: SPEED=3

## hotspots at SPEED=3: 619 frames, 618.8 fps

no measure_layer timers fired for hypercube; wrap its painters in crate::_0_profile::measure_layer
