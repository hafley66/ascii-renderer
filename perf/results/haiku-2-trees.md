# knob sweep: haiku-2-trees 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 762 | 761.8 | 1.31 | 1.30 | 1.41 | 1.61 | 1.00x |
| ENERGY=1 | 749 | 748.5 | 1.34 | 1.30 | 1.45 | 8.09 | 1.02x |
| BRANCH=1 | 758 | 757.7 | 1.32 | 1.31 | 1.46 | 1.61 | 1.01x |
| FRUIT=1 | 771 | 770.1 | 1.30 | 1.28 | 1.39 | 1.49 | 0.99x |

worst: ENERGY=1

## hotspots at ENERGY=1: 716 frames, 715.2 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| clear | 1.0 | 426.6 | 1255.8 | 30.5% |
| trees | 1.0 | 8.4 | 17.4 | 0.6% |
