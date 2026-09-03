# knob sweep: tide 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 76 | 75.7 | 13.21 | 13.20 | 13.46 | 13.52 | 1.00x |
| WAVES=4 | 76 | 75.4 | 13.26 | 13.24 | 13.55 | 13.63 | 1.00x |
| AMP=2.5 | 76 | 75.5 | 13.24 | 13.24 | 13.45 | 13.53 | 1.00x |
| SPEED=3 | 76 | 75.7 | 13.21 | 13.20 | 13.49 | 13.60 | 1.00x |

worst: WAVES=4

## hotspots at WAVES=4: 76 frames, 75.6 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| sea | 1.0 | 12304.4 | 12672.0 | 93.0% |
| waves | 1.0 | 0.0 | 0.4 | 0.0% |
