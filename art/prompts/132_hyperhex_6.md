# 132 hyperhex-6 prompt sequence

Forked from 131 hyperhex-5 at 49a415a, a byte-identical copy with the mode
renamed (struct, `NAME`, `HELP`, snapshots).

- Round six: swapped to the {6,7} tiling, an amber-to-indigo depth story, a
  spiral shear during fracture (now a `TWIST` knob), and a reassembly rebirth
  wave that re-ignites cells centre-first.

## Perf

Release bench: `CARGO_BUILD_JOBS=4 cargo test --release -- --ignored --nocapture modes::_132_hyperhex_6`
(the `bench` module in `_132_hyperhex_6_bench.rs`): 60 frames at t = 0..60,
min of 8 rounds, per-layer attribution, and an FNV checksum over the whole sweep.

Session comparison: base `6313304` (separate worktree + target dir) and HEAD were
benched alternately in one session, same ROUNDS, both sizes; min of 3 runs.

| size | base 6313304 ms/frame | HEAD ms/frame | speedup |
|---|---|---|---|
| 200x60 | 1.147 | 0.795 | 1.44x |
| 400x120 | 3.399 | 2.281 | 1.49x |

Hottest layer, base -> HEAD:
- 200x60: fill 0.498 -> edges 0.296 (fill 0.254)
- 400x120: fill 1.812 -> fill 0.867 (ground 0.655, edges 0.641)

Checksums byte-identical in base and HEAD: `c3d27c99d2aafb19` (200x60) /
`82d35e9506652d8d` (400x120).

Commits:
- `7449924` per-thread tiling cache + scanline fill + hoist ground rows
