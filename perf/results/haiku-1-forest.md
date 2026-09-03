# knob sweep: haiku-1-forest 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 227 | 226.8 | 4.41 | 4.41 | 4.61 | 4.71 | 1.00x |
| ATMOS=1 | 179 | 178.2 | 5.61 | 5.61 | 5.83 | 5.86 | 1.27x |
| DENSITY=1 | 226 | 225.4 | 4.44 | 4.41 | 4.74 | 7.39 | 1.01x |
| LAYERS=5 | 226 | 225.5 | 4.43 | 4.44 | 4.64 | 4.82 | 1.01x |
| HUE=1 | 227 | 226.3 | 4.42 | 4.41 | 4.70 | 4.78 | 1.00x |
| SWAY=2 | 227 | 226.6 | 4.41 | 4.41 | 4.63 | 4.63 | 1.00x |
| SPEED=1 | 228 | 227.2 | 4.40 | 4.39 | 4.64 | 4.74 | 1.00x |

worst: ATMOS=1

## hotspots at ATMOS=1: 179 frames, 178.5 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| atmosphere | 1.0 | 3285.3 | 3409.5 | 58.6% |
| clear | 1.0 | 927.5 | 1066.1 | 16.6% |
| sky | 1.0 | 425.4 | 524.2 | 7.6% |
| layers | 1.0 | 17.8 | 28.4 | 0.3% |
| ground | 1.0 | 1.1 | 6.3 | 0.0% |
