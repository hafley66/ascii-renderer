# Encoder split probe: ruler, and the three encoder revisions

Step 0 and step 2 of `perf/16_LARGE_RENDER_PLAN.md`. Step 2 makes the encoder stop
converting cells that did not move. Three revisions were built and measured:

| revision | reference for emission | delta at 366x199 |
| --- | --- | ---: |
| HEAD | Ratatui's retained buffer, flipped each frame | 1,904 us |
| landed | retained buffer refreshed per changed cell (`Cell::clone_from`) | 704 us |
| adapted | the encoder's own 12-byte adapted cache | 591 us |

The landed revision shipped first and is superceded by the adapted one; both are in
history. This file holds the ruler, the three-way measurements, the byte-identity
evidence, the per-mode churn spread and the live pipeline numbers. Step 3, which
removes the payload copy and the frame-buffer memmoves on the same path, is measured
in `perf/results/19_frame_buffer.md`.

## Ruler

```bash
perf/split_probe.sh [mode] [width height reps dt theme]
# defaults: prismata 366 199 15 0.06 moss -> perf/results/split_probe.md
```

`perf_sweep::perf_split_probe` is `#[ignore]`d and driven by `ASCII_SPLIT_MODE`,
`ASCII_SPLIT_WIDTH`, `ASCII_SPLIT_HEIGHT`, `ASCII_SPLIT_REPS`, `ASCII_SPLIT_DT`,
`ASCII_SPLIT_THEME`. It prints whole-mode render, blank grid build, and the full /
one-step delta / identical encode rows, each as median and min over the reps, plus
the encoder's `convert` and `emit` split, bytes, changed cells, skipped cells and
runs. `perf/results/split_probe.md` is the current run.

First run of the ruler on the adapted revision (prismata 366x199 = 72,834 cells,
moss, seed 42, dt 0.06, 15 reps):

| stage | median us | min us | convert us | emit us | bytes | cells changed | cells skipped | runs |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| render (whole mode) | 765.6 | 668.1 | - | - | - | - | - | - |
| blank grid build + row fill | 53.7 | 53.2 | - | - | - | - | - | - |
| encode full repaint | 2,519.8 | 2,345.2 | 1,583.6 | 905.8 | 78,021 | 72,834 | 0 | 199 |
| encode delta over one dt step | 617.8 | 546.7 | 395.2 | 222.2 | 25,421 | 2,806 | 67,204 | 2,476 |
| encode identical frame | 303.8 | 283.0 | 260.2 | 42.5 | 0 | 0 | 72,834 | 0 |

Section 1 of the plan reports 2013 us render, 3206 us full encode, 2087 us delta and
1992 us identical for the same grid. Those were single-shot measurements taken once
per size on a loaded host; the probe reports a median over reps after a warm-up rep
and keeps a min column, which is why its medians sit 15 to 40 percent lower. The
encoder rows are the stable ones; render still moves with host load. Treat this table
as the ruler and section 1 as the earlier reconnaissance.

A full repaint now costs about 300 us more than HEAD (2,520 against 2,214) because
it adapts every cell and refreshes both caches for every cell. There is at most one
per session in the live pipeline, and the live numbers below include it.

## Three-way measurement

Three binaries from one working tree, run alternately in the same session so all see
the same host load. 11 reps per row, prismata at 366x199 unless noted. Bytes, changed
cells and skipped cells are identical in every row of both tables.

| stage, 366x199 | HEAD | landed | adapted | adapted vs HEAD |
| --- | ---: | ---: | ---: | ---: |
| encode identical frame | 1,757.0 / 1,709.4 | 263.4 / 276.2 | 297.5 / 307.9 | 5.8x |
| encode delta over one dt step | 1,917.0 / 1,899.1 | 707.5 / 760.8 | 613.1 / 639.4 | 3.1x |
| encode full repaint | 2,214.2 / 2,093.0 | 2,165.5 / 2,270.1 | 2,493.0 / 2,596.2 | 0.86x |

