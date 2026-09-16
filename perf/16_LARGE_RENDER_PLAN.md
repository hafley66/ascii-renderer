# Large-resolution render performance: recon and plan

Scope: what it costs to produce and deliver one frame at large grids, where that
cost sits now, and a phased plan to reduce it. "Large" here means roughly 48,000
cells and up (400x120), the sizes where a terminal is 366x199 to 400x200 cells
and where the demo's cadence target (frame interval p95 <= 33.34 ms, max <= 100 ms,
perf/0_E2E.md:47-49) starts to bind.

Two different problems share the name "large render", and they have different
answers:

1. One-shot render at a large grid (CLI, preview subprocess, knob sweep, snapshot).
   Here render plus encode is the entire cost, and render dominates.
2. Live animation at a large terminal (the demo). Here the terminal consumer
   dominates and the application's render plus encode is a minority share.

The plan below separates them and orders work by measured payoff over risk.

## 1. Recon: measured pipeline split

In-process, release, 366x199 = 72,834 cells, `moss`, seed 42, one 0.06 s step for
the delta column. Render numbers come from `IterateFrameRenderer` (the path the
demo uses); encode numbers from `AnsiFrameEncoder::encode`. `Cell` is 12 bytes.

| stage | default knobs | max knobs | 800x240 default | 800x240 max |
| --- | ---: | ---: | ---: | ---: |
| whole-mode render (5 layers) | 2013 us (27.6 ns/cell) | 2552 us | 5402 us (28.1 ns/cell) | 6322 us |
| blank fill inside `IterateFrameRenderer` | 35 us | 37 us | 100 us | 99 us |
| encode, full repaint | 3206 us / 78,091 B | 3710 us / 113,371 B | 8322 us / 201,365 B | 9365 us / 291,735 B |
| encode, delta over one 0.06 s step | 2087 us / 2,906 cells / 26,076 B | 3736 us / 44,065 cells / 163,660 B | 5473 us / 8,509 cells / 76,816 B | 9919 us / 120,183 cells / 437,396 B |
| encode, **identical frame (0 cells changed)** | **1992 us / 0 B** | 2251 us | **5132 us / 0 B** | 5549 us |

Findings:

1. The encoder pays about **27 ns/cell of fixed scan even when nothing changed**
   (1992 us and 0 bytes at 366x199; 5132 us at 800x240). That is the same order as
   the entire mode render at the same size. Source: the target-population loop
   converts every cell unconditionally (`src/gridio.rs:258-264`, two
   `ratatui_color` conversions per cell, each a three-branch `cube_index` at
   `src/gridio.rs:319-335`), and ratatui then diffs all cells (`src/gridio.rs:270-278`).
2. Blank-fill is 35 us at 72,834 cells (1.9 percent of render). Not a target.
3. Render cost is linear: 26.6-27.6 ns/cell at 366x199 and 27.5-28.1 ns/cell at
   800x240, matching the 2000x1000 sweep (27.0 ns/cell).
4. Prismata is already **lean in delivered bytes** for its size: a full frame is
   78,046 B at 366x199 against 127,742 B (`gem-aetherium-2`), 137,581 B
   (`illuminarium`), 89,961 B (`chladni`) and 181,586 B (`terminal-stress`) at the
   same size and theme. The mode's foreground is band-driven, so rows keep long
   attribute runs, and `runs` reports one run per row (199) on a full frame.
5. The dominant live term is the terminal, not the application. Prior measurement
   at 400x200 under samply: scene 1.109 ms, ANSI encode 2.160 ms, **terminal
   delivery and PTY backpressure 37.469 ms** (92 percent of frame work), against a
   60.08 ms frame interval (perf/results/15_samply_animation.md:30-35). The
   consumer cost is iTerm2 attributed-string construction and drawing
   (`PTYTextView drawRect` -> `iTermTextDrawingHelper`, 4,264 of 4,952 main-thread
   samples, perf/results/15_samply_animation.md:68-75). At 400x200 the PTY accepts
   about 1,024 B per write, 78.9 percent of write attempts block, and the worker
   spends 576 ms of a 595 ms interval waiting on output capacity
   (perf/results/17_pty_work.md:76-83).

So at 366x199 the application spends about 5.7 ms per frame (2.0 render + 2.1-3.7
encode) against a live interval of 14 to 19 ms in the headless driver and about
40 ms under iTerm at 400x200. Rendering is roughly one third of the application's
own cost and a low single digit share of the live frame under a GUI terminal.

