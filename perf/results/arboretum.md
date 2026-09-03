# knob sweep: arboretum 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 148 | 147.4 | 6.79 | 6.70 | 8.16 | 9.58 | 1.00x |
| DENS=60 | 103 | 102.2 | 9.78 | 9.69 | 10.47 | 15.01 | 1.44x |
| GIRTH=3 | 123 | 122.0 | 8.19 | 8.13 | 9.15 | 9.29 | 1.21x |
| RELIEF=1 | 130 | 129.7 | 7.71 | 7.63 | 8.75 | 9.50 | 1.14x |
| CLUMP=1 | 147 | 146.6 | 6.82 | 6.68 | 8.76 | 10.29 | 1.01x |
| FERNS=1 | 149 | 149.0 | 6.71 | 6.62 | 7.95 | 9.34 | 0.99x |
| GALE=1 | 150 | 149.8 | 6.67 | 6.54 | 8.08 | 8.31 | 0.98x |
| SPEED=3 | 152 | 151.5 | 6.60 | 6.51 | 7.43 | 7.57 | 0.97x |
| HAZE=1 | 153 | 152.2 | 6.57 | 6.43 | 7.58 | 19.68 | 0.97x |
| DRIFT=180 | 154 | 153.5 | 6.52 | 6.45 | 7.84 | 9.03 | 0.96x |
| STRATA=4 | 183 | 182.4 | 5.48 | 5.41 | 6.47 | 6.99 | 0.81x |
| SPECIES=1 | 204 | 203.7 | 4.91 | 4.85 | 5.67 | 6.34 | 0.72x |
| CLEAR=1 | 221 | 220.3 | 4.54 | 4.49 | 5.27 | 6.22 | 0.67x |

worst: DENS=60

## hotspots at DENS=60: 107 frames, 106.8 fps

no measure_layer timers fired for arboretum; wrap its painters in crate::_0_profile::measure_layer
