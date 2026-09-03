# knob sweep: haiku-2-ripple 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 31 | 30.7 | 32.53 | 32.60 | 33.64 | 33.77 | 1.00x |
| FREQ=12 | 30 | 29.4 | 34.05 | 33.92 | 35.81 | 36.02 | 1.05x |
| PDENSE=1.2 | 30 | 29.6 | 33.81 | 33.52 | 36.12 | 38.34 | 1.04x |
| HUE=360 | 31 | 30.1 | 33.27 | 32.95 | 35.34 | 45.93 | 1.02x |
| SPEED=3 | 31 | 30.4 | 32.91 | 32.74 | 36.11 | 36.33 | 1.01x |
| DECAY=1 | 31 | 30.8 | 32.42 | 32.37 | 33.88 | 33.96 | 1.00x |
| WFORM=2 | 42 | 41.5 | 24.08 | 23.81 | 26.20 | 27.44 | 0.74x |

worst: FREQ=12

## hotspots at FREQ=12: 30 frames, 29.1 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| waves | 1.0 | 27517.5 | 28000.8 | 80.2% |
| particles | 1.0 | 5121.4 | 7788.1 | 14.4% |
| clear | 1.0 | 918.4 | 982.8 | 2.7% |
| guide | 1.0 | 0.0 | 0.2 | 0.0% |
