# knob sweep: murmuration 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 332 | 331.5 | 3.02 | 2.99 | 3.28 | 3.86 | 1.00x |
| BIRDS=500 | 298 | 297.1 | 3.37 | 3.30 | 4.26 | 4.57 | 1.12x |
| FLOCKS=9 | 331 | 329.9 | 3.03 | 2.99 | 3.37 | 3.53 | 1.00x |
| SPEED=3 | 333 | 332.4 | 3.01 | 2.98 | 3.28 | 4.21 | 1.00x |

worst: BIRDS=500

## hotspots at BIRDS=500: 303 frames, 302.6 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| sky | 1.0 | 1343.3 | 1567.8 | 40.7% |
| stars | 1.0 | 739.5 | 816.0 | 22.4% |
| density | 1.0 | 275.0 | 373.9 | 8.3% |
| birds | 1.0 | 25.4 | 40.8 | 0.8% |
| flocks | 1.0 | 0.0 | 0.2 | 0.0% |
