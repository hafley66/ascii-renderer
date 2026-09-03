# knob sweep: murmuration 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 393 | 392.5 | 2.55 | 2.52 | 2.77 | 2.91 | 1.00x |
| BIRDS=500 | 355 | 354.3 | 2.82 | 2.79 | 3.18 | 3.35 | 1.11x |
| SPEED=3 | 391 | 391.0 | 2.56 | 2.53 | 2.87 | 3.06 | 1.00x |
| FLOCKS=9 | 393 | 392.3 | 2.55 | 2.52 | 2.87 | 2.98 | 1.00x |

worst: BIRDS=500

## hotspots at BIRDS=500: 354 frames, 353.7 fps

no measure_layer timers fired for murmuration; wrap its painters in crate::_0_profile::measure_layer
