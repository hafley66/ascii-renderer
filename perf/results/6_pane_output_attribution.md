# Pane output attribution, 2026-09-06

The actual `ascii-renderer-4:0.0` pane was `%687`, 1718x357, and had exited
back to bash when inspected. History showed `cargo run --release -- 42 demo`;
the latest Cargo startup took 0.06 seconds. Animation worker 55292 rendered
1684x356 after reserving the knob pane and status row.

Two clients were attached. Client 55243 was 1718x358 and descended through bash,
login, iTermServer and iTerm2. Client 37199 was 320x105 and descended from Instant.
Both advertised RGB. The latest large-screen run used the iTerm2 client.

## Actual run

`6_pane_55292.ndjson` preserves the 89 animation records from the default log.
There are 78 records at 1684x356. These are periodic/slow samples, so their
medians describe recorded frames, not an unbiased distribution of all frames.

| Stage | Median recorded frame | Worst recorded frame |
| --- | ---: | ---: |
| Mode generation | 3.634 ms | 6.387 ms |
| ANSI encoding | 5.276 ms | 6.298 ms |
| Worker output write | 148.827 ms | 352.768 ms |
| Total | 158.012 ms | 358.383 ms |

The final roll completed frames 643 through 658 at 4.001 frames/s, emitting
about 1.28 MB per frame. The overall large-grid interval includes faster knob
combinations and averaged 9.682 frames/s. The slowest frame spent 352.768 ms
of 358.383 ms writing output. This measured interval does not establish a
ten-second key-dispatch delay.

Replay the final actual frame, including its palette, dimensions and knobs:

```bash
cargo run --release -- replay perf/results/6_pane_55292.ndjson
```

## Controlled before/after

All declared knobs were simultaneously maximal, seed 42, native iterate,
100 frames. Config was isolated. Direct tests drained a Unix PTY. The tmux
comparison used a private server, one drained client, and explicit RGB support.
It excludes iTerm2 painting and the real session's second attached client.

| Trace file | Frames | Relay wall seconds | Render median ms | Encode median ms | Write median ms | Total median ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 6_direct_max_2000x2000_before.ndjson | 100 | 38.246 | 29.552 | 67.195 | 265.651 | 361.663 |
| 6_direct_max_2000x2000_burst.ndjson | 100 | 18.037 | 29.380 | 65.063 | 65.300 | 159.950 |
| 6_tmux_rgb_1684x356_before.ndjson | 100 | 51.191 | 5.678 | 11.581 | 464.071 | 481.397 |
| 6_tmux_rgb_1684x356_after.ndjson | 100 | 50.391 | 5.439 | 11.264 | 453.948 | 472.310 |

At 2000x2000, all 100 before/after input sets and encoded byte counts match.
The direct-PTY workload improved from 38.254 to 18.039 seconds wall time.
The RGB tmux workload was 51.219 seconds before and 50.417 seconds after;
this does not establish a material tmux frame-rate improvement.

The relay previously retried nonblocking terminal writes and polled crossterm
on each pass. On the initial isolated tmux run, 2,807,288 writes were rejected
and input polling consumed 45.219 seconds of a 55.638-second relay interval.
The revised relay checks input on a 2 ms schedule. Eight consecutive rejected
writes trigger a 25 microsecond minimum retry interval until a write succeeds.
Short stalls can still resume immediately. Timings remain wall measurements,
not CPU-cycle measurements or hard scheduling guarantees.

## tmux server sample

`6_tmux_server.sample.txt` samples the private RGB tmux server for three seconds
at one-millisecond intervals while the same all-max workload runs. Of 2,557
main-thread samples, 2,364 (92.5%) pass through `input_parse_pane`. Within that
path, 1,616 (63.2%) pass through `screen_write_collect_add`, underneath
`input_top_bit_set`, with screen flush and visible-range calculations prominent.
These are nested inclusive samples and must not be added together.

This reproduces expensive Unicode screen-update processing inside tmux without
an emulator painting the output. Renderer-only optimizations do not remove
that measured downstream work. The exact ten-second visible input delay in
the real two-client session remains unconfirmed.

## Trace and test changes

Default `playback_relay` records cover intervals with no completed frame and
partition child reads, terminal writes, input checks, control forwarding,
child-output waits and terminal-output waits. They include screen dimensions,
parent/worker PIDs, failed-write counts, maximum write duration and actual input
poll gaps. `playback_event` records input receipt, completed worker handling,
worker shutdown and session cleanup. Each NDJSON record is serialized before
appending to avoid interleaving parent and worker JSON fragments.

Default `replay FILE` selects the last frame, skipping trailing relay and input
records. The replay integration test verifies this against exact serialized
grids. A continuously writable descriptor plus a rejecting writer tests the
false-readiness case and bounds retry count while checking quit handling.

Reproduce the sustained workloads:

```bash
python3 scripts/3_test_animation.py --max --size 2000x2000 --frames 100
python3 scripts/3_test_animation.py --max --tmux --size 1684x356 --frames 100
python3 scripts/4_test_input_latency.py --size 1684x356 --stall 10 --max-ms 250
python3 scripts/4_test_input_latency.py --tmux --size 1684x356 --key knob --stall 1 --max-ms 250
```

Saved pre-change binary SHA-256:
`4ea9054831749d1e41026c6754aa1a2e98beb3b942cc8f3626fdb86d407412dd`.
Final release binary SHA-256:
`6a690731f4b73ea0a0cd822bbf2bb85800f4da769023d3bb5bcc40c565f671d5`.
