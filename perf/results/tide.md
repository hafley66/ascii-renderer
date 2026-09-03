# knob sweep: tide 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 76 | 75.3 | 13.28 | 13.33 | 13.51 | 13.68 | 1.00x |
| WAVES=4 | 75 | 74.7 | 13.39 | 13.38 | 14.90 | 15.18 | 1.01x |
| AMP=2.5 | 76 | 75.4 | 13.27 | 13.33 | 13.55 | 13.64 | 1.00x |
| SPEED=3 | 76 | 75.5 | 13.25 | 13.34 | 13.46 | 13.49 | 1.00x |

worst: WAVES=4

## hotspots at WAVES=4: 76 frames, 75.2 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| sea | 1.0 | 12362.7 | 12625.4 | 93.0% |
| waves | 1.0 | 0.1 | 0.4 | 0.0% |
