# knob sweep: tide 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 75 | 74.8 | 13.37 | 13.37 | 13.98 | 14.06 | 1.00x |
| SPEED=3 | 75 | 74.4 | 13.43 | 13.29 | 18.32 | 18.69 | 1.00x |
| WAVES=4 | 75 | 74.5 | 13.42 | 13.40 | 14.19 | 14.95 | 1.00x |
| AMP=2.5 | 76 | 75.3 | 13.27 | 13.27 | 13.54 | 13.60 | 0.99x |

worst: SPEED=3

## hotspots at SPEED=3: 76 frames, 75.3 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| sea | 1.0 | 12320.4 | 12595.5 | 92.8% |
| waves | 1.0 | 0.1 | 0.2 | 0.0% |