## 2. Where the mode sits against the ecosystem

From `perf/results/SUMMARY.md` (2000x1000, 2,000,000 cells, 1 s per run), baseline
ms = 1000 / fps, ns/cell = ms * 1e6 / 2e6:

| mode | baseline ms | ns/cell |
| --- | ---: | ---: |
| mahoraga-5 | 111.1 | 55.6 |
| cosmograph | 62.5 | 31.3 |
| **prismata** | **54.1** | **27.1** |
| gem-aetherium | 47.6 | 23.8 |
| hyperloom | 43.5 | 21.7 |
| qwen-cathedral | 26.3 | 13.2 |
| poincare | 19.2 | 9.6 |
| chladni | 9.3 | 4.6 |
| pendulum-wave | 6.6 | 3.3 |
| illuminarium | 3.9 | 2.0 |
| astrolabe | 1.2 | 0.6 |

Prismata is in the slowest quartile for per-cell cost. The modes below it are not
doing the same job: `chladni` and `pendulum-wave` evaluate separable axes or a
recurrence instead of a 2D transcendental field, and `illuminarium` caches a
time-invariant background. Techniques worth copying, with the source pattern:

- Separable 1D coordinate buffers, so the inner loop is multiplies instead of trig
  (`src/chladni.rs:247-258`).
- Complex recurrence to advance a phase one step per row instead of a `sin_cos` per
  cell (`src/pendwave.rs:509-519`).
- Time-invariant geometry cached in a thread-local and blitted per frame
  (`src/modes/_30_illuminarium.rs:955-976`).
- Row-parallel shading gated by grid size, `width * height >= 65_536`, with
  `with_min_len(16)` and serial rows below the gate
  (`src/modes/_50_gem_aetherium_2.rs:493-497`). `rayon` is already a dependency
  (Cargo.toml:31).
- Scanline-incremental lattice interpolation (`src/modes/_50_gem_aetherium_2.rs:26-90`).

## 3. Plan

Each step states the lever, the measured basis, the expected gain, the risk, and
the check that proves it. Steps 0-3 are byte-identical in output by construction,
which is what makes them safe to land independently.

### Step 0: make the split measurable before changing anything

Lever: none, this is the ruler. The numbers in section 1 came from a throwaway
bench that has been removed; without it, every later step is unmeasurable in the
same units.

Change: promote it into an ignored, env-driven probe next to the existing knob
sweep, e.g. `ASCII_SPLIT_MODE` / `ASCII_SPLIT_W` / `ASCII_SPLIT_H`, printing
render, blank-fill, full, delta and unchanged-frame encode at one size, plus
`runs` and bytes. Record the first run under `perf/results/`.

Check: the probe reproduces section 1's table within noise on the current commit,
and `cargo test` is unchanged (the probe is `#[ignore]`d).

### Step 1: row-parallel field and paint passes (biggest render lever) - LANDED

Lever: the field pass is embarrassingly parallel across rows. Nothing is shared
between rows: no RNG, no cross-row reads, no blending, and every row writes a
disjoint slice of the scratch and of the grid.

Landed in `src/modes/_53_prismata.rs` only: `shade_rows` (four paint passes) and
`fill_rows` (field pass) run one shared row body through
`par_iter_mut().enumerate()` and `par_chunks_mut(w).enumerate()`, both with
`with_min_len(16)`, above the gate, and through the serial iterator below it. Rows
write disjoint slices and read only immutable inputs, so both paths are
bit-identical by construction. The thread-local scratch stays outside the parallel
region: the parent borrows it once and the pool takes a slice of it, so no
re-entry.

Gate: 16,384 cells, not the 65,536 that `_50_gem_aetherium_2.rs` uses. The
crossover is a property of the mode's body, so it was measured rather than
inherited (release, 12 threads, pristine tree against an always-parallel build of
the same code, same session, whole-mode render, FORM 0/dialect 0 then FORM 4/dialect 3):

