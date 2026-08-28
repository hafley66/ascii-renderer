# PERF: arboretum frame path

`draw_arboretum` re-rendered the whole grove every frame. Everything except the
tree life cycle and grass sway is t-independent, so it was recomputed for
identical output ~60x/second.

## Results (release, 300 frames, t sweeping 0..18)

| grid   | before ms/frame | after ms/frame | delta |
|--------|-----------------|----------------|-------|
| 80x24  | 0.023           | 0.011          | -52%  |
| 150x50 | 0.070           | 0.024          | -66%  |

## Measured bottlenecks, ranked

Reproduce: `CARGO_TARGET_DIR=$PWD/target-perf cargo test --release perf_arboretum -- --ignored --nocapture`

| # | bottleneck | evidence | fix | before ms | after ms |
|---|------------|----------|-----|-----------|----------|
| 1 | Static base re-render every frame: sky stars, clouds, ground-line walk, ground fill (hsl_to_rgb + char roll per cell), fog band | `render_static` measured at 0.017 ms/frame (80x24) and 0.055 ms/frame (150x50), i.e. 75-79% of total frame cost | `render_static` result cached in a thread_local keyed by (seed, w, h, palette, knobs); per frame the base grid is copied in and only trees + undergrowth redraw | 0.023 / 0.070 | 0.011 / 0.024 |
| 2 | Fresh `vec![vec![Cell::blank(); w]; h]` allocation per frame in `iterate_grid`'s arboretum arm, then fully overwritten | the arm allocated a blank grid the draw immediately replaced | `render_arboretum_frame` owns allocation; on cache hit the base clone doubles as the frame buffer | (part of above totals) | included above |
| 3 | Per-tree scratch allocations (`nodes`, `leaf_cells`, `tips` Vecs, `TreeGenome` copy) inside grow_tree/grow_species every frame | remaining frame cost after fixes is 0.011 / 0.024 ms, dominated by the dynamic tree + undergrowth passes | not fixed; left on the table (see below) | -- | -- |

## Design notes

- rand 0.10's `StdRng` implements neither `Clone` nor any state snapshot, so
  the cached base cannot stash the rng mid-stream. Instead a cache miss
  records every dynamic-pass draw (the sequence is t-independent: stop
  placement, clearing flora rolls, undergrowth rolls; per-tree streams come
  from their own (seed, layer, si)-keyed rngs) into a `Vec<Draw>` log, and
  cache hits replay the log via `FrameRng::Replay`. Byte-identity is
  structural, and `cache_hit_matches_from_scratch` asserts hit == miss at
  four t values.
- Cache key covers every t-independent input: seed, w, h, the full palette,
  and all ForestKnobs fields. `param_f32("SPEED")` only scales t math in the
  dynamic pass, so it is read per frame as before.
- The caller-visible final rng state is unchanged: the miss path consumes the
  caller's rng exactly as the original code did, and the hit path restores the
  same end state by replay.

## Tried and rejected

- Snapshotting `StdRng` state for the cache: impossible in rand 0.10 (no
  Clone, no position API, inner Rng is private). Led to the replay log.
- LUTs for `lighten`/`hsl_to_rgb` in `plot_spiral` and the broadleaf dome:
  rounding drift risks a non-byte-identical cell for tiny measurable gain
  (those sites are per-glyph in the dynamic pass, which is now the minor cost).
- Pooling the per-tree scratch Vecs across frames via thread_local: invasive
  across 9 allocation sites in species.rs for sub-microsecond gain at current
  frame times.

## Remaining hotspots on the table

- Dynamic tree pass at 0.011 / 0.024 ms/frame: per-tree scratch Vec
  allocations (bottleneck 3) and `plot_spiral`/dome color math.
- `FrameRng::Replay` clones the draw log each hit; could store an
  `Rc<[Draw]>` if the log ever grows large (currently a few hundred entries).

## Validation transcript

```
$ CARGO_TARGET_DIR=$PWD/target-perf cargo build
    (build ok, pre-existing manifest-key warnings only)

$ CARGO_TARGET_DIR=$PWD/target-perf cargo test
test result: ok. 90 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 99 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s

$ CARGO_TARGET_DIR=$PWD/target-perf cargo test --release perf_arboretum -- --ignored --nocapture
80x24: 0.011 ms/frame over 300 frames
  static rebuild: 0.017 ms/frame
150x50: 0.024 ms/frame over 300 frames
  static rebuild: 0.058 ms/frame

$ git status --short   (no .snap files modified)
 M src/arboretum.rs
 M src/morph.rs
?? PERF.md
?? target-perf/
```
