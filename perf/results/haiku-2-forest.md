# knob sweep: haiku-2-forest 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 243 | 242.6 | 4.12 | 3.89 | 6.71 | 8.37 | 1.00x |
| ATMOS=1 | 184 | 183.7 | 5.44 | 4.41 | 12.80 | 14.95 | 1.32x |
| HUE=180 | 238 | 237.5 | 4.21 | 3.91 | 7.71 | 9.88 | 1.02x |
| SPEED=2 | 255 | 254.6 | 3.93 | 3.90 | 4.14 | 10.74 | 0.95x |
| SWAY=1 | 258 | 257.7 | 3.88 | 3.86 | 4.29 | 6.49 | 0.94x |
| LAYERS=3 | 259 | 258.4 | 3.87 | 3.86 | 4.09 | 4.26 | 0.94x |
| DENSITY=1 | 278 | 277.4 | 3.61 | 3.60 | 3.82 | 3.84 | 0.87x |

worst: ATMOS=1

## hotspots at ATMOS=1: 228 frames, 227.7 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| atmosphere | 1.0 | 1437.1 | 4443.4 | 32.7% |
| near_layer | 1.0 | 824.3 | 1832.7 | 18.8% |
| ground | 1.0 | 594.3 | 1289.5 | 13.5% |
| mid_layer | 1.0 | 276.2 | 453.8 | 6.3% |
| sky | 1.0 | 128.6 | 351.1 | 2.9% |
| far_layer | 1.0 | 122.0 | 202.0 | 2.8% |
| horizon_mist | 1.0 | 41.0 | 77.8 | 0.9% |
