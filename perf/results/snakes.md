# knob sweep: snakes 2000x1000, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 567 | 566.5 | 1.77 | 1.75 | 1.92 | 2.26 | 1.00x |
| COUNT=60 | 518 | 518.0 | 1.93 | 1.91 | 2.16 | 2.24 | 1.09x |
| SPEED=10 | 536 | 535.9 | 1.87 | 1.86 | 2.02 | 2.14 | 1.06x |
| LEN=40 | 550 | 549.4 | 1.82 | 1.80 | 2.03 | 4.58 | 1.03x |
| TURN=0.9 | 553 | 553.0 | 1.81 | 1.76 | 2.13 | 5.40 | 1.02x |
| HOP=1 | 556 | 555.8 | 1.80 | 1.78 | 1.98 | 2.26 | 1.02x |
| HOPC=1.5 | 559 | 559.0 | 1.79 | 1.77 | 1.98 | 2.09 | 1.01x |
| RBOW=1 | 563 | 562.4 | 1.78 | 1.76 | 1.94 | 2.05 | 1.01x |

worst: COUNT=60

## hotspots at COUNT=60: 519 frames, 518.3 fps

no measure_layer timers fired for snakes; wrap its painters in crate::_0_profile::measure_layer
