# knob sweep: fa6 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 162 | 161.3 | 6.20 | 6.17 | 6.49 | 6.61 | 1.00x |
| SPEED=3 | 151 | 150.5 | 6.64 | 6.51 | 9.45 | 11.90 | 1.07x |
| CHAOS=100 | 160 | 159.1 | 6.28 | 6.22 | 6.75 | 6.99 | 1.01x |
| CELLS=16 | 161 | 160.9 | 6.21 | 6.19 | 6.51 | 6.80 | 1.00x |
| DENS=100 | 162 | 162.0 | 6.17 | 6.13 | 6.58 | 7.22 | 1.00x |

worst: SPEED=3

## hotspots at SPEED=3: 163 frames, 162.3 fps

no measure_layer timers fired for fa6; wrap its painters in crate::_0_profile::measure_layer
