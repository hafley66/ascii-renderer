# Render traces and named presets

## Conditional render telemetry

Slow tracing is enabled by default. Renders at or above the 32 ms default append to:

```text
$XDG_STATE_HOME/ascii-renderer/renders.ndjson
```

Without `XDG_STATE_HOME`, the path is `~/.local/state/ascii-renderer/renders.ndjson`.

```bash
ascii-renderer 1701 gem-aetherium-2 deep
ASCII_TRACE_PATH=/tmp/renders.ndjson ASCII_TRACE_ALL=1 ascii-renderer 1701 gem-aetherium-2 deep
ASCII_TRACE_SLOW_MS=8 ascii-renderer 1701 gem-aetherium-2 deep
```

Set `ASCII_TRACE=0` to disable slow tracing. Each line is one JSON object. Fields include `kind` (`render` or `slow_render`), `ts_ms`, total `dur_us`, `render_us`, `emit_us`, mode inputs, the resolved knob map, grid dimensions, and measured layer totals when the renderer declares layers.

`terminal_size` records the successful `crossterm::terminal::size()` result as
`{"w":320,"h":103}`, captured before applying grid overrides. If the lookup
fails, it is `null` and `terminal_size_fallback` is `true`; the CLI uses 80x45 as
the default dimensions. A successful lookup sets `terminal_size_fallback` to
`false`. `grid.w` and `grid.h` remain the resolved render dimensions after
`ASCII_GRID_W` and `ASCII_GRID_H` overrides. Overrides apply independently even
when terminal lookup fails. These additive fields retain schema version `v:1`
and all existing timing and input fields.

For example, a 120x40 terminal with 320x103 grid overrides records:

```json
{"terminal_size":{"w":120,"h":40},"terminal_size_fallback":false,"grid":{"w":320,"h":103}}
```

One-shot ANSI output uses at most 64 KiB of encoding scratch storage and writes
complete chunks through a locked stdout. `emit_us` includes encoding, writes,
and the final flush. The focused Unix release probe compares the previous
formatter and the optimized encoder, then times preencoded writes and both
complete paths through a drained raw PTY:

```bash
cargo test --release --bin ascii-renderer render::tests::perf_terminal_emission_320x103 -- --ignored --exact --nocapture
```

The probe uses a deterministic 320x103 colored grid and reports microseconds per
grid over 200 iterations. PTY write timings include kernel transport and reader
backpressure; terminal emulator painting is outside this measurement.

The Unix demo and animation supervisor waits for pipe readability or terminal
writability after a stalled operation. Its readiness wait has a 2 ms timeout
for keyboard checks; ready descriptors wake it immediately.

The relay probe sends 12 Gem Aetherium 2 frames at `t=0.00..0.66` through the
same child pipe and nonblocking terminal relay used by demo. It covers 320x103
and 2000x2000, reporting generation, child emission (including pipe
backpressure), and total relay time. The PTY is continuously drained, so this
measures application and kernel transport rather than emulator painting:

```bash
ASCII_PERF_BIN="$PWD/target/release/ascii-renderer" cargo test --release --bin ascii-renderer perf_preview_relay_over_time -- --ignored --nocapture
```

Animation uses the same default slow-log path and threshold. Its records have
`kind: "slow_animation_frame"` (or `"animation_frame"` with `ASCII_TRACE_ALL=1`),
`render_us`, `encoding_us`, `presentation_us`, emitted byte count, changed-cell
count, repaint status, mode, seed, theme, animation `time`, morph `phase`,
`randomize`, `roll`, effective `knobs`, terminal dimensions, and render dimensions.
The duration excludes the deliberate wait between frames. Input maps and JSON
are allocated only when a frame qualifies for logging.

Registered native animation defers morph endpoint rendering, ink sorting, and
distance fields until switching to a strategy that needs them. The end-to-end
PTY regression runs 100 animated frames with repeated random knob jumps,
including a morph-cycle boundary, at 320x103 and 2000x2000 render sizes:

```bash
python3 scripts/3_test_animation.py target/release/ascii-renderer
```

The test uses isolated `XDG_CONFIG_HOME` options and saves its trace under
`perf/results/`. It prints a replay command for the slowest frame.
Live options honor `XDG_CONFIG_HOME`, falling back to `~/.config`.

## Named replay inputs

