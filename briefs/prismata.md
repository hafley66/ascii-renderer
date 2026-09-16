# prismata

Subject: one kaleidoscopic interference field, read through seven structural forms and five engraving dialects. A seeded fold plus a small harmonic bank produce every frame from explicit inputs, so a random knob roll changes the visual world rather than the shading of the same picture.

What moves and why: the fold axis rotates with time, every wave phase advances with `FLOW`, ripple sources orbit, and the palette channels stay fixed per frame. Nothing is retained between frames, so a frame is reproducible from `(seed, dims, theme, t, knobs)` alone.

## Knobs

| knob | range | default | what it changes |
| --- | --- | --- | --- |
| FORM | 0..6 step 1 | 0 | structure: rings, plasma, spiral, lattice, cells, moire, ripple |
| DIALECT | 0..4 step 1 | 0 | ink: engrave, stipple, mosaic, hatch, weave |
| GLYPHS | 0..2 step 1 | 0 | glyph set: density ramp, blocks, marks |
| SYMMETRY | 1..12 step 1 | 6 | kaleidoscope sectors, 1 disables the fold entirely |
| FREQ | 1..12 step 0.25 | 4 | wave pitch, shared by radial and directional forms |
| WARP | 0..1 step 0.05 | 0.35 | domain warp before the fold |
| DENSITY | 0.05..1 step 0.05 | 0.6 | ink coverage: shifts the whole field and also the stipple and hatch thresholds |
| CONTRAST | 0..1 step 0.05 | 0.4 | band sharpening, contour line width |
| CHROMA | 0..1 step 0.05 | 0.6 | monochrome engraving through to spectral bands |
| GLOW | 0..1 step 0.05 | 0.45 | ground bloom, the field showing through the background |
| FLOW | 0..2 step 0.05 | 0.5 | drift rate |
| GRAIN | 0..1 step 0.05 | 0.25 | hash dither and dust sparkle |

Positional order: form dialect glyphs symmetry freq warp density contrast chroma glow flow grain.

Randomized rolls: the first four knobs are categorical, so a roll picks 1 of 7 structures, 1 of 5 inks, 1 of 3 glyph sets and 1 of 12 sector counts before the continuous knobs modulate the result. `prismata_random_knob_rolls_land_in_distinct_worlds` renders twelve `rand_knob` rolls and requires every pair to differ by more than 700 of 3960 glyphs.

The fold is a real structural term, not a post effect: `SYMMETRY s` maps the plane into s wedge sectors and the field is a function of the folded position, so the sector count reshapes every form. The rings form carries a folded-angle term so the knob is visible there too.

## Glyph families

