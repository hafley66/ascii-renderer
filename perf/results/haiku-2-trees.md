# knob sweep: haiku-2-trees 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 763 | 762.8 | 1.31 | 1.30 | 1.42 | 2.29 | 1.00x |
| FRUIT=1 | 756 | 755.1 | 1.32 | 1.31 | 1.50 | 2.18 | 1.01x |
| BRANCH=1 | 762 | 761.0 | 1.31 | 1.30 | 1.41 | 1.56 | 1.00x |
| ENERGY=1 | 763 | 762.7 | 1.31 | 1.30 | 1.43 | 1.66 | 1.00x |

worst: FRUIT=1

## hotspots at FRUIT=1: 764 frames, 763.1 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| clear | 1.0 | 391.3 | 580.2 | 29.9% |
| trees | 1.0 | 6.1 | 24.0 | 0.5% |
