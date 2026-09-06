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

## Named replay inputs

```bash
ascii-renderer preset save gem2-lab 1701 gem-aetherium-2 deep RINGS=8 SPEED=1.5
ascii-renderer preset list
ascii-renderer preset show gem2-lab
ascii-renderer preset run gem2-lab
```

Presets are stored atomically in `$XDG_CONFIG_HOME/ascii-renderer/presets.json`, or `~/.config/ascii-renderer/presets.json` when `XDG_CONFIG_HOME` is unset. Saving an existing name replaces its seed, theme, and knob map.

In the demo, `s` saves the current mode, seed, theme, and effective knobs without opening a dialog. The generated name is `<mode>-<seed>-<unix-ms>` and appears in the status line.
