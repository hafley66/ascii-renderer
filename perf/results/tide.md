# knob sweep: tide 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 20 | 19.8 | 50.51 | 50.48 | 51.66 | 52.92 | 1.00x |
| WAVES=4 | 15 | 14.1 | 70.77 | 70.83 | 72.05 | 72.54 | 1.40x |
| SPEED=3 | 20 | 19.9 | 50.16 | 50.12 | 51.01 | 51.28 | 0.99x |
| AMP=2.5 | 21 | 20.1 | 49.83 | 49.79 | 50.50 | 50.77 | 0.99x |

worst: WAVES=4

## hotspots at WAVES=4: 14 frames, 13.8 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| sea | 1.0 | 71320.5 | 73179.3 | 98.6% |
| waves | 1.0 | 0.2 | 0.3 | 0.0% |
