# knob sweep: haiku-1-trees 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 526 | 526.0 | 1.90 | 1.90 | 2.04 | 2.10 | 1.00x |
| ENERGY=1 | 523 | 522.0 | 1.92 | 1.92 | 2.08 | 3.07 | 1.01x |
| BRANCH=1 | 525 | 524.8 | 1.91 | 1.90 | 2.06 | 5.13 | 1.00x |
| FRUIT=1 | 528 | 527.1 | 1.90 | 1.90 | 2.04 | 2.08 | 1.00x |

worst: ENERGY=1

## hotspots at ENERGY=1: 527 frames, 526.2 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| clear | 1.0 | 927.8 | 1131.5 | 48.8% |
| species | 1.0 | 40.0 | 72.2 | 2.1% |
| ground | 1.0 | 2.8 | 10.0 | 0.1% |
