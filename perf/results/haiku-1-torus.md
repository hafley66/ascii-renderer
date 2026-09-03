# knob sweep: haiku-1-torus 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 248 | 247.3 | 4.04 | 4.02 | 4.63 | 4.82 | 1.00x |
| DENSITY=2 | 243 | 242.7 | 4.12 | 4.05 | 4.54 | 9.16 | 1.02x |
| HUE=360 | 243 | 242.7 | 4.12 | 4.07 | 4.86 | 7.64 | 1.02x |
| SCALE=3 | 248 | 247.8 | 4.04 | 4.01 | 4.44 | 6.57 | 1.00x |
| MINOR=0.6 | 252 | 251.7 | 3.97 | 3.94 | 4.42 | 10.02 | 0.98x |
| SPEED=4 | 257 | 256.2 | 3.90 | 3.87 | 4.33 | 4.38 | 0.96x |
| TILT=1.57 | 258 | 257.5 | 3.88 | 3.83 | 4.33 | 9.95 | 0.96x |

worst: DENSITY=2

## hotspots at DENSITY=2: 228 frames, 227.1 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| surface | 1.0 | 2952.0 | 3537.6 | 67.1% |
| clear | 1.0 | 462.6 | 3054.5 | 10.5% |
| frame | 1.0 | 22.4 | 26.6 | 0.5% |
| accent_ring | 1.0 | 0.7 | 1.5 | 0.0% |
| center_pulse | 1.0 | 0.1 | 0.8 | 0.0% |