Each cell is two alternating runs of the same binary in that session. The adapted
revision is the slowest of the three on a full repaint: it adapts every cell and
refreshes both caches for every cell, where the landed shape refreshed only the
retained buffer and HEAD refreshed nothing. A full repaint happens on the first
frame, on a resize, on a strategy change, on pause and after an unpresented frame,
and the live numbers below include the one that this case hits.

| prismata 800x240 (192,000 cells) | HEAD | landed | adapted |
| --- | ---: | ---: | ---: |
| encode identical frame | 4,546.3 | 753.8 / 704.7 | 828.1 / 820.0 |
| encode delta over one dt step | 5,032.8 | 2,291.5 / 2,143.2 | 1,866.8 / 1,875.6 |
| encode full repaint | 5,716.7 | 5,912.5 / 5,790.1 | 6,734.9 / 6,772.4 |

Same protocol, 9 reps, one HEAD run and two runs each for the other two. Two things to
read here. The delta row, which is the animated case, is 2.7x faster than HEAD and 1.2x
faster than the landed shape. The identical row is where the adapted revision is
lightly behind the landed one at this size (820 against 705 us, 13 percent): a resting
frame needs only the raw shadow, but the loop walks the adapted cache and the wide flag
alongside it, which is 13 bytes per cell of reads that a fully resting frame never
uses. At 366x199 that is under a percent; at 192,000 cells it is measurable. Reading
those two caches by index instead of zipping them is the obvious follow-up if resting
frames at this size ever matter, and they are the rare case in an animation.

## Per-mode spread in the animation loop

The same three binaries, 11 reps each, `IterateFrameRenderer` at dt 0.06 with default
knobs, 366x199. `cells changed` is from HEAD; it is identical on all three revisions.
This is the answer to "how much of an animation frame actually moves": the median
mode moves 4 percent of its cells per step.

| mode | cells changed per frame | HEAD us | landed us | adapted us | adapted vs HEAD |
| --- | ---: | ---: | ---: | ---: | ---: |
| snakes | 5 (0.0%) | 1,395 | 207 | 228 | 6.13x |
| chladni | 4 (0.0%) | 1,628 | 266 | 294 | 5.53x |
| astrolabe | 344 (0.5%) | 1,445 | 264 | 283 | 5.10x |
| flux | 216 (0.3%) | 1,390 | 249 | 286 | 4.87x |
| arboretum | 2,017 (2.8%) | 1,901 | 420 | 442 | 4.30x |
| tideglass | 1,337 (1.8%) | 1,728 | 646 | 509 | 3.39x |
| prismata | 2,806 (3.9%) | 1,904 | 704 | 591 | 3.22x |
| cosmograph | 2,752 (3.8%) | 1,796 | 735 | 628 | 2.86x |
| illuminarium | 5,763 (7.9%) | 2,095 | 917 | 886 | 2.36x |
| gem-aetherium-2 | 5,432 (7.5%) | 1,914 | 1,114 | 823 | 2.33x |
| poincare | 7,318 (10.0%) | 2,048 | 1,148 | 937 | 2.19x |
| delta | 9,338 (12.8%) | 1,972 | 1,004 | 910 | 2.17x |
| mahoraga-5 | 21,775 (29.9%) | 2,216 | 1,689 | 1,414 | 1.57x |
| pendulum-wave | 25,028 (34.4%) | 2,342 | 2,856 | 1,764 | 1.33x |
| hyperloom | 21,227 (29.1%) | 3,005 | 3,156 | 2,310 | 1.30x |
| terminal-stress | 55,776 (76.6%) | 3,409 | 3,414 | 3,136 | 1.09x |

The landed revision is the one row worth reading twice: at 34 percent churn
(`pendulum-wave`) it was 22 percent slower than the code it replaced, because it paid
a 48-byte reference copy per changed cell. The adapted revision pays no per-cell copy
and is faster than HEAD in every row above, including the 77 percent churn row.

