# knob sweep: flux 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 573 | 573.0 | 1.75 | 1.67 | 2.26 | 2.44 | 1.00x |
| COUNT=140 | 581 | 580.7 | 1.72 | 1.66 | 2.20 | 2.34 | 0.99x |
| TRAIL=18 | 583 | 582.2 | 1.72 | 1.66 | 2.21 | 2.42 | 0.98x |
| SPEED=3 | 595 | 594.6 | 1.68 | 1.62 | 2.14 | 5.37 | 0.96x |

worst: COUNT=140

## hotspots at COUNT=140: 592 frames, 591.5 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| clear | 1.0 | 421.0 | 829.2 | 24.9% |
| field | 1.0 | 280.3 | 454.8 | 16.6% |
| particles | 1.0 | 42.7 | 75.0 | 2.5% |