| grid | cells | serial us | parallel us | winner |
| --- | ---: | ---: | ---: | --- |
| 80x24 | 1,920 | 52.8 / 139.0 | 59.9 / 152.6 | serial |
| 110x36 | 3,960 | 107.2 / 287.5 | 255.1 / 356.4 | serial 2.4x |
| 96x60 | 5,760 | 155.6 / 418.0 | 338.2 / 492.8 | serial 2.2x |
| 128x64 | 8,192 | 219.4 / 599.1 | 342.3 / 478.2 | splits by FORM |
| 160x64 | 10,240 | 285.2 / 744.8 | 393.8 / 527.5 | splits by FORM |
| 160x80 | 12,800 | 345.3 / 926.0 | 426.2 / 581.2 | splits by FORM |
| 200x80 | 16,000 | 431.1 / 1156.5 | 452.0 / 659.2 | tie, then 1.75x parallel |
| 240x100 | 24,000 | 663.7 / 1718.2 | 518.9 / 787.4 | parallel 1.28x / 2.18x |
| 400x120 | 48,000 | 1312.8 / 3431.9 | 756.5 / 1332.4 | parallel 1.74x / 2.58x |
| 366x199 | 72,834 | 1990.4 / 5217.4 | 825.1 / 1815.8 | parallel 2.4x / 2.9x |

The cheapest FORM crosses at about 16,000 cells and the costliest at about 8,000,
so 16,384 is the lowest power of two at which every FORM is a measured win or tie.
Below it serial still wins outright on both forms at 5,760 cells and under, which
is why the gate was not lowered further to chase the 8,000-16,000 band.

Measured result, whole-mode render (same bench, both trees):

| grid | before | after | speedup | ns/cell before to after |
| --- | ---: | ---: | ---: | --- |
| 400x120 | 1312.8 us | 756.5 us | 1.74x | 27.35 to 15.76 |
| 366x199 | 1990.4 us | 825.1 us | 2.41x | 27.33 to 11.33 |
| 2000x1000 | 53.7 ms | 11.0 ms | 4.90x | 26.87 to 5.49 |

Knob sweep receipt (`perf/knob_sweep.sh`, IterateFrameRenderer, dt 0.06, moss):

| grid | before, baseline | after, baseline | before, worst knob | after, worst knob |
| --- | ---: | ---: | ---: | ---: |
| 400x120 | 1.34 ms, 746 fps | 0.84 ms, 1192 fps | 1.62 ms (FORM=6) | 0.87 ms (GRAIN=1) |
| 2000x1000 | 54.25 ms, 18.4 fps | 11.34 ms, 88.2 fps | 61.85 ms (FORM=6) | 12.34 ms (FORM=6) |

The field layer itself went from 49.84 ms to 8.10 ms at 2000x1000 FORM=6 (6.2x) and
its share of the frame from 80.6 to 67.9 percent, so the remaining cost is the four
paint passes rather than one hot layer.

Check, all green:

