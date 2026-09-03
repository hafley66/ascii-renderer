# knob sweep: haiku-1-forest 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 198 | 197.9 | 5.05 | 4.55 | 9.50 | 11.50 | 1.00x |
| ATMOS=1 | 176 | 176.0 | 5.68 | 5.68 | 6.04 | 6.28 | 1.12x |
| DENSITY=1 | 197 | 196.2 | 5.10 | 4.54 | 9.74 | 10.62 | 1.01x |
| LAYERS=5 | 198 | 197.6 | 5.06 | 4.51 | 9.34 | 13.02 | 1.00x |
| SPEED=1 | 223 | 222.8 | 4.49 | 4.46 | 5.02 | 8.28 | 0.89x |
| SWAY=2 | 224 | 223.5 | 4.47 | 4.46 | 4.81 | 4.84 | 0.89x |
| HUE=1 | 225 | 224.4 | 4.46 | 4.45 | 4.95 | 5.09 | 0.88x |

worst: ATMOS=1

## hotspots at ATMOS=1: 170 frames, 169.3 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| atmosphere | 1.0 | 3416.5 | 5644.2 | 57.9% |
| clear | 1.0 | 998.2 | 1955.0 | 16.9% |
| sky | 1.0 | 442.1 | 725.3 | 7.5% |
| layers | 1.0 | 18.4 | 31.0 | 0.3% |
| ground | 1.0 | 2.2 | 152.7 | 0.0% |
