# knob sweep: fullmetal-eyes 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 234 | 233.3 | 4.29 | 4.27 | 4.62 | 5.49 | 1.00x |

worst: baseline

## hotspots at baseline: 236 frames, 235.7 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| noise | 1.0 | 2641.9 | 5248.8 | 62.3% |
| rings | 1.0 | 618.6 | 1418.2 | 14.6% |
| links | 1.0 | 16.2 | 76.4 | 0.4% |
| watchers | 1.0 | 9.7 | 39.0 | 0.2% |
| nodes | 1.0 | 6.4 | 41.1 | 0.2% |
| small_eye | 1.0 | 3.1 | 6.8 | 0.1% |
| node_eyes | 1.0 | 3.0 | 8.6 | 0.1% |
| runes | 1.0 | 2.8 | 8.9 | 0.1% |
