# knob sweep: haiku-2-forest 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 264 | 263.6 | 3.79 | 3.78 | 4.20 | 4.32 | 1.00x |
| ATMOS=1 | 237 | 236.5 | 4.23 | 4.20 | 4.64 | 7.20 | 1.11x |
| SPEED=2 | 262 | 261.5 | 3.82 | 3.81 | 4.18 | 4.24 | 1.01x |
| HUE=180 | 264 | 263.5 | 3.80 | 3.78 | 4.12 | 4.27 | 1.00x |
| LAYERS=3 | 265 | 263.9 | 3.79 | 3.78 | 4.15 | 4.31 | 1.00x |
| SWAY=1 | 264 | 263.9 | 3.79 | 3.78 | 4.09 | 4.28 | 1.00x |
| DENSITY=1 | 283 | 282.3 | 3.54 | 3.53 | 3.85 | 3.95 | 0.93x |

worst: ATMOS=1

## hotspots at ATMOS=1: 237 frames, 236.5 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| atmosphere | 1.0 | 1355.6 | 1905.4 | 32.1% |
| near_layer | 1.0 | 807.0 | 953.5 | 19.1% |
| ground | 1.0 | 579.7 | 781.4 | 13.7% |
| mid_layer | 1.0 | 268.7 | 667.7 | 6.4% |
| sky | 1.0 | 117.6 | 192.7 | 2.8% |
| far_layer | 1.0 | 116.8 | 142.3 | 2.8% |
| horizon_mist | 1.0 | 40.2 | 53.9 | 1.0% |
