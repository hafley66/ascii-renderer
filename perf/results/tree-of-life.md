# knob sweep: tree-of-life 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 534 | 533.6 | 1.87 | 1.86 | 2.06 | 2.26 | 1.00x |
| SEAM=0.9 | 518 | 517.2 | 1.93 | 1.92 | 2.08 | 2.34 | 1.03x |
| SPREAD=1.2 | 532 | 531.4 | 1.88 | 1.87 | 2.03 | 2.24 | 1.00x |
| GLOW=1 | 536 | 535.6 | 1.87 | 1.84 | 2.10 | 2.19 | 1.00x |
| MOTES=300 | 537 | 536.4 | 1.86 | 1.85 | 2.11 | 2.18 | 0.99x |
| SWAY=6 | 540 | 539.2 | 1.85 | 1.84 | 2.05 | 2.22 | 0.99x |
| SPEED=4 | 547 | 546.9 | 1.83 | 1.84 | 1.96 | 2.15 | 0.98x |
| DEPTH=11 | 548 | 547.2 | 1.83 | 1.81 | 2.02 | 2.17 | 0.98x |
| ROOTS=0.5 | 553 | 552.2 | 1.81 | 1.80 | 1.97 | 2.09 | 0.97x |

worst: SEAM=0.9

## hotspots at SEAM=0.9: 515 frames, 514.1 fps

no measure_layer timers fired for tree-of-life; wrap its painters in crate::_0_profile::measure_layer
