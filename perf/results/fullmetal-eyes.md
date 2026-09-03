# knob sweep: fullmetal-eyes 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 309 | 308.5 | 3.24 | 3.21 | 3.52 | 3.65 | 1.00x |

worst: baseline

## hotspots at baseline: 311 frames, 310.2 fps

no measure_layer timers fired for fullmetal-eyes; wrap its painters in crate::_0_profile::measure_layer
