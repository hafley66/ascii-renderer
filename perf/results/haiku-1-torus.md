# knob sweep: haiku-1-torus 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 246 | 245.7 | 4.07 | 4.10 | 4.42 | 4.77 | 1.00x |
| MINOR=0.6 | 248 | 247.7 | 4.04 | 4.01 | 4.43 | 5.46 | 0.99x |
| DENSITY=2 | 250 | 249.5 | 4.01 | 3.99 | 4.34 | 4.56 | 0.98x |
| SCALE=3 | 254 | 253.1 | 3.95 | 3.95 | 4.25 | 4.70 | 0.97x |
| HUE=360 | 254 | 253.3 | 3.95 | 3.94 | 4.17 | 4.73 | 0.97x |
| SPEED=4 | 271 | 270.4 | 3.70 | 3.59 | 4.09 | 4.52 | 0.91x |
| TILT=1.57 | 279 | 279.0 | 3.58 | 3.54 | 4.04 | 4.15 | 0.88x |

worst: MINOR=0.6

## hotspots at MINOR=0.6: 250 frames, 249.4 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| surface | 1.0 | 2677.6 | 2959.3 | 66.8% |
| clear | 1.0 | 385.6 | 563.3 | 9.6% |
| frame | 1.0 | 19.7 | 25.5 | 0.5% |
| accent_ring | 1.0 | 0.4 | 0.9 | 0.0% |
| center_pulse | 1.0 | 0.1 | 0.3 | 0.0% |
