# Fixed-frame terminal wait attribution

Two guarded raw iTerm2 runs replayed the same seed-42, roll-6 gem-aetherium-2
frames retained in [the previous experiment](13_controlled_terminal.md).
Each run used literal, synchronized, synchronized, literal order, with one full
warmup and two retained deltas per arm. Geometry and encoding ran before replay.
The synchronized arm adds only begin/end synchronized-output commands.

Mean milliseconds per delta, four observations per arm per run:

| Run | Arm | Inside write calls | Waiting for capacity | Waiting for DSR | Through DSR |
| --- | --- | ---: | ---: | ---: | ---: |
| 1 | Literal | 0.778 | 25.830 | 21.688 | 48.514 |
| 1 | Synchronized | 0.700 | 26.474 | 11.923 | 39.275 |
| 2 | Literal | 0.775 | 26.364 | 12.314 | 39.685 |
| 2 | Synchronized | 0.797 | 26.120 | 12.217 | 39.384 |

The apparent 19% acknowledgement reduction in run 1 shrank to 0.8% in run 2.
These samples do not establish a repeatable latency improvement. Production
already enables synchronized output for direct iTerm sessions in morph.rs and
_1_playback.rs; no production behavior changed in this experiment.

The repeatable cost is time waiting for the terminal to accept and process
output. The write syscall interval accounts for less than one millisecond per
delta. This does not separate iTerm parsing, rendering, scheduling and PTY wakeup
latency. DSR acknowledges terminal stream processing, not completed painting.
The isolated harness excludes the production Rust relay and live input handling,
so it does not reproduce or explain multi-second keyboard stalls.

Instrumentation uses monotonic nanoseconds around os.write and select, plus an
explicit interval around DSR send/response. Syscall timing includes Python call
and exception handling; select timing includes scheduling delays. A memoryview
avoids copying the unwritten suffix on every attempt. Both arms share these
changes. Small loop/timer overhead explains the remaining interval.

Both watchdogs completed with exit 0 under unchanged 15-second, 256 MiB owned,
768 MiB shared and 128 MiB shared-growth limits. The dedicated window was closed.
Raw logs and harness hash are in [14_terminal_waits](14_terminal_waits/).
Validation: Python compilation and both live guarded ABBA runs. Rust files were
unchanged, so Rust tests were not rerun for this instrumentation-only change.

Repeat: copy the three retained frame files into a fresh directory and invoke
`scripts/6_terminal_ab.py DIRECTORY --experiment sync` through
`scripts/5_probe_guard.py`, with the shared iTerm PID supplied as `--watch-pid`.
Use the existing default limits and a dedicated 426x135 terminal.

Remaining validation: replay these frames through the production relay and
measure control-event latency and queue occupancy. A terminal DSR result alone
cannot establish that interactive animation is fixed.
