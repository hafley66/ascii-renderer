# 131 hyperhex-5 prompt sequence

Forked from 130 hyperhex-4 at a29bd5c, a byte-identical copy with the mode
renamed (struct, `NAME`, `HELP`, snapshots).

- Round five: swapped to the {6,6} tiling, a teal-to-rose depth story, an eclipse
  beat that starves the cell fills while the star field swells, and a collision
  flash where the two contagion fronts meet.

## Perf

Release bench: `CARGO_BUILD_JOBS=4 cargo test --release -- --ignored --nocapture modes::_131_hyperhex_5`
(the `bench` module in `_131_hyperhex_5_bench.rs`): 60 frames at t = 0..60,
min of 8 rounds, per-layer attribution, and an FNV checksum over the whole sweep.

Session comparison: base `6313304` (separate worktree + target dir) and HEAD were
benched alternately in one session, same ROUNDS, both sizes; min of 3 runs.

| size | base 6313304 ms/frame | HEAD ms/frame | speedup |
|---|---|---|---|
| 200x60 | 0.422 | 0.318 | 1.33x |
| 400x120 | 1.352 | 1.059 | 1.28x |

Hottest layer, base -> HEAD:
- 200x60: ground 0.171 -> ground 0.164
- 400x120: ground 0.685 -> ground 0.658 (fill 0.387 -> 0.185)

Checksums byte-identical in base and HEAD: `e83e9cf57d6452ea` (200x60) /
`f313b03082f7440e` (400x120).

Commits:
- `36f8fca` per-thread tiling cache + scanline fill + hoist ground rows
