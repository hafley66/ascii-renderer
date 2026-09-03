# knob sweep: astrolabe 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 871 | 870.1 | 1.15 | 1.13 | 1.32 | 1.37 | 1.00x |
| RINGS=9 | 827 | 827.0 | 1.21 | 1.20 | 1.32 | 1.42 | 1.05x |
| SPOKES=16 | 860 | 859.8 | 1.16 | 1.15 | 1.30 | 3.90 | 1.01x |
| RULEV=0.5 | 871 | 870.2 | 1.15 | 1.14 | 1.28 | 1.38 | 1.00x |
| RATE=0.5 | 873 | 872.6 | 1.15 | 1.13 | 1.29 | 1.35 | 1.00x |
| STARS=110 | 873 | 872.7 | 1.15 | 1.13 | 1.27 | 1.61 | 1.00x |
| TWINK=1 | 876 | 875.9 | 1.14 | 1.13 | 1.27 | 1.38 | 0.99x |
| ZOD=1 | 879 | 878.7 | 1.14 | 1.13 | 1.26 | 1.40 | 0.99x |

worst: RINGS=9

## hotspots at RINGS=9: 819 frames, 818.9 fps

no measure_layer timers fired for astrolabe; wrap its painters in crate::_0_profile::measure_layer
