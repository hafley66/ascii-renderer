# Opus quasicrystal performance repair

Measurements use the same release harness, seed progression, `moss` theme, 0.06 time step, and 2000x1000 grid before and after the change. Each knob sweep row ran for 2 seconds on 2026-09-04. Historical reports remain unchanged.

## Large-grid comparison

| mode and controls | version | avg ms | p50 ms | p99 ms | max ms |
| --- | --- | ---: | ---: | ---: | ---: |
| opus-1 default | d28663f | 10.72 | 10.40 | 14.58 | 19.12 |
| opus-1 default | repaired | 7.11 | 7.08 | 8.36 | 13.13 |
| opus-1 SYM=13 | d28663f | 12.12 | 11.85 | 15.73 | 35.43 |
| opus-1 SYM=13 | repaired | 8.30 | 8.25 | 9.72 | 16.28 |
| opus-2 default | d28663f | 28.63 | 28.65 | 29.31 | 29.80 |
| opus-2 default | repaired | 6.98 | 6.51 | 12.56 | 21.56 |
| opus-2 FOLDS=7 | d28663f | 33.50 | 33.33 | 35.21 | 37.01 |
| opus-2 FOLDS=7 | repaired | 7.69 | 7.33 | 13.36 | 21.35 |

Opus-1 row-binned shading reduced the default average by 33.7% and SYM=13 by 31.5%. Its SYM=13 shade layer changed from 6602.9 us average to 2349.6 us average. Opus-2 row-parallel lattice evaluation reduced the default average by 75.6% and FOLDS=7 by 77.0%. Its FOLDS=7 lattice layer changed from 31859.3 us average to 6443.3 us average.

The parallel tail remains above the median. Opus-2 FOLDS=7 measured 7.33 ms p50 and 21.35 ms maximum. Rayon scheduling and host contention remain visible in tail samples even though average native generation stays below the 16.67 ms budget at this synthetic size.

## Terminal-size and build-profile evidence

The existing deterministic `frame_cost` loop measured 200 frames at 200x60:

| mode | release avg | release max | debug avg | debug max |
| --- | ---: | ---: | ---: | ---: |
| opus-1 | 0.080 ms | 0.163 ms | 0.633 ms | 0.759 ms |
| opus-2 | 0.151 ms | 0.483 ms | 0.775 ms | 1.368 ms |

Opus-2 with `RAYON_NUM_THREADS=1` measured 0.254 ms average at 200x60, compared with 0.151 ms using the default pool. Opus-1 retains its serial shading path below 100,000 cells; its 200x60 result therefore has no row-bin or scheduling overhead.

The reported user command, `cargo run -- 42 demo`, uses the debug profile. Debug native generation is 7.9x slower for Opus-1 and 5.1x slower for Opus-2 in the 200x60 loop, while remaining below 1 ms average at that size.

A 240-frame sequence at 1000x500 changed five controls every frame, including the cache-key controls `SYM` and `FOLDS`, across their full declared ranges:

| mode | cold | avg | p50 | p99 | max |
| --- | ---: | ---: | ---: | ---: | ---: |
| opus-1 | 2.336 ms | 1.225 ms | 0.845 ms | 5.335 ms | 6.387 ms |
| opus-2 | 1.961 ms | 1.429 ms | 1.381 ms | 2.374 ms | 2.573 ms |

This probe forces cache rebuilds on each symmetry/fold transition while varying scale, shading/front geometry, dust, worms, and drift. It does not measure ANSI encoding or terminal presentation.

## Output preservation

Existing fixed-seed snapshots cover both modes at multiple seeds, times, and forced high symmetry/fold values. Added tests compare complete `Cell` grids, including glyph, foreground, and background colors, between one-thread and four-thread execution. Opus-1 also compares its row rasterizer directly with the original serial rasterizer across clipped parallelograms, dithered fills, solid fills, and ordered overwrites.

Three pre-edit/current binary comparisons produced byte-identical full ANSI output. The cases cover Opus-1 at 420x260, seed 73, time 19.75, SYM=13 and randomized controls; Opus-1 at 800x180, seed 991, time 87.4, SYM=7 and a second control combination; and Opus-2 at 430x250, seed 91, time 47.25, FOLDS=7 and randomized controls. Their paired SHA-256 prefixes are `611ac183`, `eb0f1024`, and `e2f256b5` respectively.

All controls, ranges, cache keys, seeded choices, floating-point evaluation within each row, and write order within each output row remain unchanged. Opus-1 only builds row bins at 100,000 cells or above.

## Playback boundary

Native frame generation does not include ANSI encoding, terminal I/O, or terminal painting. The demo loop increments simulation time by a fixed 0.06, then generates and encodes the frame, performs blocking `write_all` and `flush`, and finally waits up to 16 ms in `event::poll`. The wait is additive to the variable work rather than calculated from a frame deadline. Terminal throughput and that pacing path can still produce visible irregularity after the renderer changes.

## Commands

```bash
perf/knob_sweep.sh opus-1-quasicrystal 2000 1000 2 0.06 moss
perf/knob_sweep.sh opus-2-quasicrystal 2000 1000 2 0.06 moss
cargo test --release frame_cost -- --nocapture --test-threads=1
cargo test frame_cost -- --nocapture --test-threads=1
cargo test --release perf_random_knob_hops -- --ignored --nocapture --test-threads=1
```
