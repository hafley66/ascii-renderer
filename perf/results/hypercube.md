# knob sweep: hypercube 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 609 | 608.8 | 1.64 | 1.64 | 1.75 | 1.90 | 1.00x |
| COPIES=5 | 605 | 604.8 | 1.65 | 1.64 | 1.86 | 2.04 | 1.01x |
| SPEED=3 | 606 | 605.5 | 1.65 | 1.64 | 1.83 | 2.38 | 1.01x |
| GHOSTS=5 | 613 | 612.1 | 1.63 | 1.63 | 1.74 | 1.94 | 0.99x |

worst: COPIES=5

## hotspots at COPIES=5: 606 frames, 605.8 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| clear | 1.0 | 385.4 | 583.1 | 23.3% |
| stars | 1.0 | 339.2 | 464.2 | 20.5% |
| cube | 1.0 | 12.8 | 18.6 | 0.8% |
