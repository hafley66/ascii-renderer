# knob sweep: elevator 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 302 | 301.4 | 3.32 | 3.29 | 3.68 | 3.92 | 1.00x |
| LIFTS=6 | 288 | 287.2 | 3.48 | 3.40 | 5.06 | 10.32 | 1.05x |
| SPEED=3 | 308 | 307.9 | 3.25 | 3.21 | 3.58 | 3.98 | 0.98x |
| CROWD=3 | 310 | 309.5 | 3.23 | 3.20 | 3.52 | 4.79 | 0.97x |

worst: LIFTS=6

## hotspots at LIFTS=6: 309 frames, 308.7 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| waiting | 1.0 | 1130.0 | 1243.6 | 34.9% |
| floors | 1.0 | 727.3 | 830.3 | 22.4% |
| clear | 1.0 | 382.4 | 490.7 | 11.8% |
| lifts | 1.0 | 23.8 | 91.1 | 0.7% |
| lift_frames | 1.0 | 11.6 | 31.7 | 0.4% |
| sky | 1.0 | 11.3 | 26.2 | 0.3% |
| cabs | 1.0 | 7.3 | 14.7 | 0.2% |
| floor_paint | 1.0 | 5.2 | 14.3 | 0.2% |
