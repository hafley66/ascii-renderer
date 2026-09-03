# knob sweep: lanterns 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 242 | 241.1 | 4.15 | 4.12 | 4.45 | 5.17 | 1.00x |
| RISE=3 | 237 | 236.4 | 4.23 | 4.17 | 4.89 | 6.91 | 1.02x |
| COUNT=24 | 238 | 237.8 | 4.20 | 4.19 | 4.47 | 4.85 | 1.01x |
| SWAY=3 | 241 | 240.6 | 4.16 | 4.12 | 4.69 | 6.52 | 1.00x |

worst: RISE=3

## hotspots at RISE=3: 242 frames, 241.2 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| water | 1.0 | 2278.9 | 2582.0 | 55.0% |
| stars | 1.0 | 535.2 | 1030.2 | 12.9% |
| clear | 1.0 | 415.5 | 812.2 | 10.0% |
| lanterns | 1.0 | 1.0 | 8.6 | 0.0% |
