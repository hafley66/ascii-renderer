# knob sweep: flux 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 632 | 631.9 | 1.58 | 1.57 | 1.69 | 1.88 | 1.00x |
| COUNT=140 | 618 | 617.6 | 1.62 | 1.60 | 1.77 | 1.87 | 1.02x |
| TRAIL=18 | 620 | 619.6 | 1.61 | 1.60 | 1.77 | 2.21 | 1.02x |
| SPEED=3 | 631 | 630.8 | 1.59 | 1.57 | 1.72 | 1.84 | 1.00x |

worst: COUNT=140

## hotspots at COUNT=140: 614 frames, 613.7 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| clear | 1.0 | 394.0 | 658.0 | 24.2% |
| field | 1.0 | 273.1 | 377.2 | 16.8% |
| particles | 1.0 | 42.3 | 63.2 | 2.6% |
