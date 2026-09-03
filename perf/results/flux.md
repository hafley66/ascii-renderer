# knob sweep: flux 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 630 | 629.6 | 1.59 | 1.57 | 1.72 | 2.07 | 1.00x |
| TRAIL=18 | 553 | 552.9 | 1.81 | 1.67 | 3.87 | 13.56 | 1.14x |
| SPEED=3 | 623 | 622.9 | 1.61 | 1.60 | 1.72 | 1.85 | 1.01x |
| COUNT=140 | 624 | 623.6 | 1.60 | 1.60 | 1.77 | 1.98 | 1.01x |

worst: TRAIL=18

## hotspots at TRAIL=18: 610 frames, 609.5 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| clear | 1.0 | 406.3 | 525.9 | 24.8% |
| field | 1.0 | 275.1 | 339.6 | 16.8% |
| particles | 1.0 | 34.4 | 43.5 | 2.1% |