## Byte identity

- Probe, 48 rows: the three encode rows report identical bytes, changed cells and
  skipped cells on all three revisions across sixteen modes, plus `prismata` at
  800x240. `gem-aetherium-2` and `cosmograph` draw zodiac wide glyphs, so the
  wide-column path is covered by measurement rather than by reasoning.
- Long sequence: `morph::iterate_frame_tests::gem_bad_roll6_ansi_regression`, 60
  frames of `gem-aetherium-2`. The test is red at HEAD for a reason that predates
  this work (it expects 4,748,083 foreground bytes and observes 0, so it never
  asserted what it intended), but its observed totals are an exact comparator and are
  identical on all three revisions: `(5,124,966, 357,555, 0, 2,019,066, 2,976,090, 0)`.
- `tests/snapshot_modes.rs`: 190 passed, 1 failed (`polytope_seed_42`), and that
  single failure occurs with the change stashed too, so it is pre-existing and
  release-only.
- Encoder unit tests: the pre-existing wide-glyph test passes unchanged, and two new
  tests cover the ways a skip can go wrong: a cell that changes, rests, then returns
  to its earlier glyph must be repainted, and a resting blank cell whose foreground
  changes must emit nothing.

## Live pipeline

E2E `max-400x200` (`scripts/13_e2e.sh --headless --mode prismata --case
max-400x200`), prismata at 366x199 with all twelve knobs at max, animation frames
only. Each run passed 9 of 9 checks. The runs draw different random knob rolls, so
frame sets are comparable in aggregate rather than row by row.

| live, 366x199 all knobs max | HEAD | landed | adapted |
| --- | ---: | ---: | ---: |
| artifact | e2e-1789593387274 | e2e-1789595082362 | e2e-1789596519290 |
| frames | 17 | 18 | 18 |
| cells changed, average | 49,697 | 49,126 | 49,126 |
| cells skipped, average | 0 | 19,270 | 19,270 |
| `convert_us`, average | 1,460 | 1,726 | 1,613 |
| `emit_us`, average | 3,352 | 3,122 | 2,674 |
| `encoding_us`, average | 4,864 | 4,893 | 4,332 |
| `render_us`, average | 1,372 | 1,256 | 1,153 |
| frame `dur_us`, average | 22,286 | 22,593 | 17,730 |
| bytes, average | 492,233 | 491,572 | 491,572 |

This is the adversarial end of the range, 67 percent churn, and it is where the
adapted revision separates from both others: it encodes 11 percent faster than HEAD
and 12 percent faster than the landed shape, and emits byte-for-byte what the landed
shape emits. `emit` is the term that turns, because HEAD compares all 72,834 cells in
the diff while the adapted revision compares none: every cell is either skipped or
explicitly marked for update.

## Reproduce

```bash
# ruler, current tree
perf/split_probe.sh prismata 366 199 15 0.06 moss

# three-way: one binary per revision, alternated in one session.
# The test binary's file name hash depends on crate metadata, not on source, so
# copy it out before each rebuild.
cargo test --release --no-run --message-format=json \
  | jq -r 'select(.executable!=null)|select(.target.name=="ascii-renderer")|.executable' | sort -u
cp target/release/deps/ascii_renderer-1ac34f855c6cd0b7 /tmp/probe-before
#   ... apply the change, rebuild, cp to /tmp/probe-after ...
ASCII_SPLIT_REPS=15 /tmp/probe-before perf_split_probe --ignored --nocapture --test-threads=1
ASCII_SPLIT_REPS=15 /tmp/probe-after  perf_split_probe --ignored --nocapture --test-threads=1

# suite
cargo test --release
cargo test --release --test snapshot_modes --test 0_mode_generator --test 1_render_trace --test 2_input_replay

# live
scripts/13_e2e.sh --headless --mode prismata --case max-400x200
```
