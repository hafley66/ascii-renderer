# knob sweep: fa6 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 165 | 164.1 | 6.09 | 6.07 | 6.38 | 6.45 | 1.00x |
| DENS=100 | 164 | 163.9 | 6.10 | 6.08 | 6.46 | 6.53 | 1.00x |
| CELLS=16 | 165 | 164.2 | 6.09 | 6.06 | 6.41 | 6.92 | 1.00x |
| CHAOS=100 | 165 | 164.3 | 6.09 | 6.06 | 6.46 | 6.50 | 1.00x |
| SPEED=3 | 165 | 164.9 | 6.07 | 6.05 | 6.38 | 6.42 | 1.00x |

worst: DENS=100

## hotspots at DENS=100: 163 frames, 163.0 fps

no measure_layer timers fired for fa6; wrap its painters in crate::_0_profile::measure_layer
