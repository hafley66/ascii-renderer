# knob sweep: elevator 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 304 | 303.7 | 3.29 | 3.29 | 3.53 | 3.63 | 1.00x |
| CROWD=3 | 303 | 302.1 | 3.31 | 3.30 | 3.55 | 4.60 | 1.01x |
| LIFTS=6 | 304 | 303.1 | 3.30 | 3.31 | 3.52 | 4.81 | 1.00x |
| SPEED=3 | 305 | 304.4 | 3.29 | 3.29 | 3.46 | 3.89 | 1.00x |

worst: CROWD=3

## hotspots at CROWD=3: 302 frames, 301.8 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| waiting | 1.0 | 1154.7 | 1226.8 | 34.8% |
| floors | 1.0 | 748.2 | 863.5 | 22.6% |
| clear | 1.0 | 426.8 | 565.2 | 12.9% |
| lifts | 1.0 | 12.8 | 28.7 | 0.4% |
| sky | 1.0 | 11.4 | 12.3 | 0.3% |
| lift_frames | 1.0 | 5.8 | 6.9 | 0.2% |
| cabs | 1.0 | 3.9 | 28.6 | 0.1% |
| floor_paint | 1.0 | 3.3 | 29.5 | 0.1% |
