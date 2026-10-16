# beacon benchmark renders

All commands run from the repo root. The release binary is at
`$CARGO_TARGET_DIR/release/ascii-renderer` (in this container `/build/target/release/ascii-renderer`).

## Deterministic frames (seed 42, 80x24, times 0 and 6)

```bash
# plain-text frame (ANSI stripped), time 0 and 6
ASCII_T=0 $BIN 42 beacon night > art/benchmark-136/frame_t0.ans
ASCII_T=6 $BIN 42 beacon night > art/benchmark-136/frame_t6.ans
# strip escape codes for the .txt previews (python, see receipt)

# 80x24 override like the snapshot tests use
ASCII_T=0 ASCII_GRID_W=80 ASCII_GRID_H=24 $BIN 42 beacon night
ASCII_T=6 ASCII_GRID_W=80 ASCII_GRID_H=24 $BIN 42 beacon night
```

Saved artifacts:

- `frame_t0.ans` / `frame_t6.ans` -- raw ANSI frames for terminal replay
  (`cat art/benchmark-136/frame_t0.ans` in a real terminal).
- `frame_t0.txt` / `frame_t6.txt` -- the same frames, ANSI stripped, 24 x 80.

## Custom knob values

Positional mode args override knobs in PARAMS order:
`[spin] [swell] [tide] [stars] [moon] [beam] [glow] [tower] [hue] [flick]`

```bash
# fast spin, rough sea, warm hue shift
ASCII_T=3 $BIN 42 beacon night 1.2 1.8 0.62 0.14 4 0.3 1.2 0.8 30 0.3
```

## Live knobs

`ASCII_P_<KEY>` environment overrides, e.g. `ASCII_P_SPIN=1.0`, match the demo
slider keys. The demo browser (`$BIN 42 demo`) exposes the same knobs; `a`
animates, `q` exits.

## Headless probe

`probe/report.json` and `probe/guard.ndjson`: live demo playback (picker ->
beacon -> `a`) under `scripts/5_probe_guard.py` in a 120x40 PTY. Six seconds of
animation produced two windows of output with different content hashes and a
clean `q`/`q` exit 0, peak owned RSS 16.9 MiB. Application cadence only; no GUI
painting was observed (no iTerm in this container).
