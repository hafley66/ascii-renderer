# knob sweep: lanterns 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 236 | 235.7 | 4.24 | 4.24 | 4.39 | 4.46 | 1.00x |
| COUNT=24 | 235 | 234.7 | 4.26 | 4.25 | 4.44 | 4.46 | 1.00x |
| RISE=3 | 238 | 237.2 | 4.22 | 4.15 | 4.42 | 14.28 | 0.99x |
| SWAY=3 | 242 | 241.0 | 4.15 | 4.12 | 4.38 | 4.83 | 0.98x |

worst: COUNT=24

## hotspots at COUNT=24: 242 frames, 241.4 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| water | 1.0 | 2298.2 | 2424.7 | 55.5% |
| stars | 1.0 | 536.6 | 593.7 | 13.0% |
| clear | 1.0 | 388.0 | 507.7 | 9.4% |
| lanterns | 1.0 | 3.1 | 23.2 | 0.1% |
