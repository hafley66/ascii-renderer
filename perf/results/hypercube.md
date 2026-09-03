# knob sweep: hypercube 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 604 | 603.2 | 1.66 | 1.64 | 1.84 | 2.49 | 1.00x |
| GHOSTS=5 | 605 | 604.5 | 1.65 | 1.64 | 1.83 | 2.11 | 1.00x |
| COPIES=5 | 606 | 605.3 | 1.65 | 1.63 | 1.85 | 2.68 | 1.00x |
| SPEED=3 | 607 | 606.4 | 1.65 | 1.63 | 1.82 | 2.73 | 0.99x |

worst: GHOSTS=5

## hotspots at GHOSTS=5: 604 frames, 603.9 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| clear | 1.0 | 389.7 | 543.1 | 23.5% |
| stars | 1.0 | 340.1 | 401.4 | 20.5% |
| cube | 1.0 | 14.3 | 22.5 | 0.9% |
