# knob sweep: fireworks 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 413 | 412.9 | 2.42 | 2.41 | 2.65 | 3.39 | 1.00x |
| SPARKS=48 | 409 | 408.9 | 2.45 | 2.40 | 2.93 | 3.24 | 1.01x |
| BURSTS=12 | 417 | 416.8 | 2.40 | 2.38 | 2.59 | 3.35 | 0.99x |
| SPEED=3 | 419 | 419.0 | 2.39 | 2.38 | 2.55 | 2.82 | 0.99x |

worst: SPARKS=48

## hotspots at SPARKS=48: 413 frames, 412.5 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| stars | 1.0 | 1068.8 | 1154.3 | 44.1% |
| clear | 1.0 | 432.5 | 871.2 | 17.8% |
| bursts | 1.0 | 6.2 | 25.0 | 0.3% |
| horizon | 1.0 | 3.7 | 4.6 | 0.2% |
