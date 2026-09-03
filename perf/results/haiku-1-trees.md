# knob sweep: haiku-1-trees 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 524 | 523.5 | 1.91 | 1.90 | 2.16 | 2.35 | 1.00x |
| BRANCH=1 | 455 | 454.4 | 2.20 | 1.97 | 5.47 | 7.14 | 1.15x |
| FRUIT=1 | 483 | 482.5 | 2.07 | 1.95 | 4.19 | 5.24 | 1.08x |
| ENERGY=1 | 516 | 515.2 | 1.94 | 1.92 | 2.40 | 4.52 | 1.02x |

worst: BRANCH=1

## hotspots at BRANCH=1: 464 frames, 463.9 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| clear | 1.0 | 1058.3 | 4520.0 | 49.1% |
| species | 1.0 | 51.8 | 1308.0 | 2.4% |
| ground | 1.0 | 4.5 | 698.5 | 0.2% |
