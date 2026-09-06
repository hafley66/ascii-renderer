---
name: add-mode
description: Add a file-owned rendering mode to ascii-renderer through the generated dyn Mode registry, including knobs, deterministic snapshots, and animation. Use for new standalone modes or mode aliases. Do not use for a sprite with no standalone mode.
---

# Add Mode

Add one mode without changing the output of existing modes.

Read [references/0_mode-lifecycle.md](references/0_mode-lifecycle.md) before editing. It records the current dispatch, demo, animation, parameter, and snapshot seams.

## Shape the change

Before writing the body, state the proposed Rust signatures and place pseudocode comments inside the planned body. Then record:

- Instance timeline: grid allocation, RNG seeding, handler call, render, and repeated animation frames.
- Storage: CLI arguments, `ASCII_T`, `ASCII_P_<KEY>`, persisted option values, and where each value is read.
- Uniqueness: canonical mode name, aliases, parameter keys within the mode, and exactly one CLI dispatch path.

New standalone modes live in `src/modes/_N_name.rs`. Choose `N` from dependency and reading order. The file owns its `Mode` implementation, `MODE` static, parameter declarations, renderer, and snapshot tests. The generated `src/modes/mod.rs` is the only module and registration list.

## Implement

1. Run the existing tests before changing a mode family.
2. Search the canonical name and aliases across `src/`, `tests/`, and `AGENTS.md`.
3. Create `src/modes/_N_name.rs` and implement `crate::registry::Mode` for a zero-sized type.
4. Define `pub(super) static MODE`, returning the canonical name, help text, `AnimKind`, and a file-owned static parameter slice.
5. Render exclusively through `Mode::render(&self, frame: &mut ModeFrame<'_>)`. Read positional arguments from `frame.args` and live values through `param_f32`.
6. Use the existing grid, palette, RNG, scene, walker, sprite, or tree APIs. Treat every field in `ModeFrame` as borrowed frame input.
7. Add fixed-seed snapshots in the mode file. Snapshot files are generated under `src/modes/snapshots/`.
8. Run `scripts/0_generate_modes.sh`. Never edit `src/modes/mod.rs` directly.
9. Run `scripts/0_generate_modes.sh --check`, the focused test, and `cargo test`. Inspect pending snapshot output before accepting it.

## High-resolution behavior and triage

- Treat `(width, height)` as unconstrained render inputs. Before implementation, state time complexity, dominant allocations, and the maximum count derived from every knob.
- Keep per-frame work bounded by the grid and declared parameters. Avoid allocation, filesystem work, environment reads, map construction, or string formatting inside inner cell loops unless the output requires it.
- Precompute geometry and seed-derived choices outside the cell loop when they are reused. Use linear grid passes and existing retained storage where it fits the renderer's frame lifetime.
- Input capture, frame timing, conditional slow logs, and NDJSON replay are automatic in the shared CLI and animation dispatchers. Use `Mode::params` and supplied `ModeFrame` inputs; do not add mode-specific telemetry wiring. Optional `measure_layer(mode_name, layer_name, || ...)` spans can provide finer phase totals. Never emit records from individual cells or particles.
- Keep every output-affecting live control in `Mode::params`. The demo `s` hotkey, named presets, and NDJSON trace records use the declared keys to preserve an exact seed/theme/knob replay.
- Test a small fixed-seed snapshot and 100 native animated frames with **every declared knob at maximum simultaneously**: `python3 scripts/3_test_animation.py --mode <mode> --max --size 2000x2000`. This builds release, isolates options, verifies every maximum on every frame, and saves input/timing NDJSON under `perf/results/`. It prints the command to replay the slowest frame. Use `--frames` for longer runs and `--size 1000x10000` for tall grids.
- Also run the same command without `--max` to exercise random knob jumps over time. The PTY is continuously drained; measured presentation excludes emulator painting. `perf/knob_sweep.sh <mode> <width> <height> <seconds> <dt> <theme>` remains available for headless individual-knob comparisons, which do not replace the simultaneous-max test.

## Constraints

- Keep every existing mode and alias working.
- Do not add per-mode branches or lists to `src/main.rs`, `src/cli.rs`, `src/opts.rs`, `src/registry.rs`, or `src/morph.rs`.
- Do not use `#[path]`, `automod`, `inventory`, or constructor-based registration.
- Treat `(dimensions, seed, palette/theme, arguments, parameters, time)` as the render inputs.
- Recreate randomness deterministically from the supplied seed. A frame must not depend on how many frames preceded it.
- Bound all walks, recursion, particle counts, and writes by the grid dimensions.
- Preserve display width for wide Unicode cells through the existing grid and render helpers.
- Expose visual constants that materially change the result as CLI arguments or `ModeForm` parameters.
