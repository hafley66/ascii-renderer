# knob sweep: ferris 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 240 | 239.1 | 4.18 | 4.16 | 4.58 | 4.70 | 1.00x |
| RADIUS=12 | 241 | 240.6 | 4.16 | 4.13 | 4.59 | 4.83 | 0.99x |
| GONDOLAS=14 | 241 | 240.8 | 4.15 | 4.13 | 4.47 | 4.79 | 0.99x |
| SPEED=3 | 242 | 241.3 | 4.14 | 4.12 | 4.60 | 7.91 | 0.99x |

worst: RADIUS=12

## hotspots at RADIUS=12: 244 frames, 243.6 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| crown | 1.0 | 2757.1 | 3131.7 | 67.2% |
| clear | 1.0 | 409.6 | 792.5 | 10.0% |
| rim | 1.0 | 0.6 | 1.9 | 0.0% |
| gondolas | 1.0 | 0.4 | 5.1 | 0.0% |
| spokes | 1.0 | 0.3 | 3.5 | 0.0% |
| lights | 1.0 | 0.2 | 3.5 | 0.0% |
| struts | 1.0 | 0.2 | 1.1 | 0.0% |
| riders | 1.0 | 0.1 | 0.2 | 0.0% |
| queue | 1.0 | 0.0 | 0.5 | 0.0% |
