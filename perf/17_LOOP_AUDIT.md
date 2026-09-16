# Loop audit: every hot loop, its blowout class, and the tracing added

Scope: find every loop in the app that runs per cell, per row or per frame; rank
the blowouts (allocation, copy, per-cell transcendental, per-cell RNG, per-cell
hash); and add tracing where a hot path had none. The instrument is
`sprefa-extract`, not grep: its dataflow family answers the whole question in one
query, and the queries are recorded at the bottom so this is reproducible.

## 1. The instrument

```sh
extract fast --sqlite /tmp/ascii-fast.db $(ls src/*.rs src/modes/*.rs)
# Wrote /tmp/ascii-fast.db (1983339 rows)   real 0m4.177s
```

On this crate (107 Rust files) the facts that matter for a loop audit:

| fact | meaning | rows |
| --- | --- | ---: |
| `df_loop` | every loop, with the collection it iterates | 3,211 |
| `df_nest` | every call **inside** a loop, with nesting depth | 18,100 |
| `df_allocates` | functions that allocate | 515 |
| `site` | every call site with `callee` and `callee_path` | 34,608 |
| `unresolved` | edges the tool could not bind, with a reason | 19,281 |

Nesting depth is the per-cell tell. A double loop over the grid puts its body at
depth 2:

| depth | call sites | reads as |
| ---: | ---: | --- |
| 1 | 12,368 | per row / per item |
| 2 | 4,796 | **per cell** |
| 3 | 817 | per cell, inner |
| 4 | 110 | per cell, inner |
| 5 | 9 | per cell, inner |

`extract rename` and `extract move` exist too (both confirmed), which is what the
root `AGENTS.md` recommends for mechanical refactors.

## 2. Blowouts found

865 call sites inside nested (per-cell) loops are transcendental or RNG calls.
Ranked by file, then by function:

| sites | file | function | callee |
| ---: | --- | --- | --- |
| 17 | `cli_city.rs` | `cli_skyline` | `random_range` |
| 14 | `cli_city.rs` | `cli_metro` | `random_range` |
| 14 | `_34_gemini_astrolabe.rs` | `draw_gem_aetherium` | `sin` (+10 `cos`) |
| 14 | `_44_tideglass.rs` | `draw_tideglass` | `sin` |
| 13 | `cli_scenes.rs` | `cli_world2` | `random` |
| 12 | `_50_gem_aetherium_2.rs` | `draw_gem_aetherium_2` | `sin` (+9 `cos`) |
| 9 | `ink.rs` | `draw_ink` | `sin` |
| 9 | `modes_geo.rs` | `draw_spiro_tile` | `sin` (+9 `cos`) |
| 8 | `cli_forest.rs` | `cli_forest8` | `random_range` |
| 8 | `modes_geo.rs` | `draw_eyes3` | `powi` |
| 6 | `arboretum.rs` | `draw_dynamic` | `rem_euclid` |

Worst files by per-cell transcendental/RNG count: `cli_city.rs` 85,
`cli_forest.rs` 52, `modes_geo.rs` 43, `cli_scenes.rs` 40,
`_34_gemini_astrolabe.rs` 33, `modes_sky.rs` 27, `fable_1_trees.rs` 27,
`_50_gem_aetherium_2.rs` 25, `cli_basic.rs` 24, `_44_tideglass.rs` 22.

Three classes, in the order they pay:

1. **Per-cell RNG** (`random`, `random_range`, 323 sites). A PRNG draw per cell is
   both slow and hostile to vectorization; the fix is hash-based noise or a
   precomputed scratch, which is what `_53_prismata.rs` already does with `hash3`.
2. **Per-cell libm trig** (`sin` 219 sites, `cos` 74, `powi` 89). The mode that
   was measured hardest, `prismata`, was fixed with a 4096-entry sine table and
   went 92.7 ms to 56.0 ms at 2000x1000. The same table applies to
   `_34_gemini_astrolabe`, `_50_gem_aetherium_2`, `modes_geo`, `ink` and `modes_sky`.
3. **Per-cell `rem_euclid`** (49 sites, e.g. `arboretum::draw_dynamic`). This is a
   libm `fmodf` call per cell; `prismata` replaced it with a multiply, a `floor`
   and a subtract against a precomputed reciprocal.

Allocation inside loops is real but small on this corpus: the ranked list tops out
at `opts.rs` `push_str` 12 sites, then `clone` 8 each in `_43_azulejo.rs` and
`_48_astra_jurassic_park.rs`, `collect` 7 in `morph.rs`. Nothing here is an
O(cells) allocation blowout, which matches the pipeline measurement: encode costs
come from per-cell conversion and the diff scan, not from allocation.

## 3. Tracing added

### 3.1 Eight registered modes had no timers at all

`measure_layer` coverage was the largest blind spot: 8 of 24 in-process registered
modes had zero timers, so no hotspot table could be produced for them and the
existing gate could not catch it because it iterates `NATIVE_MODES`, a 65-name
sweep roster that omits 13 registered modes.

| mode | file | layers added |
| --- | --- | ---: |
| `tideglass` | `_44_tideglass.rs` | 7 |
| `chimera-shadow-garden` | `_45_chimera_shadow_garden.rs` | 7 |
| `volute` | `_46_volute.rs` | 4 |
| `astra-opus-1-chronofold` | `_47_astra_opus_1_chronofold.rs` | 8 |
| `astra-jurassic-park` | `_48_astra_jurassic_park.rs` | 8 |
| `astra-chaos-theory` | `_49_astra_chaos_theory.rs` | 7 |
| `terminal-stress` | `_51_terminal_stress.rs` | 4 |
| `gem-aetherium-3` | `_52_gem_aetherium_3.rs` | 8 |

