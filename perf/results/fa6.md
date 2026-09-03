# knob sweep: fa6 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 126 | 125.8 | 7.95 | 7.00 | 15.44 | 18.18 | 1.00x |
| CELLS=16 | 127 | 126.8 | 7.89 | 7.17 | 12.48 | 13.16 | 0.99x |
| DENS=100 | 136 | 135.2 | 7.40 | 7.10 | 9.75 | 12.79 | 0.93x |
| SPEED=3 | 137 | 136.9 | 7.30 | 7.11 | 8.88 | 11.15 | 0.92x |
| CHAOS=100 | 140 | 139.8 | 7.15 | 7.03 | 8.11 | 8.71 | 0.90x |

worst: CELLS=16

## hotspots at CELLS=16: 140 frames, 139.9 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| background | 1.0 | 3822.5 | 5967.3 | 53.5% |
| core_void | 1.0 | 2123.9 | 3640.5 | 29.7% |
| fractures | 1.0 | 64.0 | 101.6 | 0.9% |
| chamber_paint | 1.0 | 52.8 | 231.5 | 0.7% |
| nodes | 1.0 | 32.8 | 269.2 | 0.5% |
| seals | 1.0 | 21.0 | 28.0 | 0.3% |
| finale | 1.0 | 19.5 | 23.5 | 0.3% |
| chambers | 1.0 | 3.4 | 8.6 | 0.0% |
