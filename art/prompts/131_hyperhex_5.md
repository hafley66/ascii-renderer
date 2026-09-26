# 131 hyperhex-5 prompt sequence

Forked from 130 hyperhex-4 at a29bd5c, a byte-identical copy with the mode
renamed (struct, `NAME`, `HELP`, snapshots).

- Round five: swapped to the {6,6} tiling, a teal-to-rose depth story, an eclipse
  beat that starves the cell fills while the star field swells, and a collision
  flash where the two contagion fronts meet.

## Perf

Release bench: `CARGO_BUILD_JOBS=4 cargo test --release -- --ignored --nocapture modes::_131_hyperhex_5`
(the `bench` module in `_131_hyperhex_5_bench.rs`): 60 frames at t = 0..60,
min of 8 rounds, with per-layer attribution and an FNV checksum over the whole
sweep (not just the snapshot times).

| size | baseline ms/frame | final ms/frame |
|---|---|---|
| 200x60 | 0.443 | 0.318 |
| 400x120 | 1.419 | 1.055 |

Hottest layer, baseline -> final:
- 200x60: ground 0.181 -> ground 0.164 (fill 0.054)
- 400x120: ground 0.714 -> ground 0.655 (fill 0.184, edges 0.172)

Both checksums byte-identical: `e83e9cf57d6452ea` (200x60) /
`f313b03082f7440e` (400x120).

Commits:
- `36f8fca` per-thread tiling cache + scanline fill + hoist ground rows
