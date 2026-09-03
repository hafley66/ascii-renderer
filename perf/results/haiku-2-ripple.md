# knob sweep: haiku-2-ripple 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 31 | 30.8 | 32.50 | 32.35 | 33.74 | 33.77 | 1.00x |
| FREQ=12 | 30 | 29.2 | 34.24 | 34.10 | 36.49 | 37.09 | 1.05x |
| PDENSE=1.2 | 30 | 29.8 | 33.51 | 33.34 | 35.76 | 35.80 | 1.03x |
| HUE=360 | 31 | 30.7 | 32.53 | 32.33 | 34.23 | 36.75 | 1.00x |
| SPEED=3 | 31 | 30.9 | 32.38 | 32.30 | 33.78 | 33.90 | 1.00x |
| DECAY=1 | 31 | 31.0 | 32.31 | 32.15 | 33.68 | 33.77 | 0.99x |
| WFORM=2 | 42 | 41.3 | 24.19 | 24.02 | 25.57 | 25.75 | 0.74x |

worst: FREQ=12

## hotspots at FREQ=12: 29 frames, 28.9 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| waves | 1.0 | 27373.9 | 28252.8 | 79.1% |
| particles | 1.0 | 5557.6 | 9167.8 | 15.5% |
| clear | 1.0 | 916.7 | 962.5 | 2.7% |
| guide | 1.0 | 0.0 | 0.2 | 0.0% |
