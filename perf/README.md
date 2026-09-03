# perf

Headless knob sweeps and layer hotspots for one mode at a time.

## Run

```bash
perf/knob_sweep.sh chladni              # 2000x1000, 5 s per run
perf/knob_sweep.sh pendulum-wave 2000 1000 5 0.06 moss
```

Output lands in `perf/results/<mode>.md`.

## What it does

1. Renders the mode with default knobs for N seconds and counts frames.
2. Repeats once per registry knob with that knob at its `max`, all others default.
3. Sorts by fps and names the worst knob.
4. Reruns the worst knob with an in-process layer capture open and prints per-layer
   calls, average, max, and share of the frame.

## Layer timers

Painters wrapped in `crate::_0_profile::measure_layer(mode, layer, || ...)` show up in
the hotspot table. The same timers emit tracing events when `ASCII_PROFILE=1
ASCII_PROFILE_LAYERS=1` is set on the binary (see `PERF.md` for the JSON log form).

## Env

| var | default |
| --- | --- |
| ASCII_PERF_MODE | chladni |
| ASCII_PERF_WIDTH x ASCII_PERF_HEIGHT | 2000 x 1000 |
| ASCII_PERF_SECS | 5 |
| ASCII_PERF_DT | 0.06 |
| ASCII_PERF_THEME | moss |

## All modes

```bash
perf/sweep_all.sh 1        # 1 s per run, every knob-form and registered mode
```

Writes one `perf/results/<mode>.md` per mode and ranks them in
`perf/results/SUMMARY.md` by the fps of each mode's worst knob.
