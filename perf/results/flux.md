# knob sweep: flux 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 651 | 650.6 | 1.54 | 1.47 | 2.76 | 8.62 | 1.00x |
| TRAIL=18 | 661 | 660.3 | 1.51 | 1.50 | 1.68 | 1.94 | 0.99x |
| COUNT=140 | 664 | 663.4 | 1.51 | 1.49 | 1.64 | 1.87 | 0.98x |
| SPEED=3 | 676 | 675.9 | 1.48 | 1.46 | 1.60 | 1.76 | 0.96x |

worst: TRAIL=18

## hotspots at TRAIL=18: 670 frames, 669.5 fps

no measure_layer timers fired for flux; wrap its painters in crate::_0_profile::measure_layer
