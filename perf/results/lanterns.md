# knob sweep: lanterns 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 239 | 238.7 | 4.19 | 4.14 | 4.56 | 5.75 | 1.00x |
| SWAY=3 | 239 | 238.6 | 4.19 | 4.16 | 4.65 | 4.68 | 1.00x |
| RISE=3 | 241 | 240.2 | 4.16 | 4.13 | 4.46 | 9.09 | 0.99x |
| COUNT=24 | 241 | 240.4 | 4.16 | 4.13 | 4.42 | 4.44 | 0.99x |

worst: SWAY=3

## hotspots at SWAY=3: 238 frames, 237.6 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| water | 1.0 | 2330.3 | 2457.1 | 55.4% |
| stars | 1.0 | 543.5 | 585.9 | 12.9% |
| clear | 1.0 | 403.1 | 566.8 | 9.6% |
| lanterns | 1.0 | 1.1 | 5.1 | 0.0% |
