# 132 hyperhex-6 prompt sequence

Forked from 131 hyperhex-5 at 49a415a, a byte-identical copy with the mode
renamed (struct, `NAME`, `HELP`, snapshots).

- Round six: swapped to the {6,7} tiling, an amber-to-indigo depth story, a
  spiral shear during fracture (now a `TWIST` knob), and a reassembly rebirth
  wave that re-ignites cells centre-first.

## Perf

Release bench: `CARGO_BUILD_JOBS=4 cargo test --release -- --ignored --nocapture modes::_132_hyperhex_6`
(the `bench` module in `_132_hyperhex_6_bench.rs`): 60 frames at t = 0..60,
min of 8 rounds, with per-layer attribution and an FNV checksum over the whole
sweep (not just the snapshot times).

| size | baseline ms/frame | final ms/frame |
|---|---|---|
| 200x60 | 1.194 | 0.794 |
| 400x120 | 3.545 | 2.276 |

Hottest layer, baseline -> final:
- 200x60: fill 0.517 -> fill 0.254 (then edges 0.296)
- 400x120: fill 1.886 -> fill 0.866 (ground 0.653, edges 0.641)

Both checksums byte-identical: `c3d27c99d2aafb19` (200x60) /
`82d35e9506652d8d` (400x120).

Commits:
- `7449924` per-thread tiling cache + scanline fill + hoist ground rows
