# knob sweep: astra-jurassic-park 2000x1000, 1s per run, dt 0.06, theme deep

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 166 | 166.0 | 6.02 | 6.00 | 6.80 | 7.70 | 1.00x |
| RAIN=1 | 150 | 149.6 | 6.69 | 6.60 | 7.79 | 8.09 | 1.11x |
| SCALE=1.35 | 161 | 161.0 | 6.21 | 6.16 | 7.87 | 9.58 | 1.03x |
| DEPTH=1 | 162 | 161.9 | 6.18 | 5.99 | 8.85 | 10.96 | 1.03x |
| HERD=1 | 168 | 167.8 | 5.96 | 5.90 | 6.73 | 7.60 | 0.99x |
| SPEED=3 | 173 | 172.7 | 5.79 | 5.73 | 6.53 | 7.08 | 0.96x |
| FERNS=1 | 173 | 172.8 | 5.79 | 5.75 | 6.58 | 6.83 | 0.96x |
| VARIATION=1 | 174 | 173.6 | 5.76 | 5.73 | 6.37 | 6.78 | 0.96x |
| LIGHT=1 | 176 | 175.9 | 5.69 | 5.64 | 6.32 | 6.59 | 0.94x |
| SPECIES=1 | 177 | 175.9 | 5.68 | 5.65 | 6.14 | 6.27 | 0.94x |

worst: RAIN=1

## hotspots at RAIN=1: 152 frames, 151.3 fps

no measure_layer timers fired for astra-jurassic-park; wrap its painters in crate::_0_profile::measure_layer
