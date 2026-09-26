# 130 hyperhex-4 prompt sequence

Forked from 115 hyperhex-3 at 74256a9, a byte-identical copy with the mode
renamed (struct, `NAME`, `HELP`, snapshots).

- Round four: swapped to the {6,5} tiling (circumradius from cos(pi/5)/sin(pi/6))
  and recoloured it gold-to-violet with a spring-green wound.

## Perf

Release bench: `CARGO_BUILD_JOBS=4 cargo test --release -- --ignored --nocapture modes::_130_hyperhex_4`
(the `bench` module in `_130_hyperhex_4_bench.rs`): 60 frames at t = 0..60,
min of 8 rounds, per-layer attribution, and an FNV checksum over the whole sweep.

Session comparison: base `6313304` (separate worktree + target dir) and HEAD were
benched alternately in one session, same ROUNDS, both sizes; min of 3 runs.

| size | base 6313304 ms/frame | HEAD ms/frame | speedup |
|---|---|---|---|
| 200x60 | 1.443 | 0.981 | 1.47x |
| 400x120 | 4.250 | 2.783 | 1.53x |

Hottest layer, base -> HEAD:
- 200x60: fill 0.655 -> edges 0.396 (fill 0.328)
- 400x120: fill 2.430 -> fill 1.147 (edges 0.848, ground 0.660)

Checksums byte-identical in base and HEAD: `cf724811eca47655` (200x60) /
`1385ca6eb6be9598` (400x120).

Commits (oldest first):
- `5c77352` bench harness + scanline cell fill
- `97907d8` per-thread tiling cache + dedupe arc cos/sin
- `6d12d8c` hoist row-constant ground terms
