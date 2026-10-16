# beacon -- receipt (index 136)

## Mode

**beacon**: a night coast. A banded lighthouse stands on rubble at the edge of
an animated swell; twin beams rotate from the lantern, flaring stars overhead
and dragging a glitter path across the water; a cratered moon throws a widening
glade and a mooring buoy blinks and bobs. Parametric animation only: every
frame is computed from `(seed, knobs, t)`, no retained state, no rng stream
consumption (per-element phases come from a splitmix hash, so t=0 is byte
deterministic).

File: `src/modes/_136_beacon.rs` (generated registry via
`scripts/0_generate_modes.sh`; `mod.rs` regenerated, not hand edited).

## Controls (Mode::params, positional arg order)

| arg | key | meaning | default | range |
| --- | --- | --- | --- | --- |
| 1 | SPIN | beam rotation rad/s | 0.45 | -1.5..1.5 |
| 2 | SWELL | sea swell height | 1.0 | 0..2 |
| 3 | TIDE | horizon height | 0.62 | 0.4..0.85 |
| 4 | STARS | star density | 0.14 | 0..0.5 |
| 5 | MOON | moon radius cells | 3 | 0..8 |
| 6 | BEAM | beam half width rad | 0.22 | 0.05..0.7 |
| 7 | GLOW | beam brightness | 1.0 | 0..2 |
| 8 | TOWER | tower height fraction | 0.8 | 0.35..0.95 |
| 9 | HUE | hue shift degrees | 0 | 0..360 |
| 10 | FLICK | lamp flicker depth | 0.25 | 0..1 |

Knob order: positional CLI arg, then live demo slider (`ASCII_P_<KEY>` env),
then the default above. Registry defaults equal renderer fallbacks.

## Commands

```bash
cargo build --release
ASCII_T=0 $CARGO_TARGET_DIR/release/ascii-renderer 42 beacon night   # static frame
ASCII_T=6 $CARGO_TARGET_DIR/release/ascii-renderer 42 beacon night   # +6s
$CARGO_TARGET_DIR/release/ascii-renderer 42 demo                     # demo, 'a' animates
```

Previews: `art/benchmark-136/frame_t{0,6}.{ans,txt}` (raw ANSI + stripped),
render notes in `art/benchmark-136/commands.md`, probe evidence in
`art/benchmark-136/probe/`. Prompt preserved verbatim in
`art/prompts/136_benchmark.md` before the first completed implementation.
Perf tables in `briefs/beacon.md`, raw sweep in `perf/results/beacon.md`.

## Test outcomes

- `cargo test modes::_136_beacon` -- 7/7 passed (t0 and t6 snapshots at 80x24
  seed 42, determinism + seed sensitivity, time sensitivity, knob sensitivity,
  small-dimension and parameter-extrema bounds, 200x60 frame cost).
- `cargo test --release modes::_136_beacon::tests::frame_cost -- --nocapture`
  -- avg 0.442 ms, worst 0.656 ms (budget 6 ms).
- `cargo test --test snapshot_modes beacon` -- 2/2 passed (CLI frames at
  80x24, t0 and t6; verified identical to the unit frames before accepting).
- `scripts/0_generate_modes.sh --check` -- registry up to date.
- `perf/layer_coverage.sh 400 120 3 moss beacon` -- 6 layers, 98.6% attributed,
  not thin, not nested.
- `perf/knob_sweep.sh beacon 2000 1000 2` -- no pathological knob; worst
  STARS=0.5 at 1.13x baseline; bounded painters (tower/rocks/lamp/buoy) ~0%.
- Full `cargo test`: 822 passed, 5 failed, 27 ignored. All 5 failures are
  pre-existing on the frozen baseline commit e66e74f (verified by stashing):
  `gridio::ansi_frame_tests::animation_encoder_collapses_adjacent_rgb_levels`,
  `perf_sweep::every_registered_mode_that_renders_in_process_has_layer_timers`
  (six legacy modes lack timers: slice, fma, gothic-icons, gothic-circles,
  slice-2, woven_eclipse -- beacon is not among them; the gate counts 72 modes
  checked with my mode present),
  `sonnet_2_clifford` static+moving snapshots,
  `morph::iterate_frame_tests::gem_bad_roll6_ansi_regression`.
- Snapshot review: each pending `.snap.new` was inspected before accepting
  (moon, tower, beams, glitter, buoy all present and intentional; t0 vs t6 show
  beam rotation, wave drift, buoy bob, star twinkle). No existing snapshot was
  modified. Note: the container sets `CI=true`, which makes insta default to
  NoUpdate; tests were run with `CI` unset to generate pending snapshots.

## Headless terminal check and live probe

- Headless PTY check: `42 demo` driven in a 120x40 PTY (alt-screen, raw mode,
  keypresses) -- picker selected beacon, animation ran, clean quit. Evidence:
  `art/benchmark-136/probe/`.
- Live playback probe under the repository watchdog
  (`scripts/5_probe_guard.py`): 6 s of animated playback sampled in two windows
  with different content hashes (`frames_moved: true`), exit status 0, peak
  owned RSS 16.9 MiB against a 256 MiB cap, guard exit 0 with no breaker
  events. This measures application cadence (frames rendered and changing),
  not GUI painting.

## Unavailable E2E prerequisites

- `scripts/13_e2e.sh` / `scripts/12_test_e2e.py` require a dedicated iTerm2
  window via iTerm's Python API (`com.googlecode.iterm2`), pinned tools, and a
  venv. No GUI/iTerm exists in this container, so the real-terminal E2E was
  not run and no GUI validation is claimed. The authorized headless probe
  above stands in for cadence only.

## Limitations

- Plain-text (ANSI-stripped) output loses the palette; bands on the tower, the
  sky gradient, and beam warmth read through background color only, so the
  stripped frame shows mostly outline and wave glyphs. Glyphs are ASCII-only
  (safe with the unicode-width assertions).
- Beam geometry is aspect corrected at a fixed 2.0 cols-per-row; at unusual
  cell aspect ratios the wedges skew slightly.
- The buoy blink phase and beam start angle derive from the seed, so different
  seeds start the sweep elsewhere (intended, but not tunable per knob).
- The sea is a stylized per-row sine field with hash jitter, not a simulation;
  waves do not interact with the rocks or buoy.

## Commit

`feat: add art mode 136` -- see `git log -1 --format=%H` for the final hash.
