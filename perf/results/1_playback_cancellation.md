# Playback cancellation

## Cause

The previous morph loop generated a complete frame, encoded ANSI, wrote and
flushed stdout, then read one terminal event. Subprocess startup and MorphState
construction happened before that loop. CPU work, subprocess waits, and terminal
backpressure could therefore delay keyboard handling. A backlog of controls
required another frame per event. In raw mode, Ctrl+C arrives as keyboard input;
it does not automatically interrupt this work. Uppercase Q also lacked a handler.
Demo previews synchronously waited on a child with `Command::status`.

## Change

On Unix, `_1_playback.rs` keeps terminal input in the parent and runs animation
in a persistent subprocess. The process retains native renderer caches and Rayon
workers throughout the session. Its process group includes nested frame children.
Quit bypasses the controls queue, kills the group, and reaps the worker.

Both output pipes and a separate `/dev/tty` descriptor are nonblocking. The relay
holds at most 32 KiB of source bytes and 64 KiB of display bytes, in addition to
bounded OS pipe/terminal buffers. It polls input before each output chunk and
sleeps 2 ms only when no output advances. Cancellation discards queued terminal
output and resets partial ANSI sequences. Preview navigation also cancels work.

The parent retains 32 unsent controls, plus one partially sent record. When this
queue fills, older unsent controls are discarded in favor of recent controls.
The worker's channel holds 32 events and applies batches of up to 32 between
frames. Quit is never enqueued. Saved knobs are written through a temporary file
and atomic rename because cancellation may interrupt a worker during a save.

Ctrl+C exits demo completely. q/Q/Escape returns from animation to demo; q/Q
exits demo. Existing in-process playback remains the fallback outside Unix.
Rendering itself can still take time; cancellation is independent of its duration.

## Validation

`cargo test --offline`: 374 unit + 1 generator + 181 integration tests passed;
10 ignored. No visual snapshots changed. `cargo build --release --offline` passed.

Four supervisor tests cover CPU loops, sleeping children, an output sink that
always returns WouldBlock, 512 queued controls, q/Q/Ctrl+C, preview navigation,
newline preservation, and cancellation of a nested child before a delayed write.

Run the real-terminal regression with:

```sh
python3 scripts/2_test_quit.py target/release/ascii-renderer
```

macOS, 2000 columns by 1000 rows, release build: all 13 cases passed. Terminal
canonical mode and signal handling were restored. Latency is measured from key
injection until process exit; the harness polls on a roughly 2 ms interval.

| Case | Measured quit latency |
| --- | --- |
| Standalone morph, q/Q/Ctrl+C, startup and paused output | 5.1–7.7 ms |
| Demo preview, q/Q/Ctrl+C, startup and paused output | 4.9–5.2 ms |
| Demo animation, 512 controls followed by Ctrl+C | 10.2 ms |

For paused-output cases the harness does not read the PTY for one second before
quit. It resumes reading after key injection so the PTY can deliver cleanup bytes.
The always-blocked sink unit test independently verifies cancellation without any
output progress. These are cancellation timings, not frame-rate measurements.
