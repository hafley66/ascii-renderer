# knob sweep: vesper 2000x1000, 5s per run, dt 0.06, theme deep

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 122 | 24.3 | 41.11 | 41.03 | 43.07 | 46.80 | 1.00x |
| THREADS=160 | 78 | 15.5 | 64.63 | 64.39 | 71.03 | 71.52 | 1.57x |
| SPEED=2 | 116 | 23.1 | 43.34 | 42.40 | 56.91 | 62.84 | 1.05x |
| FOLDS=9 | 117 | 23.2 | 43.06 | 42.93 | 47.74 | 49.85 | 1.05x |
| OPEN=0.65 | 124 | 24.8 | 40.33 | 40.15 | 44.73 | 46.75 | 0.98x |

worst: THREADS=160

## hotspots at THREADS=160: 75 frames, 14.8 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| filaments | 1.0 | 46617.9 | 54733.4 | 69.2% |
| eclipse | 1.0 | 15287.6 | 22386.2 | 22.7% |
| composite | 1.0 | 2842.1 | 3733.9 | 4.2% |