```bash
ascii-renderer preset save gem2-lab 1701 gem-aetherium-2 deep RINGS=8 SPEED=1.5
ascii-renderer preset list
ascii-renderer preset show gem2-lab
ascii-renderer preset run gem2-lab
```

Presets are stored atomically in `$XDG_CONFIG_HOME/ascii-renderer/presets.json`, or `~/.config/ascii-renderer/presets.json` when `XDG_CONFIG_HOME` is unset. Saving an existing name replaces its seed, theme, and knob map.

In the demo, `s` saves the current mode, seed, theme, and effective knobs without opening a dialog. The generated name is `<mode>-<seed>-<unix-ms>` and appears in the status line.

## All knobs at maximum, recorded over time

```bash
python3 scripts/3_test_animation.py --mode gem-aetherium-2 --max --size 2000x2000
```

This builds release, reads all maxima from `Mode::params`, applies them
simultaneously, and runs 100 native animation frames through the Unix PTY
supervisor. Every recorded frame must retain every maximum. The time advances
through a cycle boundary. Generation, ANSI encoding, and PTY presentation are
measured separately; emulator painting is outside the measurement.

Use `--frames 500` for a longer run, `--size 1000x10000` for a different render
resolution, and `--trace perf/results/my-run.ndjson` for a named recording.
The trace path must be new. Omit `--max` to exercise repeated random knob jumps.
Pass an existing binary as the positional argument to skip building.

```bash
ascii-renderer inputs gem-aetherium-2 max
ascii-renderer replay perf/results/my-run.ndjson 37
```

`inputs MODE [max|default]` exports one JSON input set from the registry.
`replay FILE [LINE]` renders the selected record; line numbers start at 1 and
omitting the line selects the last record. Replay restores seed, theme, actual
palette, grid dimensions, animation time, positional arguments, and declared
knobs. It supports registered mode renders and native `iterate` frames; other
morph strategies need their endpoint state and are rejected. Replay renders
one full frame; it does not recreate prior terminal diff state or emulator speed.

Mode authors use `Mode::params` and `Mode::render` with the supplied frame inputs.
The CLI and animation dispatcher capture inputs through shared `FrameInputs`
and handle conditional timing/NDJSON automatically. No per-mode telemetry code
or bespoke stress driver is required. `measure_layer` is optional for finer
phase timing. Slow tracing remains on by default; `ASCII_TRACE_ALL=1` records
all frames when running normally.

## Input latency under terminal backpressure

```bash
cargo build --release
python3 scripts/4_test_input_latency.py --size 286x103 --stall 10 --max-ms 250
python3 scripts/4_test_input_latency.py --size 2000x2000 --stall 10 --max-ms 250
python3 scripts/4_test_input_latency.py --tmux --size 286x103 --stall 10
```

The test uses isolated all-max options, waits for an animated frame, stops
reading terminal output, and sends a knob adjustment or `q`. It observes the
persisted option change for knob application and restored terminal input mode
for quit completion. `process_exit_ms` is measured separately: on macOS,
process teardown can wait for PTY output to drain even after the renderer has
handled quit and restored terminal settings. `--key knob` or `--key quit`
selects one case. The consumer stays blocked until the knob applies or the
configured stall limit is reached; quit also waits for process exit.

Unix animation writes bounded nonblocking chunks and checks controls between
writes and during backpressure. Input abandons an unfinished frame; the next
frame resets ANSI parsing and repaints from the newly applied controls. Completed
frame traces remain replayable. These tests inject input below the terminal GUI;
they do not measure a WebView's keyboard dispatch or painting latency.

## Detecting a terminal that falls behind a fast renderer

Default animation tracing records the first completed frame and a periodic
sample at least once per second of completed-frame activity, even when every
frame is below the slow threshold. Slow frames continue to be recorded as before.
The sampled record contains replay inputs, `pid`, monotonically increasing
`frame_index`, `interval_ms`, `interval_frames`, and `interval_bytes`. These
interval counters describe completed writes to the output pipe, not WebView
painting. A terminal can accumulate an output backlog while these durations
remain small. `ASCII_TRACE_ALL=1` still records every completed frame;
`ASCII_TRACE=0` disables the default log.

Verify sampling with slow-frame records deliberately suppressed:

```bash
python3 scripts/4_test_input_latency.py --key knob --sampled --stall 1 --max-ms 250
```

This checks three periodic records, exact all-max inputs and dimensions, PID,
frame indices, and interval counters while the PTY is continuously consumed,
then checks knob application under blocked output.
