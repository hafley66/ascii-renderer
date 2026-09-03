# knob sweep: fireworks 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 428 | 427.5 | 2.34 | 2.31 | 2.66 | 4.51 | 1.00x |
| SPEED=3 | 434 | 433.5 | 2.31 | 2.28 | 2.58 | 3.31 | 0.99x |
| SPARKS=48 | 435 | 434.1 | 2.30 | 2.28 | 2.50 | 2.60 | 0.98x |
| BURSTS=12 | 436 | 435.5 | 2.30 | 2.28 | 2.42 | 2.51 | 0.98x |

worst: SPEED=3

## hotspots at SPEED=3: 430 frames, 429.9 fps

no measure_layer timers fired for fireworks; wrap its painters in crate::_0_profile::measure_layer
