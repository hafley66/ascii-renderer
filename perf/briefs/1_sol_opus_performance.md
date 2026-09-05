# Opus tessellation performance repair

Read AGENTS.md. Work only inside assigned isolated checkout. You are not alone in the repository; do not revert others' edits. Own src/opus_1_quasicrystal.rs and src/opus_2_quasicrystal.rs, focused tests and perf reports. Coordinate before editing shared perf harness, Cargo.toml/Cargo.lock or playback code. Astra owns a new architectural mosaic mode. User explicitly requested Sol for this task.

## Objective and preservation contract

User loves the Opus tiling/tessellation modes but reports stutter. Diagnose and optimize both opus-1-quasicrystal and opus-2-quasicrystal. Preserve their visible geometry, density, colors, motion, controls, and existing modes. Do not change snapshots to hide regressions, reduce resolution, omit ornament, lower frame rate, freeze motion, or silently change defaults. Prioritize byte-identical full colored grids where possible; any necessary floating-point differences need explicit measurement and visual review. Implement the improvements, not just a recommendation.

## Evidence available

Existing perf/results/opus-1-quasicrystal.md: historical 2000x1000 baseline 10.05 ms; worst SYM=13 11.99 ms, shade 46.7%, solve 21.7%, edges 18.8%. opus-2 report: baseline 28.42 ms, FOLDS=7 33.21 ms, lattice 95.8%. These are historical 1s runs, not current evidence. Both renderers have thread-local caches. Read the actual keys and hot loops. New modes Bower and Vesper are unrelated to this repair.

## Method

Run tests before edits. Obtain fresh release baselines at 2000x1000 with perf/knob_sweep.sh, plus representative long animation sequences that expose rebuilds or periodic spikes. Ask root for the benchmark slot and release it when finished; do not run benchmarks concurrently with Astra. Account for cold frame separately. Check actual native playback path and debug-vs-release, ANSI generation/terminal cost, and allocations if native timings don't explain perceived stutter. Don't claim native speed eliminates terminal limitations.

Read hot loops for invariant recomputation, repeated trig/color conversion, lookup structure, bounded scan regions, cache reuse/invalidation, avoidable allocation, and write ownership. User asked earlier about Rayon: it may be used where beneficial (e.g. independent rows), but preserve draw order for overlapping writes. Request shared Cargo ownership before adding it. If adding dependency or making current API claims, follow browsing/verification requirements. Avoid inventing a task scheduler or handrolled thread pool. Keep helpers meaningful and reference counts clear.

Validate optimized outputs against captured baseline full colored grids over multiple seeds, times, themes, and low/default/high controls. Use existing insta snapshots and meaningful deterministic regression coverage for cache invalidation or changed algorithms. Run focused and full tests. Benchmark before/after under matching conditions, with p50/p99/max and expensive controls, and preserve original measurements in a separate comparison report rather than overwrite all evidence.

## Delivery

Commit milestones scoped to your files, no push. Report commits, exact changes, why they affect measured work, baseline/after timing tables, cold and tail behavior, output-equivalence evidence, run command, and remaining stutter risks. Root reviews and integrates. If shared playback code needs repair, explain exact evidence and coordinate ownership rather than expanding scope silently.

## Subsequent user steering

The user values maximum variability from random animation knob hopping. Preserve all control ranges and their interactions. Measure knob-change rebuild spikes and randomized combinations as well as steady frames. Dependency ownership for Cargo.toml/Cargo.lock is granted to Sol for measured Rayon row parallelism; root and Astra will avoid those files.