1. Engrave: `DENSITY` ramp for shading, field-aligned `- | / \` contour lines on band boundaries.
2. Stipple: halftone dots from a per-glyph-set dot ramp, thresholded against a per-cell hash.
3. Mosaic: quantized block ramp for the value, per-band background for the pigment.
4. Hatch: gradient-direction strokes, cross-hatched above a coverage threshold.
5. Weave: `- | +` plaid from the primary and secondary channels, with an over/under accent.

## Measured cost

Cost is linear in grid cells and split by a cell gate. At or above 16,384 cells the
five layers shade rows on rayon, so per-cell cost falls as the grid grows: 17.5 ns/cell
at 400x120, 11.3 at 366x199 and 5.7 at 2000x1000 on 12 cores. Below the gate the
serial path stays, at 27 to 31 ns/cell plus a small cost for routing the field body
through a row slice.

| grid | cells | before | after | per cell after | fps after |
| --- | ---: | ---: | ---: | ---: | ---: |
| 400x120 | 48,000 | 1.34 ms | 0.84 ms | 17.5 ns | 1192 |
| 2000x1000 | 2,000,000 | 54.1 ms | 11.3 ms | 5.7 ns | 88.2 |

Both rows are `perf/knob_sweep.sh prismata <w> <h>` (release, theme moss, dt 0.06);
the same sweep at 366x199 and 512x256 over a plain render is 1.99 to 0.83 ms and
3.57 to 1.43 ms. Worst knob at 2000x1000 is `FORM=6` (ripple) at 12.34 ms, 1.09x
baseline; at 400x120 it is `GRAIN=1` at 0.87 ms. Per-layer share of the worst frame
at 2000x1000 is now field 67.9%, detail 8.0%, ink 6.1%, ground 5.0%, dust 4.6%, so
the four paint passes are no longer negligible against the field.

The gate is 16,384 cells rather than the 65,536 used by `_50_gem_aetherium_2.rs`,
because the break-even belongs to the body: the cheapest FORM crosses at about
16,000 cells and the costliest at about 8,000, so 16,384 is the lowest power of two
where every FORM is a win or tie. Below 5,760 cells serial wins outright, and the
crossover table is in `perf/16_LARGE_RENDER_PLAN.md`.

Where the remaining time goes, from an isolated field-pass benchmark (2000x1000,
warm buffer, serial): rings with the default fold and warp 23.1 ns/cell, rings
without the fold (SYMMETRY=1) 11.8, rings without warp 17.8, so the dihedral fold
costs 11.3 ns/cell and the domain warp 5.3. The `cells` form costs 73 ns/cell
because nine hash candidates and two nearest-neighbour updates are inherent to an
exact crackle.

Optimizations that landed, each measured at 2000x1000:

1. `libm hypot` replaced by `sqrt(x*x + y*y)` in the field path (coords are bounded, so the scaling in hypot is unneeded).
2. A single 4096-entry sine table, built once, replaces every per-cell `sin`/`cos`. Lookup is a multiply, a truncating convert, a mask and a load. Cost it removed: field 92.7 ms to 56.0 ms.
3. The fold's `rem_euclid` (a libm `fmodf` call) became a multiply, `floor` and subtract against a precomputed reciprocal.
4. `cells` orders candidates by squared distance and takes only the two square roots it needs, and its two-nearest update is branchless `min`/`max` instead of a compare chain that mispredicts per cell.
5. A hand-written polynomial `atan2` was tried and **rejected**: macOS `atan2f` measured 20.7 ns/cell against 23.1 for the polynomial, so the library call stays.
6. Row-parallel shading above the gate. The row closures write disjoint rows and read only immutable inputs, so parallel output is bit-identical to serial: 15 grid/knob rows re-rendered by the parallel build hash identically to the pristine serial build, at and above the gate as well as below it.

Whole-mode effect: baseline 82.8 ms to 54.1 ms (1.53x) from the scalar work and then
to 11.3 ms (4.8x against the original) at 2000x1000, with the field pass down 6.2x
on the worst knob. The scalar optimizations changed snapshots by 0.5 to 0.8 percent
of glyphs, all at band boundaries; the parallel work changed none.

Live demo measurement from the workflow case: 126x39 grid, `strategy: iterate`,
`render_us` 624, `changed_cells` 4914 on the sampled frame. That run profiles under
samply, so its `render_us` values are inflated; the same run also rendered the legacy
`party` mode at 3599 us in the same demo, and the demo's own frame interval was 16 to
19 ms, so the mode was not the limiter at that size.

Live in the `max-400x200` case after the parallel work (366x199, every knob at max,
18 animation frames): `render_us` median 1315, mean 1338, against 2552 us for the
same knob set in the pre-change recon bench. Frame interval median 19.8 ms over those
frames, full frame 622,323 bytes encoded in 6175 us, so the render is now about 7
percent of the frame and the encoder and terminal still own the rest.

## Large resolution and per-tick churn

The mode's render is linear in cells, and row-parallel above the gate, so the mode
itself is not what makes a big terminal slow. Measured live at 366x199 (72,834
cells) in the demo under samply before the parallel work, the frame interval was 14
to 19 ms against a render cost of about 2.0 ms (72,834 cells at 27 ns), with the
ANSI encoder taking 4.1 to 5.4 ms and emitting 0.4 to 0.6 MB per frame. The same
grid now renders in about 0.83 ms, so the encoder and the terminal are further ahead
than those numbers suggest. Nearly the whole frame repaints every tick at max-knob
settings, which is what the deltas cost.

Repainted cells per 0.06 s tick at 366x199 with everything else at default, counted on rendered `Cell` equality:

| knob | value | cells repainted |
| --- | --- | ---: |
| dialect | engrave (default) | 7.5% |
| dialect | stipple | 2.4% |
| dialect | mosaic | 6.1% |
| dialect | hatch | 3.8% |
| dialect | weave | 8.1% |
| form | rings (default) | 7.5% |
| form | plasma | 35.3% |
| form | spiral | 10.0% |
| form | lattice | 32.0% |
| form | cells | 52.9% |
| form | moire | 29.6% |
| form | ripple | 49.6% |
| FLOW | 0.5 (default) | 7.5% |
| FLOW | 0.25 | 3.9% |
| FLOW | 0.125 | 2.1% |

Churn is linear in FLOW and spans 4 to 7x across forms, so a large terminal can be made cheap with a knob change rather than a code change: rings or spiral with stipple or hatch repaints 2 to 4 percent of the frame per tick, while ripple or cells repaint half of it. The `max-400x200` case drives every knob to its maximum, which is why that case repaints 44,000 to 72,000 of 72,834 cells per tick and lands in the same byte band as the `terminal-stress` mode at the same size.

## Verification

- `cargo test --bin ascii-renderer prismata`: 4 passed. Snapshots: seed 42 still, seed 42 at t=5, a seven-form gallery, a five-dialect gallery.
- `cargo test --test snapshot_modes prismata`: 3 passed (default CLI frame, a ripple mosaic at t=9, a pinwheel mosaic driven by positional knobs).
- `cargo test`: 452 passed, 2 failed, 15 ignored. Both failures (`gridio::ansi_frame_tests::animation_encoder_collapses_adjacent_rgb_levels`, `morph::iterate_frame_tests::gem_bad_roll6_ansi_regression`) reproduce on the pristine tree with this work reverted, so they are pre-existing and unrelated. No `.snap.new` files are left in any snapshot directory.
- Bit-identity of the parallel path, the acceptance bar for the row-parallel step: a throwaway release bench hashed every cell's glyph, fg and bg for 15 grid/knob rows (80x24, 110x36, 400x120, 512x256, 2000x1000 and 366x199, forms 0/4/6 with dialects 0/3/4) and produced identical hashes against the pristine serial tree, in two independent controlled builds. Rows write disjoint slices and read only immutable inputs, so this holds by construction as well as by measurement.
- Break-even gate: measured by building the same source with the gate forced to 1 and to `usize::MAX`, then timing both across ten grid sizes; the table and the choice of 16,384 are in `perf/16_LARGE_RENDER_PLAN.md`.
- Headless demo E2E at large resolution, `scripts/13_e2e.sh --headless --mode prismata --case max-400x200`: **passed**, with `actual-demo-startup`, `mode-search-and-preview`, `exact-animation-inputs`, `terminal-cell-motion`, `knob-reaches-render`, `random-knob-diversity`, `q-returns-to-demo`, `exit-and-terminal-restoration` and `profile-includes-render-worker` all green at 366x199, every knob at max. The run reports `complete_suite: false` because the other E2E cases were not requested, so they are unexecuted. Guard rows end in `completed` with no breaker.
- Headless demo E2E before the parallel work, `scripts/12_test_e2e.py --headless --case workflow --mode prismata` under `scripts/5_probe_guard.py`: startup, mode search and preview, save, `exact-animation-inputs` and `terminal-cell-motion` all passed with `mode: prismata` and `strategy: iterate`. The case then fails at a later iTerm-only step (`import iterm2`, HTTP 401 without the iTerm2 API), which is a harness limitation in this environment. Guard rows show no threshold breach: owned 112 MiB of 256 MiB, watched growth 704 KiB of 256 MiB, 0.79 s of 15 s.
- Profile artifacts: the E2E run saves `profile.json.gz` under samply, and because the release binary carries no debug info the saved profile has no symbols (`meta.debug` false, functions reported as addresses), so it can only confirm that one region dominates. The `measure_layer` timers above are the usable hotspot source, and the demo's own `animation.ndjson` render records carry a layer table per frame.

Render commands:

```bash
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 42 prismata moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=5 ./target/release/ascii-renderer 42 prismata moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 42 prismata neon 2 2 1 8 5 0.3 0.6 0.5 0.8 0.5 | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 1701 prismata ember 4 3 0 7 6 0.5 0.7 0.6 0.5 0.6 | sed 's/\x1b\[[0-9;]*m//g'
```
