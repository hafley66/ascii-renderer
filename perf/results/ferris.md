# knob sweep: ferris 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 240 | 239.8 | 4.17 | 4.17 | 4.38 | 4.44 | 1.00x |
| GONDOLAS=14 | 240 | 239.9 | 4.17 | 4.18 | 4.43 | 4.49 | 1.00x |
| RADIUS=12 | 240 | 239.9 | 4.17 | 4.17 | 4.44 | 4.77 | 1.00x |
| SPEED=3 | 241 | 240.0 | 4.17 | 4.18 | 4.45 | 4.67 | 1.00x |

worst: GONDOLAS=14

## hotspots at GONDOLAS=14: 239 frames, 238.8 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| crown | 1.0 | 2828.6 | 3518.4 | 67.5% |
| clear | 1.0 | 407.2 | 533.6 | 9.7% |
| rim | 1.0 | 0.6 | 0.9 | 0.0% |
| gondolas | 1.0 | 0.6 | 1.0 | 0.0% |
| spokes | 1.0 | 0.3 | 3.6 | 0.0% |
| lights | 1.0 | 0.2 | 0.8 | 0.0% |
| struts | 1.0 | 0.2 | 3.6 | 0.0% |
| riders | 1.0 | 0.1 | 0.2 | 0.0% |
| queue | 1.0 | 0.0 | 0.0 | 0.0% |
