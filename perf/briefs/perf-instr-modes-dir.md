# perf-instr-modes-dir: wrap painters in measure_layer (pass 1 of 2)

You are pass 1 of 2. Favor plain, minimal edits. If reality deviates from this brief, STOP and write what you found in REPORT.md; do not improvise.

Repo: ascii-renderer (Rust, cargo, edition 2024). No package manager besides cargo. Never add dependencies.

## Files you own (edit nothing else)

src/modes/_31_qwen_cathedral.rs, src/modes/_32_glm_apotheosis.rs, src/modes/_33_cosmograph.rs, src/modes/_34_gemini_astrolabe.rs, src/modes/_35_hyperloom.rs

## Modes to instrument (registry names, use them verbatim as the first measure_layer argument)

qwen-cathedral, glm-apotheosis, cosmograph, gem-aetherium (file _34_gemini_astrolabe.rs), hyperloom. Reference the already wrapped siblings src/modes/_30_illuminarium.rs and src/modes/_36_aetherforge.rs for how registered modes wrap their painters.

## Read first

1. perf/INSTRUMENT.md (the pattern, rules, validation command). Follow it exactly.
2. src/chladni.rs and src/pendwave.rs as finished examples of wrapped painters.
3. src/_0_profile.rs, the measure_layer signature: measure_layer(mode: &'static str, layer: &'static str, render: impl FnOnce() -> T) -> T.

## Per mode

1. Find the draw function morph.rs calls for the mode: grep -n '"<mode>" =>' -A4 src/morph.rs.
2. Add `use crate::_0_profile::measure_layer;` at the top of the owned file if missing.
3. Wrap 3 to 8 contiguous painter sections or painter calls in paint order. Layer names snake_case: clear, background, field, strokes, glyphs, frame, and so on. Wrap the sections that do the per-cell or per-particle work; the sweep must show at least 85 percent of the frame inside layers.
4. Zero behavior change. No reordering, no new allocations, no reformatting of untouched lines.
5. Run the validation block from perf/INSTRUMENT.md for that mode. Paste the hotspot table into REPORT.md under a heading with the mode name.

## Laws

- At most 2 consecutive comment lines anywhere. No em dashes. Never the words provenance, substrate, load-bearing, regime, signal.
- `cargo test` must end with every target ok and must create no `.snap.new` file. If a snapshot changes you altered behavior: revert that wrap and try a smaller section.
- If a wrap fails the borrow checker, wrap a smaller section or wrap only the call. Never restructure the function.
- Run all cargo commands from the worktree root. Build with `cargo build --release`.

## Deliverable

1. One commit on your branch: `perf: measure_layer timers for <modes>` (list the modes).
2. REPORT.md at the worktree root: per mode, the layer table from the validation command, the number of layers, and the summed share. End with the three gate lines: `cargo test` result lines, the `.snap.new` ls output (empty), and `git status --short` (clean after commit).

## Build isolation

Before any cargo command run `export CARGO_TARGET_DIR=$PWD/target-perf-instr-modes-dir` in your shell (the worktree is $PWD). Never cd into another checkout.
