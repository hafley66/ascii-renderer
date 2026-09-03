# knob sweep: fa6 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 147 | 146.1 | 6.84 | 6.81 | 7.20 | 12.14 | 1.00x |
| CHAOS=100 | 146 | 145.8 | 6.86 | 6.85 | 7.25 | 7.27 | 1.00x |
| CELLS=16 | 146 | 145.8 | 6.86 | 6.85 | 7.28 | 7.35 | 1.00x |
| DENS=100 | 147 | 146.3 | 6.84 | 6.82 | 7.14 | 7.17 | 1.00x |
| SPEED=3 | 148 | 147.0 | 6.80 | 6.78 | 7.20 | 7.28 | 0.99x |

worst: CHAOS=100

## hotspots at CHAOS=100: 145 frames, 144.7 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| background | 1.0 | 3701.7 | 3988.5 | 53.6% |
| core_void | 1.0 | 2068.5 | 2512.4 | 29.9% |
| fractures | 1.0 | 116.1 | 234.5 | 1.7% |
| chamber_paint | 1.0 | 29.0 | 52.1 | 0.4% |
| finale | 1.0 | 18.3 | 27.5 | 0.3% |
| nodes | 1.0 | 14.8 | 25.5 | 0.2% |
| seals | 1.0 | 10.2 | 15.0 | 0.1% |
| chambers | 1.0 | 1.9 | 5.6 | 0.0% |
