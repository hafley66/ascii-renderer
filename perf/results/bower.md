# knob sweep: bower 2000x1000, 5s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 1856 | 371.1 | 2.69 | 2.63 | 3.31 | 5.20 | 1.00x |
| FRUIT=1 | 1727 | 345.4 | 2.90 | 2.82 | 4.20 | 7.53 | 1.07x |
| VINES=14 | 1736 | 347.1 | 2.88 | 2.78 | 3.90 | 4.93 | 1.07x |
| DETAIL=1.5 | 1747 | 349.3 | 2.86 | 2.82 | 3.81 | 5.38 | 1.06x |
| SPEED=2 | 1779 | 355.6 | 2.81 | 2.80 | 3.82 | 5.19 | 1.04x |

worst: FRUIT=1

## hotspots at FRUIT=1: 1684 frames, 336.8 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| trees | 1.0 | 982.9 | 5397.0 | 33.1% |
| vellum | 1.0 | 522.2 | 2025.5 | 17.6% |
| tracery | 1.0 | 401.6 | 1041.0 | 13.5% |
| lanterns | 1.0 | 5.8 | 64.5 | 0.2% |
