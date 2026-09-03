# knob sweep: haiku-2-ripple 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 29 | 28.7 | 34.90 | 35.05 | 36.63 | 39.93 | 1.00x |
| FREQ=12 | 29 | 28.1 | 35.60 | 35.75 | 36.73 | 36.96 | 1.02x |
| PDENSE=1.2 | 29 | 28.3 | 35.34 | 35.40 | 37.83 | 37.89 | 1.01x |
| DECAY=1 | 30 | 29.2 | 34.21 | 33.95 | 36.39 | 36.75 | 0.98x |
| HUE=360 | 30 | 29.5 | 33.93 | 33.86 | 36.06 | 36.21 | 0.97x |
| SPEED=3 | 30 | 29.6 | 33.83 | 33.91 | 35.64 | 36.13 | 0.97x |
| WFORM=2 | 40 | 39.6 | 25.24 | 25.46 | 27.00 | 27.08 | 0.72x |

worst: FREQ=12

## hotspots at FREQ=12: 29 frames, 28.2 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| waves | 1.0 | 28281.6 | 28680.2 | 79.8% |
| particles | 1.0 | 5504.5 | 6744.8 | 15.0% |
| clear | 1.0 | 926.0 | 965.1 | 2.6% |
| guide | 1.0 | 0.0 | 0.1 | 0.0% |
