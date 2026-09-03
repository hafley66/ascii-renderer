# knob sweep: haiku-1-torus 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 242 | 241.1 | 4.15 | 4.06 | 5.19 | 8.73 | 1.00x |
| HUE=360 | 240 | 239.7 | 4.17 | 4.14 | 4.74 | 4.82 | 1.01x |
| TILT=1.57 | 241 | 240.1 | 4.17 | 4.11 | 4.67 | 5.86 | 1.00x |
| DENSITY=2 | 243 | 242.4 | 4.13 | 4.11 | 4.47 | 4.72 | 0.99x |
| SCALE=3 | 244 | 243.2 | 4.11 | 4.08 | 4.54 | 4.84 | 0.99x |
| SPEED=4 | 244 | 243.5 | 4.11 | 4.07 | 4.75 | 5.14 | 0.99x |
| MINOR=0.6 | 250 | 249.2 | 4.01 | 3.99 | 4.41 | 5.26 | 0.97x |

worst: HUE=360

## hotspots at HUE=360: 244 frames, 243.4 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| surface | 1.0 | 2758.6 | 3306.6 | 67.1% |
| clear | 1.0 | 393.0 | 843.1 | 9.6% |
| frame | 1.0 | 20.7 | 46.1 | 0.5% |
| accent_ring | 1.0 | 0.4 | 1.2 | 0.0% |
| center_pulse | 1.0 | 0.1 | 1.0 | 0.0% |
