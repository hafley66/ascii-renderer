# 130 hyperhex-4 prompt sequence

Forked from 115 hyperhex-3 at 74256a9, a byte-identical copy with the mode
renamed (struct, `NAME`, `HELP`, snapshots).

- Round four: swapped to the {6,5} tiling (circumradius from cos(pi/5)/sin(pi/6))
  and recoloured it gold-to-violet with a spring-green wound.

## Perf

Release bench: `CARGO_BUILD_JOBS=4 cargo test --release -- --ignored --nocapture modes::_130_hyperhex_4`
(the `bench` module in `_130_hyperhex_4_bench.rs`): 60 frames at t = 0..60,
min of 8 rounds, with per-layer attribution and an FNV checksum over the whole
sweep (not just the snapshot times).

| size | baseline ms/frame | final ms/frame |
|---|---|---|
| 200x60 | 1.503 | 0.990 |
| 400x120 | 4.430 | 2.772 |

Hottest layer, baseline -> final:
- 200x60: fill 0.684 -> fill 0.330 (then edges 0.400)
- 400x120: fill 2.529 -> fill 1.142 (edges 0.844, ground 0.659)

Both checksums byte-identical: `cf724811eca47655` (200x60) /
`1385ca6eb6be9598` (400x120).

Commits (oldest first):
- `5c77352` bench harness + scanline cell fill
- `97907d8` per-thread tiling cache + dedupe arc cos/sin
- `6d12d8c` hoist row-constant ground terms
