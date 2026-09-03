# knob sweep: murmuration 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 326 | 325.7 | 3.07 | 3.06 | 3.33 | 3.45 | 1.00x |
| BIRDS=500 | 302 | 301.0 | 3.32 | 3.32 | 3.63 | 4.05 | 1.08x |
| SPEED=3 | 328 | 327.4 | 3.05 | 3.05 | 3.32 | 3.52 | 0.99x |
| FLOCKS=9 | 331 | 330.1 | 3.03 | 3.03 | 3.28 | 3.31 | 0.99x |

worst: BIRDS=500

## hotspots at BIRDS=500: 301 frames, 300.7 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| sky | 1.0 | 1337.6 | 1572.1 | 40.2% |
| stars | 1.0 | 749.7 | 893.7 | 22.5% |
| density | 1.0 | 278.4 | 397.8 | 8.4% |
| birds | 1.0 | 26.0 | 37.2 | 0.8% |
| flocks | 1.0 | 0.0 | 3.1 | 0.0% |
