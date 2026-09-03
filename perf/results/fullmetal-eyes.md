# knob sweep: fullmetal-eyes 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 242 | 241.9 | 4.13 | 4.12 | 4.33 | 4.52 | 1.00x |

worst: baseline

## hotspots at baseline: 242 frames, 241.2 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| noise | 1.0 | 2587.7 | 2827.8 | 62.4% |
| rings | 1.0 | 603.2 | 699.3 | 14.5% |
| links | 1.0 | 15.3 | 29.7 | 0.4% |
| watchers | 1.0 | 9.0 | 18.5 | 0.2% |
| nodes | 1.0 | 5.5 | 12.9 | 0.1% |
| node_eyes | 1.0 | 2.9 | 7.7 | 0.1% |
| small_eye | 1.0 | 2.7 | 5.8 | 0.1% |
| runes | 1.0 | 2.5 | 6.0 | 0.1% |
