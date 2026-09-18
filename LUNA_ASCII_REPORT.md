# Luna implementation report

Status: implementation committed. Required GUI and headless E2E cases did not reach case execution because the script-owned release build was interrupted during cold dependency/build preparation. Full-suite completion is pending.

Commits:

- `14c71e1` picker cancel-row wrapping and deterministic test
- `2c4769c` animation seed prompt, reseeding, history, random-knob state, footer and trace inputs
- `ae43c6d` `_59_gothic_trace` mode, generated registry entry, and reviewed snapshots
- `172a157` `seed-search` GUI/headless roster and E2E documentation
- `e3ab692` curved segment rasterization, completed-path hold rendering, trace timing, mirrored frame corners, tiny-grid diagonals, ghost-overdraw correction, and seed-search picker/effective-knob assertions

Touched files:

- `src/opts.rs`
- `src/morph.rs`
- `src/modes/_59_gothic_trace.rs`
- generated `src/modes/mod.rs`
- five `src/modes/snapshots/*gothic_trace*` files
- `scripts/12_test_e2e.py`
- `scripts/16_headless_e2e.py`
- `perf/0_E2E.md`
- `LUNA_ASCII_REPORT.md`

Validation:

- `cargo test demo_picker_wraps_through_cancel`: pass
- `cargo test gothic_trace`: pass, 8 passed, with five reviewed fixed-seed snapshots
- `cargo test animation_`: seed/reseed tests passed; an unrelated existing `gridio::ansi_frame_tests::animation_encoder_collapses_adjacent_rgb_levels` failed in the filtered run
- `cargo test every_registered_mode_that_renders_in_process`: pass after adding the gothic trace layer timer
- `cargo test --locked -j 2`: 499 passed, 2 failed, 18 ignored. Failures: `gridio::ansi_frame_tests::animation_encoder_collapses_adjacent_rgb_levels` and `morph::iterate_frame_tests::gem_bad_roll6_ansi_regression`. Both reproduced individually in the parent baseline without seed/art changes. The playback and layer-timer failures from the earlier run passed on rerun.
- `scripts/0_generate_modes.sh --check`: pass
- `python3 scripts/12_test_e2e_test.py`: pass, 3 tests
- `python3 scripts/5_probe_guard_test.py`: pass, 4 tests
- `cargo fmt --check`: fail on pre-existing repository-wide formatting drift. The unrelated formatter changes were reverted.

Snapshot review:

The five ASCII snapshots were inspected directly: `t0` shows the guide frame, `t1` shows partial reveal and pen tips, `t4.5` shows completed paths during hold, `t6` shows wrapped traces, and the rosette snapshot shows woven tracing geometry. No bulk snapshot acceptance was used.

E2E case results:

- GUI `scripts/13_e2e.sh --case seed-search --mode gothic-trace`: not_run. The script entered its release build and was interrupted during cold dependency/build preparation after root observation recorded dependency progress through 130. Artifact build directory: `perf/results/e2e-build-1789747659/`.
- Headless `scripts/13_e2e.sh --headless --case seed-search --mode gothic-trace`: not_run. The same release-build preparation did not reach case execution and was interrupted. Artifact build directory: `perf/results/e2e-build-1789748181/`.
- GUI `workflow`, `backpressure`, `bad-400x200`, `max-400x200`, and the full GUI suite: not_run because the required release build did not complete.
- Headless full suite: not_run because the required release build did not complete.

Remaining limitations:

- The live seed-search workflow has not been observed through a real terminal.
- The full E2E suite has not established GUI motion, terminal-cell behavior, latency, cadence, profiler evidence, or termios restoration.
- The two reproduced Rust regression failures remain unresolved and are outside the requested files.

## Primary checkout verification

All requested implementation changes are integrated. Existing uncommitted ferrofluid files and registry entries are preserved.

- Picker regression: passed.
- Gothic mode tests: 8 passed; five snapshots reviewed as ASCII.
- Seed input, renderer reseeding, and seed/roll independence: 3 passed.
- Primary binary suite: 507 passed, 2 failed, 18 ignored. Failures: existing encoder RGB assertion (also reproduced before seed/art integration) and gem_bad_roll6_ansi_regression.
- Registry generator check and E2E runner tests: passed.
- Unsandboxed watchdog tests: 4 passed.
- Guarded headless gothic-trace seed-search: passed. Artifacts: perf/results/e2e-1789749322886/.
- Guarded GUI gothic-trace seed-search: passed. Artifacts: perf/results/e2e-1789749329385/.
- Full GUI suite attempt: failed. Artifacts: perf/results/e2e-1789748950639/. Backpressure failed an existing terminal-wait assertion; GUI cases initially failed exact-name PID lookup. Bundle-ID lookup was then corrected and the requested GUI functional case passed. Full-suite pass is not claimed.

Primary harness corrections: compare picker markers against plain screen text, check the seed footer against screen text rather than screenshot hashes, obtain terminal PID through the application bundle ID, and retain cadence gates only for performance cases. Watchdog limits are unchanged.
