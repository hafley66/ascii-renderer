# knob sweep: hypercube 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 604 | 603.8 | 1.66 | 1.64 | 1.79 | 2.25 | 1.00x |
| GHOSTS=5 | 574 | 573.4 | 1.74 | 1.69 | 2.31 | 2.58 | 1.05x |
| COPIES=5 | 589 | 588.4 | 1.70 | 1.67 | 2.28 | 3.69 | 1.03x |
| SPEED=3 | 591 | 591.0 | 1.69 | 1.66 | 2.18 | 2.59 | 1.02x |

worst: GHOSTS=5

## hotspots at GHOSTS=5: 565 frames, 564.2 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| clear | 1.0 | 442.0 | 1035.5 | 24.9% |
| stars | 1.0 | 351.6 | 1244.0 | 19.8% |
| cube | 1.0 | 15.2 | 49.8 | 0.9% |
