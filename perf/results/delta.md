# knob sweep: delta 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 215 | 214.2 | 4.67 | 5.27 | 5.60 | 5.68 | 1.00x |
| WIND=3 | 214 | 213.9 | 4.67 | 5.23 | 5.83 | 6.14 | 1.00x |
| K=12 | 216 | 215.3 | 4.64 | 5.26 | 5.77 | 6.02 | 0.99x |
| TURB=3 | 216 | 215.4 | 4.64 | 5.28 | 5.88 | 6.09 | 0.99x |
| RBOW=1 | 216 | 215.6 | 4.64 | 5.27 | 5.85 | 6.04 | 0.99x |
| D=0.03 | 218 | 217.3 | 4.60 | 5.24 | 5.63 | 5.67 | 0.99x |
| ZETA=1 | 218 | 217.9 | 4.59 | 5.23 | 5.59 | 5.78 | 0.98x |

worst: WIND=3

## hotspots at WIND=3: 218 frames, 217.0 fps

no measure_layer timers fired for delta; wrap its painters in crate::_0_profile::measure_layer
