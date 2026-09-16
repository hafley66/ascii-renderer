# Encoder split probe: ruler, and step 2 before/after

Step 0 and step 2 of `perf/16_LARGE_RENDER_PLAN.md`. Step 2 makes the encoder stop
converting cells that did not move. This file holds the ruler, the controlled
before/after, the byte-identity evidence, and the live pipeline numbers.

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

First run (landed revision, prismata 366x199 = 72,834 cells, moss, seed 42,
dt 0.06, 15 reps):

| stage | median us | min us | convert us | emit us | bytes | cells changed | cells skipped | runs |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| render (whole mode) | 1259.2 | 826.5 | - | - | - | - | - | - |
| blank grid build + row fill | 58.1 | 53.7 | - | - | - | - | - | - |
| encode full repaint | 2263.4 | 2162.0 | 1319.9 | 938.5 | 78,021 | 72,834 | 0 | 199 |
| encode delta over one dt step | 788.6 | 687.5 | 444.7 | 312.7 | 25,421 | 2,806 | 67,204 | 2,476 |
| encode identical frame | 279.1 | 260.0 | 231.0 | 44.8 | 0 | 0 | 72,834 | 0 |

Section 1 of the plan reports 2013 us render, 3206 us full encode, 2087 us delta
and 1992 us identical for the same grid. Those were single-shot measurements taken
once per size on a loaded host; the probe reports a median over reps after a warm-up
rep and keeps a min column, which is why its medians sit 15 to 40 percent lower. The
encoder rows are the stable ones: repeated runs move by a few percent. Render still
moves with host load, which is what the min column is for. Treat this table as the
ruler and section 1 as the earlier reconnaissance.

## Controlled before/after

Two binaries from one working tree, `HEAD` (before) and the landed revision (after),
run alternately in the same session so both see the same host load. 15 reps at
366x199, 9 reps at 800x240. Each cell shows two alternate runs.

| prismata 366x199 | HEAD | landed | ratio |
| --- | ---: | ---: | ---: |
| encode identical frame | 1791.7 / 1799.7 | 275.1 / 283.4 | 6.5x |
| encode delta over one dt step | 1986.0 / 1957.6 | 796.7 / 784.2 | 2.5x |
| encode full repaint | 2268.0 / 2221.3 | 2238.2 / 2350.5 | parity |

| prismata 800x240 (192,000 cells) | HEAD | landed | ratio |
| --- | ---: | ---: | ---: |
| encode identical frame | 4721.7 / 4707.0 | 741.9 / 749.2 | 6.3x |
| encode delta over one dt step | 5288.1 / 5155.1 | 2234.4 / 2371.5 | 2.3x |
| encode full repaint | 5873.3 / 5728.6 | 6189.9 / 6141.1 | parity |

| gem-aetherium-2 366x199 | HEAD | landed | ratio |
| --- | ---: | ---: | ---: |
| encode identical frame | 1565.0 | 238.4 | 6.6x |
| encode delta over one dt step | 1937.0 | 1200.9 | 1.6x |
| encode full repaint | 1966.1 | 2137.5 | parity |

Bytes, changed cells and runs are identical in every row above, on both revisions.
Skipped cells on the landed revision are 0 for a full repaint, 176,123 of 192,000
for the 800x240 delta, and every cell for an identical frame.

## Byte identity

- Probe: the three encode rows report the same `bytes`, `cells changed` and `runs`
  on both revisions across 2 modes and 2 grid sizes (rows above).
- Long sequence: `morph::iterate_frame_tests::gem_bad_roll6_ansi_regression`,
  60 frames of `gem-aetherium-2`. The test is red at HEAD, and for a reason that
  predates this work: it expects 4,748,083 foreground bytes and observes 0, so it
  never asserted what it intended. Its observed totals are still an exact
  comparator: `(5,124,966, 357,555, 0, 2,019,066, 2,976,090, 0)` on HEAD and on the
  landed revision.
- `tests/snapshot_modes.rs`: 190 passed, 1 failed (`polytope_seed_42`), and the same
  single failure occurs with the change stashed, so it is pre-existing and
  release-only.
- Two new encoder tests in `src/gridio.rs` cover the two ways a skip can go wrong: a
  cell that changes, rests, then returns to its earlier glyph must be repainted, and
  a resting blank cell whose foreground changes must emit nothing.

## Live pipeline

E2E `max-400x200` (`scripts/13_e2e.sh --headless --mode prismata --case
max-400x200`), prismata at 366x199 with all twelve knobs at max, animation frames
only. Artifacts `perf/results/e2e-1789593387274` (HEAD) and
`perf/results/e2e-1789595082362` (landed). The two runs draw different random knob
rolls, so frame sets are comparable in aggregate rather than row by row.

| live, 366x199 all knobs max | HEAD, 17 frames | landed, 18 frames |
| --- | ---: | ---: |
| cells changed, average | 49,697 | 49,126 |
| cells skipped, average | 0 | 19,270 |
| `convert_us`, average | 1,460 | 1,726 |
| `emit_us`, average | 3,352 | 3,122 |
| `encoding_us`, average | 4,864 | 4,893 |
| `render_us`, average | 1,372 | 1,256 |
| frame `dur_us`, average | 22,286 | 22,593 |
| bytes, average | 492,233 | 491,572 |

Read this as the adversarial end of the range, not the typical one. All twelve knobs
at max makes prismata churn 67 percent of its cells every frame, and at that churn
the per-changed-cell reference clone, the shadow store and the now-unpredictable
skip branch cost about as much as the skipped conversions save: `convert` rises 18
percent while total encode and frame duration stay at parity, and the terminal
consumer still owns about 90 percent of the frame. The probe rows above are the
other end: at 3.9 percent churn the delta encoder is 2.5x faster and a frame that
did not change at all is 6.5x faster, because a fully resting frame clones nothing.

## Reproduce

```bash
# ruler, current tree
perf/split_probe.sh prismata 366 199 15 0.06 moss

# before/after: one binary per revision, alternated in one session.
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
