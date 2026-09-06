# Gem Aetherium 2: simultaneous maximum controls

Command (builds release automatically):

```bash
python3 scripts/3_test_animation.py --mode gem-aetherium-2 --max --size 2000x2000
```

100 native animation frames, seed 42, 2000x2000 render grid, 2034x2001 PTY.
Every frame retained all 12 declared maxima, with animation time advancing
through the cycle boundary. Wall time: 38.360 seconds.

| Stage | Median ms | Maximum ms |
|---|---:|---:|
| Generate | 29.915 | 62.115 |
| Encode ANSI | 66.186 | 133.004 |
| Write through PTY | 263.703 | 367.920 |
| Total | 359.163 | 444.238 |

The PTY was continuously drained. Presentation includes kernel transport and
backpressure; it excludes emulator painting. These measurements do not establish
interactive frame rates in a terminal emulator.

[Recorded frame inputs and timings](gem-aetherium-2-1788711921445.ndjson).
Replay the slowest generated frame:

```bash
ascii-renderer replay perf/results/gem-aetherium-2-1788711921445.ndjson 45
```

Replay restores the recorded visual inputs for a single full frame. It does not
recreate the preceding frame's terminal diff or its presentation timing.
