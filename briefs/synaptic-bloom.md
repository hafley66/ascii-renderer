# synaptic-bloom perf receipt

## layer coverage (400x120, moss, seed 42)

# layer coverage: 400x120, theme moss, seed 42, dt 0.06, 3 reps, release

| mode | layers | calls/frame | attributed | nested | thin |
| --- | ---: | ---: | ---: | --- | --- |
| synaptic-bloom | 5 | 5.0 | 98.5% | no | no |

1 modes reported: 0 thin under 85 percent, 0 nested over 100 percent, 0 with no timers at all

## knob sweep (2000x1000, 2s per run, moss)

# knob sweep: synaptic-bloom 2000x1000, 2s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 1065 | 532.2 | 1.88 | 1.85 | 2.15 | 2.39 | 1.00x |
| SOMAS=4 | 1039 | 519.4 | 1.93 | 1.90 | 2.25 | 2.53 | 1.02x |
| ASPECT=4 | 1065 | 532.1 | 1.88 | 1.86 | 2.17 | 2.32 | 1.00x |
| KILL=4 | 1070 | 534.8 | 1.87 | 1.84 | 2.18 | 2.28 | 1.00x |
| PERCEIVE=14 | 1071 | 535.3 | 1.87 | 1.85 | 2.17 | 2.45 | 0.99x |
| STEP=2 | 1082 | 540.8 | 1.85 | 1.83 | 2.14 | 3.44 | 0.98x |
| ATTRACT=380 | 1082 | 540.9 | 1.85 | 1.83 | 2.14 | 2.55 | 0.98x |
| SPLIT=2.2 | 1083 | 541.0 | 1.85 | 1.82 | 2.14 | 2.38 | 0.98x |
| PULSE=30 | 1088 | 543.7 | 1.84 | 1.81 | 2.22 | 4.00 | 0.98x |
| DEPTH=1.5 | 1097 | 548.0 | 1.82 | 1.81 | 2.09 | 2.27 | 0.97x |
| TROPISM=1 | 1099 | 549.2 | 1.82 | 1.81 | 2.07 | 2.24 | 0.97x |
| SPIN=1 | 1100 | 549.9 | 1.82 | 1.81 | 2.01 | 2.49 | 0.97x |
| REPEL=3 | 1103 | 551.1 | 1.81 | 1.81 | 2.01 | 2.13 | 0.97x |
| HUE=360 | 1107 | 553.4 | 1.81 | 1.80 | 1.99 | 2.16 | 0.96x |

worst: SOMAS=4

## hotspots at SOMAS=4: 1068 frames, 533.6 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| field | 1.0 | 403.1 | 627.5 | 21.5% |
| grow | 1.0 | 63.1 | 89.2 | 3.4% |
| edges | 1.0 | 0.7 | 5.2 | 0.0% |
| somas | 1.0 | 0.3 | 3.6 | 0.0% |
| pulses | 1.0 | 0.1 | 1.1 | 0.0% |
