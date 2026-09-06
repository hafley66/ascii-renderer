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

## Named replay inputs

```bash
ascii-renderer preset save gem2-lab 1701 gem-aetherium-2 deep RINGS=8 SPEED=1.5
ascii-renderer preset list
ascii-renderer preset show gem2-lab
ascii-renderer preset run gem2-lab
```

Presets are stored atomically in `$XDG_CONFIG_HOME/ascii-renderer/presets.json`, or `~/.config/ascii-renderer/presets.json` when `XDG_CONFIG_HOME` is unset. Saving an existing name replaces its seed, theme, and knob map.

In the demo, `s` saves the current mode, seed, theme, and effective knobs without opening a dialog. The generated name is `<mode>-<seed>-<unix-ms>` and appears in the status line.
