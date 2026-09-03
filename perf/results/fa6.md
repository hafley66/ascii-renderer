# knob sweep: fa6 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 137 | 136.7 | 7.32 | 6.87 | 14.64 | 24.93 | 1.00x |
| CELLS=16 | 147 | 146.5 | 6.82 | 6.86 | 7.09 | 7.46 | 0.93x |
| SPEED=3 | 147 | 146.9 | 6.81 | 6.82 | 7.89 | 10.08 | 0.93x |
| CHAOS=100 | 149 | 148.1 | 6.75 | 6.72 | 7.06 | 7.17 | 0.92x |
| DENS=100 | 151 | 150.4 | 6.65 | 6.63 | 6.87 | 6.99 | 0.91x |

worst: CELLS=16

## hotspots at CELLS=16: 151 frames, 150.1 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| background | 1.0 | 3606.5 | 3834.1 | 54.1% |
| core_void | 1.0 | 2019.9 | 2158.1 | 30.3% |
| chamber_paint | 1.0 | 37.8 | 65.0 | 0.6% |
| fractures | 1.0 | 29.5 | 73.7 | 0.4% |
| nodes | 1.0 | 21.8 | 31.0 | 0.3% |
| seals | 1.0 | 17.0 | 23.5 | 0.3% |
| finale | 1.0 | 17.0 | 20.1 | 0.3% |
| chambers | 1.0 | 1.2 | 6.2 | 0.0% |
