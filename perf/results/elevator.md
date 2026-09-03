# knob sweep: elevator 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 331 | 330.2 | 3.03 | 3.00 | 3.27 | 4.23 | 1.00x |
| LIFTS=6 | 330 | 329.3 | 3.04 | 3.01 | 3.26 | 3.70 | 1.00x |
| SPEED=3 | 333 | 332.4 | 3.01 | 2.99 | 3.27 | 3.45 | 0.99x |
| CROWD=3 | 334 | 333.9 | 3.00 | 2.96 | 3.21 | 7.02 | 0.99x |

worst: LIFTS=6

## hotspots at LIFTS=6: 331 frames, 330.7 fps

no measure_layer timers fired for elevator; wrap its painters in crate::_0_profile::measure_layer
