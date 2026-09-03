# knob sweep: braid 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 193 | 192.2 | 5.20 | 5.19 | 5.48 | 5.54 | 1.00x |
| STRANDS=16 | 186 | 185.8 | 5.38 | 5.36 | 5.68 | 5.85 | 1.03x |
| TWIST=1 | 189 | 188.5 | 5.30 | 5.25 | 5.99 | 8.42 | 1.02x |
| FILL=1 | 190 | 189.3 | 5.28 | 5.26 | 5.61 | 5.68 | 1.02x |
| WIDTH=13 | 191 | 190.2 | 5.26 | 5.23 | 5.62 | 5.73 | 1.01x |
| PITCH=30 | 192 | 191.2 | 5.23 | 5.22 | 5.53 | 5.63 | 1.01x |
| SPEED=20 | 193 | 192.7 | 5.19 | 5.15 | 5.54 | 5.56 | 1.00x |
| GAP=40 | 193 | 192.9 | 5.18 | 5.16 | 5.52 | 5.58 | 1.00x |
| SWAY=8 | 194 | 193.2 | 5.18 | 5.15 | 5.40 | 5.60 | 0.99x |
| CROSS=1 | 194 | 193.3 | 5.17 | 5.15 | 5.49 | 5.51 | 0.99x |
| DUST=1 | 207 | 206.3 | 4.85 | 4.82 | 5.17 | 8.00 | 0.93x |

worst: STRANDS=16

## hotspots at STRANDS=16: 184 frames, 183.4 fps

no measure_layer timers fired for braid; wrap its painters in crate::_0_profile::measure_layer
