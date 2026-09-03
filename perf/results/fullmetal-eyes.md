# knob sweep: fullmetal-eyes 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 240 | 239.9 | 4.17 | 4.15 | 4.47 | 5.25 | 1.00x |

worst: baseline

## hotspots at baseline: 234 frames, 233.9 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| noise | 1.0 | 2663.1 | 3467.4 | 62.3% |
| rings | 1.0 | 620.9 | 743.9 | 14.5% |
| links | 1.0 | 16.8 | 40.0 | 0.4% |
| watchers | 1.0 | 10.0 | 30.8 | 0.2% |
| nodes | 1.0 | 6.7 | 41.1 | 0.2% |
| small_eye | 1.0 | 3.4 | 14.5 | 0.1% |
| node_eyes | 1.0 | 3.3 | 12.6 | 0.1% |
| runes | 1.0 | 3.0 | 16.0 | 0.1% |
