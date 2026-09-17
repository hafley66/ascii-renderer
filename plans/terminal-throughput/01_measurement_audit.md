# Page 1: is the timing trustworthy

Series TOC

| page | question |
| --- | --- |
| 01 (this) | do the spans measure what their names say |
| 02 | raw ceiling: bytes/sec the terminal path swallows with no app code involved |
| 03 | bytes per frame: every text-only lever, with the cost model |
| 04 | tmux and the emulator: what each layer adds |
| 05 | codebases that solved this (Rust, Go, TS, C) and what they do |
| 06 | image protocols, only if 02 to 05 leave a gap |

## Spans that exist

Two clocks, two processes. Worker = renders; relay = parent that copies worker stdout to /dev/tty.

| span | start | stop | covers | leaks in |
| --- | --- | --- | --- | --- |
| `generation` | morph.rs:1400 | :1431 | mode render into retained grid | nothing |
| `encoding` | morph.rs:1452 | :1454 | `AnsiFrameEncoder::encode` | nothing |
| `convert` | gridio.rs:329 | :387 | per-cell shadow compare + adapt | nothing |
| `emit` | gridio.rs:388 | :403 | ratatui diff + Termion byte emit | nothing |
| `presentation` | morph.rs:1455 | :1585 | status format, pane, `write_frame` to **pipe** | status diff (:1493-1516), pane (:1522-1533), pipe backpressure |
| `frame_started` | morph.rs:1325 | :1628 | whole loop | `recv_timeout(16ms - elapsed)` pacing |
| relay `read_us` | _1_playback.rs:215 | :243 | `read()` from worker pipe | nothing |
| relay `write_us` | :246 | :249 | `write()` to /dev/tty, O_NONBLOCK | nothing (nonblocking, always short) |
| relay `terminal_wait_us` | :277 | :303 | `poll(POLLOUT)` up to 2 ms | **1 ms forced `sleep` after 8 EAGAINs (:295-302)** |
| relay `child_wait_us` | :277 | :303 | `poll(POLLIN)` on worker pipe | nothing |
| `unattributed_us` | _0_profile.rs:461 | | interval minus the sum | loop overhead, `input()` crossterm poll |

## What the spans cannot see

| blind spot | why it matters |
| --- | --- |
| Worker `presentation` ends at the 64 KiB pipe, never at the terminal | a 1 MB frame is 16 pipe fills; worker "presentation" is relay speed, not terminal speed |
| Relay writes land in the PTY slave queue, 1024 B per accept (perf/results/17_pty_work.md:76) | the relay measures kernel queue drain, which is tmux reading its pty, not iTerm painting |
| No span on the tmux side | tmux parses, stores, re-serializes to iTerm; invisible to both processes |
| No span on the iTerm side except one samply capture (perf/results/15_samply_animation.md) | that capture is wall-clock inclusive; not a per-frame cost |
| `terminal_wait_us` includes self-inflicted sleeps | false-ready backoff at _1_playback.rs:295-302 sleeps 1 ms per stall burst; at 1 MB/frame and 1 KiB per accept that is up to 1000 stall opportunities per frame |
| fps at _0_profile.rs:749 = frames / wall | includes the 16 ms `recv_timeout` cap; never reports above ~60 even if the pipeline could |

## Verdict on "choked by rendering"

| claim | measured where | holds at 1000x1000 |
| --- | --- | --- |
| render is the wall | `generation_us_avg` | test it: `ASCII_PROFILE=1` run, read `generation_us_avg` vs `wall_ms/frames`. If generation < 20% of frame, rendering is not the wall |
| terminal is the wall | `terminal_wait_us` vs `interval_us` in `playback_relay` records | polluted by the 1 ms sleeps; page 02 replaces it with a `cat` timing that has no app code in it |

Both are inferences. Neither span touches the terminal. Page 02 measures the terminal directly.

## Run this before page 02

```sh
cargo build --release
mkdir -p perf/results/29_1000x1000
ASCII_TRACE_PATH=perf/results/29_1000x1000/relay.ndjson ASCII_PROFILE=1 ASCII_PROFILE_EVERY=60 \
  ./target/release/ascii-renderer 42 morph moss prismata 42 prismata 43 iterate \
  2> perf/results/29_1000x1000/frames.log      # q after ~5 s
grep '"playback_relay"' perf/results/29_1000x1000/relay.ndjson | tail -3
grep 'animation frame profile' perf/results/29_1000x1000/frames.log | tail -3
```

Arg order: morph.rs:1174-1195 (`mode_a seed_a mode_b seed_b strat`). Frame profile lines go to stderr (`_0_profile.rs:135-145`); relay records go to `ASCII_TRACE_PATH` (`_0_profile.rs:447`).

Read four numbers: `generation_us_avg`, `presentation_us_avg`, `bytes_avg`, relay `terminal_wait_us / interval_us`.

## Red/green results (2026-09-17)

Harness: `tests/3_relay_throughput.rs` (real binary in a `portable-pty`, reader throttled to 512 B per 10 ms) and `src/_1_playback.rs` tests (`pump` with `/bin/cat` into a `nix::pty::openpty` pair).

| defect | test | red | green | fix |
| --- | --- | --- | --- | --- |
| 1 clock per frame | `slow_terminal_drops_frames_instead_of_slowing_the_animation` | walk seed 43 after 5 s (1/6 speed) | seed 46 | morph.rs `ticks` from wall time |
| 2 relay sleep backoff | `relay_moves_bytes_at_least_half_as_fast_as_cat`, `relay_keeps_pace_with_cat_into_a_slow_consumer` | never red: relay 332 vs cat 187 MiB/s; throttled 1.5 vs 1.5 | retracted | none; tests kept as guards |
| 3 stale frames queued | `pause_reaches_the_screen_without_replaying_stale_frames` | 2.8 s, 120 KB after keypress | 306 ms, 11 KB | length-framed worker output, relay sends `Control::Displayed(n)` when the terminal accepted frame n, worker holds the next frame until then |

Darwin PTY output queue measured at 1024 B before EAGAIN (`python3 pty.openpty` probe), so every byte beyond that sits in the pipe and the relay buffer, which the ack now bounds to one frame.