- Bit-identity: 15 grid/knob rows rendered by the parallel build hash identically to
  the pristine serial build (FNV over every cell's glyph, fg and bg), at 400x120,
  512x256 and 2000x1000 as well as below the gate; the same 12 rows were then
  re-hashed in a second controlled build against the serial tree with identical
  results. The committed snapshots are unchanged and the 4 in-module tests plus 3
  CLI snapshot tests pass without re-accepting anything.
- `prismata_boundaries_and_hostile_inputs` stays green, including the 0x0, 1x1 and
  NaN-clock cases; `w == 0` returns before any `chunks_mut` call, which would panic.
- The knob sweep shows the expected ns/cell at both grid sizes (table above).
- `every_native_mode_has_layer_timers` passes with the same five layers, and their
  shares still sum to the frame.

Known cost, accepted and confined: below the gate the serial branch's field body
runs through a row slice instead of the original indexed double loop, which costs
about 13 percent of the field layer and nothing measurable in the four paint
layers (110x36: field 100.0 to 113.3 us, detail 17.8 to 18.8, ink, ground and dust
flat). An indexed-row serial variant measured the same as the chunked one (121.1
against 120.7 us), so this is the abstraction and not `chunks_mut` padding. In
absolute terms it is 8 us at 80x24, 13 us at 3,960 cells and 60 us at 16,000 cells,
and it is the price of keeping one copy of the row body for both paths. Grids at or
above the gate, which is every grid the mode is judged on at large size, gain
1.74x to 4.9x.

Live impact, measured through the real pipeline (E2E `max-400x200`, prismata at
366x199 with all 12 knobs at max, 18 animation frames): per-frame `render_us` has a
median of 1315 and a mean of 1338, against 2552 us for the same knob set in this
plan's pre-change recon bench, so the change removes about 1.2 ms of application
work per frame. The frame interval over those frames had a median of 19.8 ms
(12.1 to 29.3), and the full frame is 622,323 bytes encoded in 6175 us, which is
where the frame time actually goes. The E2E case passed all 9 checks
(`actual-demo-startup`, `mode-search-and-preview`, `exact-animation-inputs`,
`terminal-cell-motion`, `knob-reaches-render`, `random-knob-diversity`,
`q-returns-to-demo`, `exit-and-terminal-restoration`,
`profile-includes-render-worker`); it was run as a single case, so the suite's other
cases are unexecuted (`complete_suite: false`).

This is CPU headroom, not frame rate: the render is now about 7 percent of a 19.8 ms
frame instead of about 13 percent, and the terminal consumer still owns the rest.
The user-facing value of step 1 is the one-shot large render (1.74x at 400x120,
4.9x at 2000x1000) and the room it leaves for steps 2 to 4.

Rollback: the gate is one constant; reverting to the serial branch is a one-line
change.

### Step 2: stop converting unchanged cells in the encoder (biggest shared lever)

Lever: the encoder's fixed 27 ns/cell scan is paid on every frame regardless of how
little changed, and is the same order as the whole render at large sizes.

Basis: 1992 us for an identical frame, 0 bytes emitted, at 366x199; 5132 us at
800x240. The work is the unconditional target-population loop
(`src/gridio.rs:258-264`) plus ratatui's full-grid diff (`src/gridio.rs:270-278`).

Change, inside `src/gridio.rs`: keep a raw `Vec<Cell>` copy of the art grid (12
bytes per cell) alongside the ratatui buffers, and write into `current` only for
cells whose raw `Cell` differs from the copy. The diff then sees exactly the same
changed set, so **emitted bytes are unchanged**; the conversion cost is skipped for
the unchanged majority.

Expected: identical-frame encode 1992 us to roughly 300 to 500 us at 366x199, and
5132 us to roughly 800 us to 1.3 ms at 800x240; the same absolute saving applies to
every delta frame and to every other mode for free.

Risk: an extra 12 bytes per cell of memory (874 KB at 366x199, 2.3 MB at 800x240)
and one more pass over the grid; the `full_repaint` path must still populate every
cell (or keep `AlwaysUpdate`).

Check: `morph::iterate_frame_tests::gem_bad_roll6_ansi_regression` asserts exact
ANSI totals for 60 frames of `gem-aetherium-2` and must pass unchanged, which is a
strong byte-identity guard; plus the animation encoder unit tests in
`src/gridio.rs`, the mode snapshot suite, and the step 0 probe showing the saving.

Rollback: the raw copy is additive; the loop change is localized to `encode`.

### Step 3: buffer hygiene on the frame path

Lever: two avoidable full-buffer operations per frame at large sizes.

Basis: `output.clear(); output.push_str(std::str::from_utf8(&self.bytes)...)`
validates and copies the whole payload (`src/gridio.rs:280`, 78 KB here but 290 KB
at 800x240 with max knobs, and up to ~600 KB in the prior 400x200 study), and the
worker prepends with `frame_buffer.insert_str(0, ...)`, which memmoves the entire
frame buffer (`src/morph.rs:1451`, `src/morph.rs:1530`).

Change: have `encode` target a `Vec<u8>` (or hand out `&[u8]`) so no UTF-8
validation and no copy is needed, and reserve the prefix once, or write the prefix
into a small separate buffer that the relay concatenates.

Expected: roughly 0.2 to 0.6 ms per frame at large sizes, scaling with payload.

Risk: touches the String-typed callers in `src/morph.rs` and the CLI paths; keep
the change mechanical and let the existing byte-count tests guard it.

Check: the same byte-count regression test, the E2E `workflow` and `max-400x200`
cases (terminal output must stay identical), and the step 0 probe.

### Step 4: cache the fold geometry across frames (mode-local, animation only)

Lever: the fold is 11.3 ns/cell of the 27, and it is time-invariant apart from the
final rotation. Measured decomposition of the field pass at 2000x1000 (warm buffer,
throwaway bench): rings with the default fold and warp 23.1 ns/cell, rings with
`SYMMETRY=1` 11.8, rings without warp 17.8, so the fold costs 11.3 and the warp 5.3.

Basis: the same numbers, plus the field pass being 80.5 percent of the frame.

Change: apply the domain warp in folded space rather than before the fold, which
makes the folded angle `af` and the radius `r` functions of position only; then
cache `(af, r)` per cell in a thread-local keyed by `(w, h, seed, symmetry)` and
per frame compute only `cos`/`sin` through the table. Resize or knob changes
invalidate the key.

Expected: 27 ns/cell to roughly 17 to 19 ns/cell serial, on top of step 1 rather
than instead of it.

Risk: a derived cache is state; it must be a pure function of the key or
determinism breaks, and moving the warp changes the art slightly (it must be
inspected and the snapshots re-accepted, unlike steps 1-3).

Check: snapshots re-inspected and re-accepted, `prismata_random_knob_rolls_land_in_distinct_worlds`
still passing, and the probe showing the delta.

### Step 5: decide about the terminal, with a measurement gate (investigation)

The remaining live cost is outside the application. Prior work already rejected the
two obvious delivery tricks on evidence, so this step is a decision gate, not a
code change:

- Unicode REP (`\e[Nb`) cut bytes 17.2 percent but raised acknowledgement time 21.4
  percent and write time 7.4 percent (perf/results/13_controlled_terminal.md:30-36).
- Gap-cost pricing of horizontal cursor commands cut bytes 1.9 percent but raised
  cursor commands from 4,762 to 5,519 and DSR time 12.4 to 17.3 percent
  (perf/results/18_gap_cost.md:1-6, 33-36).
- The general lesson recorded there: fewer bytes with more commands is slower.

What to measure instead, before changing anything: attribute-command counts per
frame (foreground and background SGR groups, cursor moves) for prismata against the
same-size references, with the user's actual terminal (iTerm2, and tmux inside it),
using the E2E `max-400x200` case. Prismata's full frame is 78 KB against 128-182 KB
for the reference modes at 366x199, so the expected outcome is that the mode is
already at or below the field's delivery cost, and that the honest levers are the
churn knobs rather than the pipeline.

If the measurement shows churn is the issue, the response is product-level and
already measurable: `FORM` spans a 4 to 7x range in cells repainted per tick (rings
7.5 percent, ripple 49.6 percent, cells 52.9 percent at defaults; the same
measurement is recorded in `briefs/prismata.md`), `FLOW` is linear (0.5 to 7.5
percent, 0.125 to 2.1 percent), and dialect spans 2 to 3x (stipple 2.4 percent,
weave 8.1 percent). Options: leave it, document it, or add a churn-oriented knob
that quantizes the animated phase per tick without changing the still frame.

## 4. Constraints and non-goals

- Never raise the probe guard ceilings to obtain a pass (perf/0_E2E.md:52-56; the
  rule is repeated in perf/results/15_samply_animation.md:21 and 16_pacing.md:53).
- Steps 1-3 must not change a single emitted byte; that is the acceptance bar, not
  an aspiration. Art changes are confined to step 4, whose snapshots are re-inspected.
- Keep the serial path for grids below the cell gate, so previews and small
  snapshots keep their current behaviour. The gate for this mode is 16,384 cells,
  its measured break-even, not the 65,536 that `_50_gem_aetherium_2.rs` uses for a
  cheaper body.
- Do not re-propose REP, gap-cost pricing, or pyte-based cadence benchmarking; each
  is measured and rejected.
- Do not add retained simulation state to the mode. The step 4 cache is derived
  geometry, keyed and invalidated, not simulation.
- GUI iTerm E2E stays out; it cannot run in this environment (HTTP 401 on the
  iTerm2 API), so live evidence comes from the headless `--headless` cases.

## 5. Expected end state

Step 1 is landed with actuals: at 366x199 the whole-mode render is 1990 to 825 us
(2.4x) and at 2000x1000 it is 53.7 to 11.0 ms (4.9x), measured through the knob
sweep as 54.25 to 11.34 ms and 1.34 to 0.84 ms at 400x120. Steps 2 and 3 are still
projections: at 366x199 application-side per-frame CPU from about 5.7 ms to roughly
1.2 to 1.6 ms, and at 2000x1000 the encoder proportionally. On the one-shot path
(problem 1) step 1 alone is already a 1.7 to 4.9x improvement in wall time for a
large static render.

Under a GUI terminal at 400x200 the live cadence improves only modestly, because the
measured consumer cost was 37.5 ms of a 40 ms frame; in the headless E2E the render
fell from about 2.5 to about 1.3 ms inside a 19.8 ms frame. The value of steps 1-3
there is CPU headroom and cadence stability rather than frame rate, and the plan
says so rather than promising a number it cannot defend. Step 5 exists to turn that
statement into a decision with data.
