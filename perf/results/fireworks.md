# knob sweep: fireworks 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 414 | 413.5 | 2.42 | 2.37 | 2.65 | 7.71 | 1.00x |
| BURSTS=12 | 418 | 417.1 | 2.40 | 2.38 | 2.59 | 3.27 | 0.99x |
| SPEED=3 | 421 | 420.1 | 2.38 | 2.36 | 2.57 | 2.75 | 0.98x |
| SPARKS=48 | 422 | 421.5 | 2.37 | 2.35 | 2.54 | 2.79 | 0.98x |

worst: BURSTS=12

## hotspots at BURSTS=12: 419 frames, 418.7 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| stars | 1.0 | 1065.3 | 1245.8 | 44.6% |
| clear | 1.0 | 401.5 | 854.9 | 16.8% |
| bursts | 1.0 | 6.5 | 14.1 | 0.3% |
| horizon | 1.0 | 3.8 | 25.8 | 0.2% |
