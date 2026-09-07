# Gemini 3.8 Flash: Gem Aetherium 2 performance copy

Create a new standalone mode by copying the implementation in `src/modes/_34_gemini_astrolabe.rs` into `src/modes/_D_next.rs`. Do not modify `_34_gemini_astrolabe.rs`, its snapshots, or any existing mode.

Rename the copied mode’s public identity to `gem-aetherium-2`. Keep the visual subject and parameter semantics recognizable as the second Gem Aetherium renderer, then optimize the copy’s rendering path aggressively. Target large-grid throughput through bounded sampling, precomputed or cached static geometry where compatible with the current deterministic frame contract, allocation reduction, and eliminating repeated calculation in hot loops. Preserve deterministic output for an unchanged copy configuration. Do not degrade the composition into a sparse substitute.

Retain or improve the copied mode’s controls, animation, and inline tests. Add an in-file test demonstrating the optimized renderer is deterministic, time-sensitive where expected, bounded on a small grid, and parameter-safe. The primary checkout will lease the numeric filename, generate registration, create the integration snapshot, run `perf/knob_sweep.sh gem-aetherium-2`, install, and commit.

Write only `src/modes/_D_next.rs`. Do not edit generated registration, external snapshots, integration tests, performance reports, dependencies, playback, or other source files.

Commit with this exact subject:

`feat: add optimized Gem Aetherium 2 draft`

Report the original computational hot paths, changes made, expected performance effects, controls, tests, and commit hash.
