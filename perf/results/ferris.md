# knob sweep: ferris 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 248 | 247.4 | 4.04 | 4.02 | 4.41 | 4.51 | 1.00x |
| SPEED=3 | 246 | 245.6 | 4.07 | 4.05 | 4.39 | 4.59 | 1.01x |
| GONDOLAS=14 | 247 | 246.5 | 4.06 | 4.03 | 4.40 | 4.82 | 1.00x |
| RADIUS=12 | 249 | 248.7 | 4.02 | 4.00 | 4.29 | 4.70 | 0.99x |

worst: SPEED=3

## hotspots at SPEED=3: 243 frames, 242.7 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| crown | 1.0 | 2774.7 | 5900.2 | 67.4% |
| clear | 1.0 | 401.9 | 756.9 | 9.8% |
| rim | 1.0 | 0.6 | 1.2 | 0.0% |
| gondolas | 1.0 | 0.4 | 8.2 | 0.0% |
| spokes | 1.0 | 0.3 | 3.5 | 0.0% |
| lights | 1.0 | 0.2 | 0.6 | 0.0% |
| struts | 1.0 | 0.1 | 0.5 | 0.0% |
| riders | 1.0 | 0.1 | 0.3 | 0.0% |
| queue | 1.0 | 0.0 | 3.5 | 0.0% |