Each wrap is behaviour-identical (`measure_layer` forwards to the closure with no
capture open) and every snapshot is unchanged.

Proof that the tables now exist:

```
perf/knob_sweep.sh terminal-stress 400 120 1    -> cells 89.2%, wave_table 0.4%, knobs 0.1%, glyphs 0.0%
perf/knob_sweep.sh gem-aetherium-3 400 120 1    -> bands 63.3%, rings 18.0%, composite 4.3%, instruments 1.6%
```

### 3.2 The encoder's N+1 split is now traced per frame

`AnsiFrameEncoder::encode` recorded one number for a phase that is really two:
the per-cell adapter loop (one ratatui write per cell, paid on every frame) and
the diff scan plus emission. `FrameEncodeStats` now carries `convert` and `emit`
durations, `FrameSample` and `FrameTotals` carry them, the periodic tracing
report logs `convert_us_avg/max` and `emit_us_avg/max`, and every
`animation.ndjson` frame record gains `convert_us` and `emit_us` beside the
existing `render_us`, `encoding_us`, `presentation_us`, `bytes`, `changed_cells`
and `runs`.

Step 2 of `perf/16_LARGE_RENDER_PLAN.md` adds one more counter on the same path:
`skipped`, the cells the encoder left alone because their raw `Cell` did not move.
It rides `FrameEncodeStats` to `FrameSample`, the tracing report
(`skipped_total/avg`) and the frame record, and it is what separates "the
conversion got cheaper" from "the conversion stopped happening".

The first live receipt already separates the two costs, from the E2E
`max-400x200` case at 366x199 with every knob at max (17 frames, medians
`convert` 1430 us, `emit` 3404 us, `encoding` 4869 us):

| frame | changed cells | convert us | emit us |
| ---: | ---: | ---: | ---: |
| 1 | 72,834 | 1429 | 3726 |
| 2 | 43,831 | 1578 | 3575 |
| 4 | 40,742 | 1430 | 3198 |
| 6 | 39,944 | 1459 | 3510 |

`convert` is flat at about 1430 us, 19.6 ns/cell, whether the frame repainted
72,834 cells or 39,944: the per-cell adapter loop is an unconditional O(cells)
tax, which is the N+1 problem stated as a number. `emit` is what responds to the
change count. Killing the unchanged-cell conversion (plan step 2) therefore
targets roughly 1.4 ms of every frame at this size, not the 3.4 ms.

### 3.3 A gate that covers the roster, not a subset

`perf_sweep::every_registered_mode_that_renders_in_process_has_layer_timers`
iterates `registered_modes()` and requires layer timers from every mode that
`IterateFrameRenderer::new` can build (24 today). It reports its coverage, so a
future run cannot pass by covering nothing. The original gate is untouched.

## 4. Verification

- `cargo test --release` after all of the above: **451 passed, 3 failed, 16
  ignored** - identical to the pre-change baseline, with the same three failures
  (`gridio::ansi_frame_tests::animation_encoder_collapses_adjacent_rgb_levels`,
  `morph::iterate_frame_tests::gem_bad_roll6_ansi_regression`, and
  `polytope::tests::snapshot_polytope_small`, which passes in debug and fails in
  release on untouched code because one glyph lands on the other side of a float
  threshold). With the new gate counted, 452 pass.
- No `.snap.new` anywhere; no snapshot was re-accepted.
- The eight instrumented modes' own snapshot tests pass unchanged.
- `perf/knob_sweep.sh` produced hotspot tables for `terminal-stress` and
  `gem-aetherium-3`, written to `perf/results/`.

## 5. Reproduce the audit

```sh
extract fast --sqlite /tmp/ascii-fast.db $(ls src/*.rs src/modes/*.rs)

# per-cell transcendental / RNG sites, ranked by file
sqlite3 /tmp/ascii-fast.db "
SELECT n._input_path, s.callee, COUNT(*) FROM df_nest n
JOIN site s ON s._input_path=n._input_path AND s.span__start=n.call__start AND s.span__end=n.call__end
WHERE n.depth>=2 AND (s.callee LIKE '%sin%' OR s.callee LIKE '%cos%' OR s.callee LIKE '%random%'
  OR s.callee LIKE '%powi%' OR s.callee LIKE '%rem_euclid%')
GROUP BY 1,2 ORDER BY 3 DESC;"

# allocation called inside a loop
sqlite3 /tmp/ascii-fast.db "
SELECT n._input_path, s.callee, COUNT(*) FROM df_nest n
JOIN site s ON s._input_path=n._input_path AND s.span__start=n.call__start AND s.span__end=n.call__end
WHERE s.callee LIKE '%collect%' OR s.callee LIKE '%clone%' OR s.callee LIKE '%push_str%'
GROUP BY 1,2 ORDER BY 3 DESC LIMIT 20;"
```

Two things the tool could not answer on this corpus, filed against the crate as
`extract-fast-slow-trait-divide` in `~/projects/hafley-rs`: fast mode leaves
`name` NULL on every CST definition node (so a call span cannot be attributed to
its enclosing function without re-reading source bytes), and the SCIP plane
carries no spans, so the two planes cannot be joined and fast's per-call-site
accuracy is not measurable today.
