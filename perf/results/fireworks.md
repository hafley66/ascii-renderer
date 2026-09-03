# knob sweep: fireworks 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 412 | 411.3 | 2.43 | 2.41 | 2.57 | 9.71 | 1.00x |
| BURSTS=12 | 410 | 410.0 | 2.44 | 2.44 | 2.60 | 3.83 | 1.00x |
| SPARKS=48 | 411 | 410.6 | 2.44 | 2.42 | 2.57 | 4.78 | 1.00x |
| SPEED=3 | 412 | 411.5 | 2.43 | 2.42 | 2.57 | 2.63 | 1.00x |

worst: BURSTS=12

## hotspots at BURSTS=12: 413 frames, 413.0 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| stars | 1.0 | 1082.5 | 4122.1 | 44.7% |
| clear | 1.0 | 412.5 | 544.8 | 17.0% |
| bursts | 1.0 | 6.5 | 11.0 | 0.3% |
| horizon | 1.0 | 3.8 | 4.9 | 0.2% |
