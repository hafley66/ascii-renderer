# knob sweep: elevator 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 301 | 300.2 | 3.33 | 3.26 | 4.44 | 12.56 | 1.00x |
| CROWD=3 | 303 | 302.4 | 3.31 | 3.27 | 3.86 | 4.04 | 0.99x |
| LIFTS=6 | 305 | 304.7 | 3.28 | 3.26 | 3.52 | 5.79 | 0.99x |
| SPEED=3 | 306 | 305.8 | 3.27 | 3.25 | 3.66 | 4.13 | 0.98x |

worst: CROWD=3

## hotspots at CROWD=3: 304 frames, 303.9 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| waiting | 1.0 | 1155.4 | 1315.9 | 35.1% |
| floors | 1.0 | 744.8 | 918.9 | 22.6% |
| clear | 1.0 | 400.8 | 925.2 | 12.2% |
| lifts | 1.0 | 13.2 | 22.9 | 0.4% |
| sky | 1.0 | 11.4 | 28.6 | 0.3% |
| lift_frames | 1.0 | 5.8 | 9.5 | 0.2% |
| cabs | 1.0 | 3.8 | 44.4 | 0.1% |
| floor_paint | 1.0 | 3.5 | 39.9 | 0.1% |
