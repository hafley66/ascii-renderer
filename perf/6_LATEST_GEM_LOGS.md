# User run, 2026-09-07 20:56–20:58 local

Worker PID 46289, seed 42, gem-aetherium-2, iterate, random knob rolls 0–31.
Read from `~/.local/state/ascii-renderer/renders.ndjson`; frozen records are saved
locally in ignored `perf/results/51_latest_logs/recorded.ndjson`.

These are medians of periodic `sampled=true` records. Slow-only records are
excluded to avoid selecting only slow frames. Knobs vary across these groups,
so this is descriptive evidence, not a controlled before/after benchmark.

| Art grid | Samples | Generate | Encode | Worker output | Total work | Output bytes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 162×61 | 17 | 0.295 ms | 0.537 ms | 0.027 ms | 0.879 ms | 36,197 |
| 392×134 | 84 | 1.424 ms | 2.429 ms | 130.041 ms | 134.089 ms | 134,856 |

Across 117 relay records for that worker, terminal wait occupies 97.53% of the
recorded intervals. Throughput is approximately 947,694 bytes/second. Worker
output timing includes blocking on its pipe when the supervisor cannot drain
into the terminal; it does not measure GUI painting directly.

Version 3 will be a new file-owned mode. Static celestial structure and bounded
animated marks keep per-frame terminal changes bounded as dimensions increase.
Gem 1 and Gem 2 remain available.
