# 112 moire prompt sequence

User messages, verbatim, in order:

1. > how can i use it to go make art in a worktree taint and asll but asked not to look too hard at others
2. > try not to look at latest ones in psat 3 days, try to keep it efficient, have fun be avant garde but try not to be too influence buy the previous ones
3. > thats okay only ur stuff matters skip not ur stuff
4. > dont want u tainted

Brief: `briefs/boop-start.md` (this worktree's assigned brief).

## Perf

Release bench, 60 frames t = 0..60, min of 8 rounds
(`CARGO_BUILD_JOBS=4 cargo test --release -- --ignored --nocapture modes::_112_moire`):

| size | before ms/frame | after ms/frame |
|------|-----------------|----------------|
| 200x60 | 0.516 | 0.412 |
| 400x120 | 1.980 | 1.641 |

Hottest layer before: `field` 0.347 ms @200x60 (67% of the frame; allocations of
`pa`/`pb`/`beat` plus four extra full-screen passes).
After: a single fused `field` pass 0.411 ms.

Changes:
- Fused `field`/`ground`/`rings`/`beat`/`nodes` into one loop, dropping the three
  `Vec<Vec<f32>>` intermediates and the extra passes over them.
- The noise warp is bilinear over a small integer lattice, so the four per-cell
  `pp_hash2` calls became one `pp_hash2` per lattice point, filled once per frame.
- Hoisted the constant `lerp_color(pal[0], pal[1], 0.55)` ground foreground.

Output is byte-identical: the 60-frame FNV checksum over plain chars is unchanged
(`a7f514ab5ae75c47` @200x60, `985705727408213e` @400x120).
