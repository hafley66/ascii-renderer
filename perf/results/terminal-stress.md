# knob sweep: terminal-stress 400x120, 1s per run, dt 0.06, theme moss

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 4146 | 4145.3 | 0.24 | 0.22 | 0.56 | 4.59 | 1.00x |
| BACKGROUND=1 | 4176 | 4175.1 | 0.24 | 0.22 | 0.56 | 3.91 | 0.99x |
| FREQUENCY=32 | 4183 | 4182.8 | 0.24 | 0.22 | 0.44 | 14.86 | 0.99x |
| CHURN=1 | 4237 | 4236.2 | 0.24 | 0.22 | 0.49 | 3.30 | 0.98x |
| GLYPHS=1 | 4240 | 4239.8 | 0.24 | 0.22 | 0.44 | 1.06 | 0.98x |
| COLORS=216 | 4329 | 4328.0 | 0.23 | 0.22 | 0.30 | 0.54 | 0.96x |
| SPEED=120 | 4388 | 4388.0 | 0.23 | 0.22 | 0.29 | 1.40 | 0.94x |
| PATTERN=2 | 4525 | 4524.0 | 0.22 | 0.21 | 0.35 | 0.69 | 0.92x |
| DENSITY=1 | 5190 | 5189.9 | 0.19 | 0.18 | 0.35 | 1.20 | 0.80x |

worst: BACKGROUND=1

## hotspots at BACKGROUND=1: 4305 frames, 4304.8 fps

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| cells | 1.0 | 207.2 | 2511.8 | 89.2% |
| wave_table | 1.0 | 0.9 | 19.3 | 0.4% |
| knobs | 1.0 | 0.2 | 30.5 | 0.1% |
| glyphs | 1.0 | 0.0 | 0.2 | 0.0% |
