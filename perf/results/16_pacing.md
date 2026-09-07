# Frame pacing: measure, change, measure

Changed `morph_worker_session` to start a frame deadline before frame work and
wait only for the remainder of its 16 ms budget. Frames already over budget poll
input immediately. Previously every frame with no pending events waited another
16 ms after rendering, encoding and blocking output. Simulation time increments,
knobs, palette and ANSI encoding are unchanged.

## Actual app measurements

Raw iTerm2, terminal 400x200, art grid 366x199, gem-aetherium-2 seed 42, fixed
recorded roll-6 knob values, randomization disabled. Both binaries were built
from this workspace; only the pacing diff separates builds. SHA256 hashes and
exact inputs are recorded separately for each run.

A demo baseline completed nine frames. The changed demo run stopped after four
frames at the 256 MiB terminal-growth breaker. A three-frame demo retry stopped
after two. Those interrupted runs are retained and excluded from the table.

To remove static preview memory costs, both binaries were then run through the
app's existing direct morph CLI. This runs the same animation supervisor and
worker, options pane, encoder and terminal output. It omits the demo's initial
party preview and selected-mode static preview:

```sh
BINARY 42 morph '' gem-aetherium-2 42 gem-aetherium-2 43 iterate
```

Order was before, after, after, before. All four guards completed with code 0.
Each observer requested exit after eight frames; analysis uses matching delta
frames 2 through 8 only, excluding the full repaint and exit-related extra
frames. Every matching input, time, phase, palette, byte count, changed-cell
count and run count agreed across all four runs. This checks metadata and byte
counts, not a captured terminal-image comparison.

| Matching delta medians | Before | After |
| --- | ---: | ---: |
| Frame interval | 70.256 ms | 62.329 ms |
| Cadence, reciprocal of median | 14.23 FPS | 16.04 FPS |
| Interval minus measured frame work | 19.066 ms | 0.031 ms |
| Output emission including backpressure | 46.529 ms | 58.904 ms |

There are 14 delta observations per build. Pooled median interval fell 11.3%;
reciprocal cadence rose 12.7%. Terminal work varied between runs, so that pooled
percentage is a workload observation, not an isolated causal estimate. The
eliminated post-frame delay is directly observed. Frame cadence does not measure
physical terminal paint completion.

Per-run median intervals in execution order: before 66.312 ms, after 53.278 ms,
after 73.458 ms, before 120.634 ms. This variation limits longer-term throughput
claims. Output consumption remains the dominant delay. No terminal memory
fix is claimed, and no further limits were raised.

## Validation and repeat

`cargo test --quiet`: 432 unit tests passed, 15 ignored; 3 integration tests
passed; all 186 mode snapshots passed. Release build passed. Source diff check
and Python compilation passed. Inputs and frame accounting assertions passed.

`scripts/8_measure_pacing.py` drives the dedicated iTerm window through the app.
Use `--direct` for the comparable four runs, `--frames 8`, `--window WINDOW_ID`,
`--watch-pid ITERM_PID`, a fresh `--directory`, and `--binary BINARY`.
The driver invokes `5_probe_guard.py` before the workload, with 15 s wall time,
256 MiB owned RSS, user-approved 256 MiB terminal growth, 768 MiB absolute
terminal RSS, 32 MiB artifacts and 2 GiB free disk minimum. Ctrl+C ends animation.
After the recorded runs, the driver also gained an observer-PID guard so an
automation failure triggers workload shutdown; this addition passed Python
compilation and was not part of the measured timing comparison.

[Raw logs, build hashes and comparison](16_pacing/).
