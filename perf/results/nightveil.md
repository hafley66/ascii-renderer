# knob sweep: nightveil 2000x1000, 2s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 86 | 43.0 | 23.26 | 23.20 | 24.14 | 24.26 | 1.00x |
| CURTAINS=8 | 48 | 23.8 | 41.99 | 41.87 | 43.59 | 43.96 | 1.81x |
| WAVES=3 | 86 | 42.7 | 23.39 | 23.21 | 25.34 | 26.39 | 1.01x |
| STARS=1 | 86 | 42.8 | 23.36 | 23.30 | 24.74 | 25.87 | 1.00x |
| SHIMMER=1 | 86 | 42.8 | 23.36 | 23.25 | 25.39 | 25.67 | 1.00x |
| MOTES=2 | 86 | 42.9 | 23.28 | 23.17 | 24.91 | 27.76 | 1.00x |
| DRIFT=3 | 87 | 43.3 | 23.12 | 23.06 | 23.79 | 24.21 | 0.99x |

worst: CURTAINS=8

## hotspots at CURTAINS=8: 48 frames, 23.8 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| curtains | 1.0 | 38510.3 | 40450.4 | 91.7% |
| ridge | 1.0 | 1126.6 | 1395.2 | 2.7% |
| sky | 1.0 | 695.3 | 995.1 | 1.7% |
| stars | 1.0 | 549.7 | 831.3 | 1.3% |
| motes | 1.0 | 39.0 | 101.6 | 0.1% |
