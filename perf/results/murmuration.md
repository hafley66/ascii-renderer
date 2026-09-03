# knob sweep: murmuration 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 328 | 327.9 | 3.05 | 3.06 | 3.24 | 3.46 | 1.00x |
| BIRDS=500 | 300 | 299.8 | 3.34 | 3.34 | 3.53 | 3.71 | 1.09x |
| FLOCKS=9 | 327 | 326.5 | 3.06 | 3.06 | 3.39 | 3.78 | 1.00x |
| SPEED=3 | 328 | 327.6 | 3.05 | 3.07 | 3.20 | 3.41 | 1.00x |

worst: BIRDS=500

## hotspots at BIRDS=500: 301 frames, 300.5 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| sky | 1.0 | 1342.9 | 1621.5 | 40.4% |
| stars | 1.0 | 749.5 | 925.3 | 22.5% |
| density | 1.0 | 279.0 | 334.7 | 8.4% |
| birds | 1.0 | 25.5 | 31.5 | 0.8% |
| flocks | 1.0 | 0.0 | 0.2 | 0.0% |
